//! Microphone recording for dictation (WASAPI, the default input device).
//!
//! Recording happens in Rust, not in the WebView: that avoids the browser permission prompt and
//! keeps the audio in the app. The result is a WAV in exactly the shape speech recognition wants:
//! 16 kHz, mono, 16 bit.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rust_i18n::t;
use windows::Win32::Media::Audio::{
    eCapture, eConsole, IAudioCaptureClient, IAudioClient, IMMDeviceEnumerator, MMDeviceEnumerator,
    AUDCLNT_SHAREMODE_SHARED, WAVEFORMATEX, WAVE_FORMAT_PCM,
};
use windows::Win32::Media::KernelStreaming::WAVE_FORMAT_EXTENSIBLE;
use windows::Win32::Media::Multimedia::WAVE_FORMAT_IEEE_FLOAT;
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};

/// What the speech service gets; Whisper works at 16 kHz anyway.
const TARGET_RATE: u32 = 16_000;
/// A dictated sentence, not a podcast: recording stops by itself after this.
const MAX_SECONDS: usize = 120;
/// How often the buffer is drained.
const POLL: Duration = Duration::from_millis(20);

#[derive(Default)]
pub struct Recorder {
    running: Arc<AtomicBool>,
    /// Samples collected so far (mono, 16 kHz), shared with the recording thread.
    samples: Arc<Mutex<Vec<f32>>>,
}

impl Recorder {
    /// Starts recording in a background thread. A running recording is kept (no-op).
    pub fn start(&self) -> Result<(), String> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        self.samples.lock().expect("recorder lock poisoned").clear();

        let running = self.running.clone();
        let samples = self.samples.clone();
        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<(), String>>();
        std::thread::spawn(move || {
            let result = capture(&running, &samples, &ready_tx);
            running.store(false, Ordering::SeqCst);
            if let Err(error) = result {
                log::warn!("recording: {error}");
                // Only reported here if the start itself already succeeded.
                let _ = ready_tx.send(Err(error));
            }
        });

        match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => {
                self.running.store(false, Ordering::SeqCst);
                Err(error)
            }
            Err(_) => {
                self.running.store(false, Ordering::SeqCst);
                Err(t!("voice.no_microphone", error = "timeout").into())
            }
        }
    }

    /// Stops the recording and returns it as a WAV file.
    pub fn stop(&self) -> Result<Vec<u8>, String> {
        self.running.store(false, Ordering::SeqCst);
        // Let the thread drain what is still in the buffer.
        std::thread::sleep(POLL * 2);
        let samples = self.samples.lock().expect("recorder lock poisoned").clone();
        if samples.len() < TARGET_RATE as usize / 4 {
            return Err(t!("voice.too_short").into());
        }
        Ok(wav(&samples, TARGET_RATE))
    }

    pub fn cancel(&self) {
        self.running.store(false, Ordering::SeqCst);
        self.samples.lock().expect("recorder lock poisoned").clear();
    }
}

/// Reads from the default input device until `running` turns false.
fn capture(
    running: &AtomicBool,
    samples: &Mutex<Vec<f32>>,
    ready: &std::sync::mpsc::Sender<Result<(), String>>,
) -> Result<(), String> {
    // SAFETY: COM interfaces created and released on this thread; buffers are copied before release.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let start = || -> Result<(IAudioClient, IAudioCaptureClient, Format), String> {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(device_error)?;
            let device = enumerator
                .GetDefaultAudioEndpoint(eCapture, eConsole)
                .map_err(device_error)?;
            let client: IAudioClient = device.Activate(CLSCTX_ALL, None).map_err(device_error)?;
            let mix = client.GetMixFormat().map_err(device_error)?;
            let format = Format::read(&*mix)?;
            client
                .Initialize(
                    AUDCLNT_SHAREMODE_SHARED,
                    0,
                    // 200 ms of buffer: plenty for a 20 ms poll.
                    2_000_000,
                    0,
                    mix,
                    None,
                )
                .map_err(device_error)?;
            let capture: IAudioCaptureClient = client.GetService().map_err(device_error)?;
            client.Start().map_err(device_error)?;
            Ok((client, capture, format))
        };

        let (client, capture, format) = match start() {
            Ok(parts) => {
                let _ = ready.send(Ok(()));
                parts
            }
            Err(error) => {
                let _ = ready.send(Err(error.clone()));
                return Err(error);
            }
        };

        let max_samples = MAX_SECONDS * TARGET_RATE as usize;
        // Position in the source stream, in target samples; keeps the resampling drift-free.
        let mut carry = 0.0f64;
        let step = f64::from(format.rate) / f64::from(TARGET_RATE);

        while running.load(Ordering::SeqCst) {
            let mut frames = capture.GetNextPacketSize().map_err(device_error)?;
            if frames == 0 {
                std::thread::sleep(POLL);
                continue;
            }
            while frames > 0 {
                let mut data = std::ptr::null_mut();
                let mut count = 0u32;
                let mut flags = 0u32;
                capture
                    .GetBuffer(&mut data, &mut count, &mut flags, None, None)
                    .map_err(device_error)?;
                if count > 0 {
                    let mono = format.to_mono(data, count);
                    let mut buffer = samples.lock().expect("recorder lock poisoned");
                    // Take every `step`-th sample: 48 kHz → 16 kHz without an extra library.
                    while (carry as usize) < mono.len() {
                        buffer.push(mono[carry as usize]);
                        carry += step;
                    }
                    carry -= mono.len() as f64;
                    if buffer.len() >= max_samples {
                        running.store(false, Ordering::SeqCst);
                    }
                }
                capture.ReleaseBuffer(count).map_err(device_error)?;
                frames = capture.GetNextPacketSize().map_err(device_error)?;
            }
            std::thread::sleep(POLL);
        }

        let _ = client.Stop();
        Ok(())
    }
}

/// The device's sample format, as far as the conversion needs it.
#[derive(Clone, Copy)]
struct Format {
    rate: u32,
    channels: usize,
    float: bool,
    bits: u16,
}

impl Format {
    unsafe fn read(format: &WAVEFORMATEX) -> Result<Self, String> {
        let float = match u32::from(format.wFormatTag) {
            WAVE_FORMAT_IEEE_FLOAT => true,
            WAVE_FORMAT_PCM => false,
            // Extensible: the sub-format decides; shared mode gives float in practice.
            tag if tag == WAVE_FORMAT_EXTENSIBLE => format.wBitsPerSample == 32,
            tag => return Err(t!("voice.no_microphone", error = format!("audio format {tag}")).to_string()),
        };
        Ok(Self {
            rate: format.nSamplesPerSec,
            channels: format.nChannels.max(1) as usize,
            float,
            bits: format.wBitsPerSample,
        })
    }

    /// Averages the channels into one mono signal in the range -1..1.
    unsafe fn to_mono(self, data: *const u8, frames: u32) -> Vec<f32> {
        let frames = frames as usize;
        let mut mono = Vec::with_capacity(frames);
        for frame in 0..frames {
            let mut sum = 0.0f32;
            for channel in 0..self.channels {
                let index = frame * self.channels + channel;
                sum += match (self.float, self.bits) {
                    (true, _) => *(data as *const f32).add(index),
                    (false, 16) => f32::from(*(data as *const i16).add(index)) / f32::from(i16::MAX),
                    (false, 32) => *(data as *const i32).add(index) as f32 / i32::MAX as f32,
                    _ => 0.0,
                };
            }
            mono.push(sum / self.channels as f32);
        }
        mono
    }
}

fn device_error(error: windows::core::Error) -> String {
    t!("voice.no_microphone", error = error).to_string()
}

/// A complete WAV file (PCM 16 bit, mono) from the samples.
fn wav(samples: &[f32], rate: u32) -> Vec<u8> {
    let data_len = samples.len() * 2;
    let mut out = Vec::with_capacity(44 + data_len);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&((36 + data_len) as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // chunk size
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 2).to_le_bytes()); // bytes per second
    out.extend_from_slice(&2u16.to_le_bytes()); // block align
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data_len as u32).to_le_bytes());
    for sample in samples {
        let value = (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16;
        out.extend_from_slice(&value.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_header_describes_the_samples() {
        let samples = vec![0.0f32, 1.0, -1.0, 0.5];
        let file = wav(&samples, TARGET_RATE);
        assert_eq!(&file[0..4], b"RIFF");
        assert_eq!(&file[8..12], b"WAVE");
        assert_eq!(file.len(), 44 + samples.len() * 2);
        assert_eq!(u32::from_le_bytes(file[24..28].try_into().unwrap()), TARGET_RATE);
        assert_eq!(u16::from_le_bytes(file[22..24].try_into().unwrap()), 1, "mono");
        assert_eq!(i16::from_le_bytes(file[44..46].try_into().unwrap()), 0);
        assert_eq!(i16::from_le_bytes(file[46..48].try_into().unwrap()), i16::MAX);
        assert_eq!(i16::from_le_bytes(file[48..50].try_into().unwrap()), -i16::MAX);
    }

    #[test]
    fn stereo_float_frames_become_mono() {
        let frames: [f32; 6] = [1.0, 0.0, 0.5, 0.5, -1.0, -1.0];
        let format = Format {
            rate: 48_000,
            channels: 2,
            float: true,
            bits: 32,
        };
        // SAFETY: the pointer refers to the array above, with exactly 3 stereo frames.
        let mono = unsafe { format.to_mono(frames.as_ptr() as *const u8, 3) };
        assert_eq!(mono, vec![0.5, 0.5, -1.0]);
    }

    /// Records two seconds from the real microphone. Run manually:
    /// `cargo test -- --ignored records_from_the_microphone`
    #[test]
    #[ignore = "records from the real microphone"]
    fn records_from_the_microphone() {
        let recorder = Recorder::default();
        recorder.start().unwrap();
        std::thread::sleep(Duration::from_secs(2));
        let file = recorder.stop().unwrap();
        let samples = (file.len() - 44) / 2;
        let seconds = samples as f32 / TARGET_RATE as f32;
        assert!((1.5..2.6).contains(&seconds), "{seconds} s recorded");
    }
}
