//! Microphone recording for dictation (WASAPI, the default input device).
//!
//! Recording happens in Rust, not in the WebView: that avoids the browser permission prompt and
//! keeps the audio in the app. The result is a WAV in exactly the shape speech recognition wants:
//! 16 kHz, mono, 16 bit.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use rust_i18n::t;
use windows::Win32::Media::Audio::{
    eCapture, eConsole, IAudioCaptureClient, IAudioClient, IMMDeviceEnumerator, MMDeviceEnumerator,
    AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED, WAVEFORMATEX, WAVEFORMATEXTENSIBLE, WAVE_FORMAT_PCM,
};
use windows::Win32::Media::KernelStreaming::WAVE_FORMAT_EXTENSIBLE;
use windows::Win32::Media::Multimedia::{KSDATAFORMAT_SUBTYPE_IEEE_FLOAT, WAVE_FORMAT_IEEE_FLOAT};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_MULTITHREADED};

/// What the speech service gets; Whisper works at 16 kHz anyway.
const TARGET_RATE: u32 = 16_000;
/// A dictated sentence, not a podcast: recording stops by itself after this.
const MAX_SECONDS: usize = 120;
/// How often the buffer is drained.
const POLL: Duration = Duration::from_millis(20);
/// Sub-format of plain PCM in an extensible format description (not exported by the bindings).
const SUBTYPE_PCM: windows::core::GUID = windows::core::GUID::from_u128(0x00000001_0000_0010_8000_00aa00389b71);
/// Sample rates outside this range mean a broken device description.
const RATE_RANGE: std::ops::RangeInclusive<u32> = 4_000..=768_000;

#[derive(Default)]
pub struct Recorder {
    /// Increases with every start; a thread whose number is outdated stops by itself.
    generation: Arc<AtomicU64>,
    running: Arc<AtomicBool>,
    /// Samples collected so far (mono, 16 kHz), shared with the recording thread.
    samples: Arc<Mutex<Vec<f32>>>,
    /// A problem that hit the recording after it started (e.g. the microphone was unplugged).
    failure: Arc<Mutex<Option<String>>>,
}

/// A poisoned lock only means a thread panicked while holding it; the data itself stays usable.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

impl Recorder {
    /// Starts recording in a background thread. A running recording is replaced by a fresh one.
    pub fn start(&self) -> Result<(), String> {
        // A new number lets any older thread notice that it is done, even mid-poll.
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        self.running.store(true, Ordering::SeqCst);
        lock(&self.samples).clear();
        *lock(&self.failure) = None;

        let running = self.running.clone();
        let current = self.generation.clone();
        let samples = self.samples.clone();
        let failure = self.failure.clone();
        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<(), String>>();
        std::thread::spawn(move || {
            let context = Capture {
                generation,
                current: &current,
                running: &running,
                samples: &samples,
            };
            if let Err(error) = capture(&context, &ready_tx) {
                log::warn!("recording: {error}");
                // The start already reported success: remember it for `stop`.
                *lock(&failure) = Some(error);
            }
            // Only the newest thread owns the state.
            if current.load(Ordering::SeqCst) == generation {
                running.store(false, Ordering::SeqCst);
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

    /// Asks the thread to stop; [`take`](Self::take) collects the recording afterwards.
    pub fn request_stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// The recording as a WAV file, or why there is none.
    pub fn take(&self) -> Result<Vec<u8>, String> {
        if let Some(error) = lock(&self.failure).take() {
            return Err(error);
        }
        let samples = std::mem::take(&mut *lock(&self.samples));
        if samples.len() < TARGET_RATE as usize / 4 {
            return Err(t!("voice.too_short").into());
        }
        Ok(wav(&samples, TARGET_RATE))
    }

    pub fn cancel(&self) {
        self.request_stop();
        lock(&self.samples).clear();
        *lock(&self.failure) = None;
    }
}

/// What the recording thread works with.
struct Capture<'a> {
    generation: u64,
    current: &'a AtomicU64,
    running: &'a AtomicBool,
    samples: &'a Mutex<Vec<f32>>,
}

impl Capture<'_> {
    /// False as soon as the user stopped or a newer recording took over.
    fn wanted(&self) -> bool {
        self.running.load(Ordering::SeqCst) && self.current.load(Ordering::SeqCst) == self.generation
    }
}

/// Reads from the default input device until the recording is no longer wanted.
fn capture(context: &Capture<'_>, ready: &std::sync::mpsc::Sender<Result<(), String>>) -> Result<(), String> {
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
            let format = Format::read(mix);
            let initialized = client.Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                0,
                // 200 ms of buffer: plenty for a 20 ms poll.
                2_000_000,
                0,
                mix,
                None,
            );
            // The mix format belongs to the caller.
            CoTaskMemFree(Some(mix.cast()));
            initialized.map_err(device_error)?;
            let format = format?;
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
        // Whatever happens below, the device is released again.
        let result = read_packets(context, &capture, format);
        let _ = client.Stop();
        result
    }
}

unsafe fn read_packets(context: &Capture<'_>, capture: &IAudioCaptureClient, format: Format) -> Result<(), String> {
    let max_samples = MAX_SECONDS * TARGET_RATE as usize;
    // Position in the source stream, in target samples; keeps the resampling drift-free.
    let mut carry = 0.0f64;
    let step = f64::from(format.rate) / f64::from(TARGET_RATE);

    while context.wanted() {
        let mut frames = capture.GetNextPacketSize().map_err(device_error)?;
        if frames == 0 {
            std::thread::sleep(POLL);
            continue;
        }
        while frames > 0 && context.wanted() {
            let mut data = std::ptr::null_mut();
            let mut count = 0u32;
            let mut flags = 0u32;
            capture
                .GetBuffer(&mut data, &mut count, &mut flags, None, None)
                .map_err(device_error)?;
            // A silent packet has undefined contents; only its length counts.
            let silent = flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0;
            if count > 0 {
                let mono = format.to_mono(data, count, silent);
                let mut buffer = lock(context.samples);
                // Take every `step`-th sample: 48 kHz → 16 kHz. Plain decimation, no low-pass:
                // speech has little content above 8 kHz and Whisper copes with the rest.
                while (carry as usize) < mono.len() {
                    buffer.push(mono[carry as usize]);
                    carry += step;
                }
                carry -= mono.len() as f64;
                if buffer.len() >= max_samples {
                    drop(buffer);
                    context.running.store(false, Ordering::SeqCst);
                    return Ok(());
                }
            }
            capture.ReleaseBuffer(count).map_err(device_error)?;
            frames = capture.GetNextPacketSize().map_err(device_error)?;
        }
        std::thread::sleep(POLL);
    }
    Ok(())
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
    /// Reads the mix format; unusual descriptions are refused instead of decoded wrongly.
    unsafe fn read(format: *const WAVEFORMATEX) -> Result<Self, String> {
        // The struct is packed: copy the fields instead of referencing them.
        let tag = u32::from((*format).wFormatTag);
        let bits = (*format).wBitsPerSample;
        let rate = (*format).nSamplesPerSec;
        let channels = (*format).nChannels;
        let extra = (*format).cbSize;

        let float = if tag == WAVE_FORMAT_IEEE_FLOAT {
            true
        } else if tag == WAVE_FORMAT_PCM {
            false
        } else if tag == WAVE_FORMAT_EXTENSIBLE && extra >= 22 {
            // The sub-format decides, not the bit depth.
            let sub_format = (*(format as *const WAVEFORMATEXTENSIBLE)).SubFormat;
            if sub_format == KSDATAFORMAT_SUBTYPE_IEEE_FLOAT {
                true
            } else if sub_format == SUBTYPE_PCM {
                false
            } else {
                return Err(unsupported(format!("sub-format {sub_format:?}")));
            }
        } else {
            return Err(unsupported(format!("audio format {tag}")));
        };

        if !(float && bits == 32 || !float && (bits == 16 || bits == 32)) {
            return Err(unsupported(format!("{bits} bit")));
        }
        if !RATE_RANGE.contains(&rate) {
            return Err(unsupported(format!("{rate} Hz")));
        }
        Ok(Self {
            rate,
            channels: usize::from(channels.max(1)),
            float,
            bits,
        })
    }

    /// Averages the channels into one mono signal in the range -1..1.
    unsafe fn to_mono(self, data: *const u8, frames: u32, silent: bool) -> Vec<f32> {
        let frames = frames as usize;
        if silent {
            return vec![0.0; frames];
        }
        let mut mono = Vec::with_capacity(frames);
        for frame in 0..frames {
            let mut sum = 0.0f32;
            for channel in 0..self.channels {
                let index = frame * self.channels + channel;
                sum += match (self.float, self.bits) {
                    (true, _) => *(data as *const f32).add(index),
                    (false, 16) => f32::from(*(data as *const i16).add(index)) / f32::from(i16::MAX),
                    // `read` rejects every other combination.
                    (false, _) => *(data as *const i32).add(index) as f32 / i32::MAX as f32,
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

fn unsupported(what: String) -> String {
    t!("voice.no_microphone", error = what).to_string()
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

    fn wave_format(tag: u16, bits: u16, rate: u32, channels: u16) -> WAVEFORMATEX {
        WAVEFORMATEX {
            wFormatTag: tag,
            nChannels: channels,
            nSamplesPerSec: rate,
            wBitsPerSample: bits,
            cbSize: 0,
            ..Default::default()
        }
    }

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
        let mono = unsafe { format.to_mono(frames.as_ptr() as *const u8, 3, false) };
        assert_eq!(mono, vec![0.5, 0.5, -1.0]);
        // SAFETY: a silent packet is not read at all.
        assert_eq!(unsafe { format.to_mono(std::ptr::null(), 2, true) }, vec![0.0, 0.0]);
    }

    #[test]
    fn broken_device_descriptions_are_refused() {
        let float = wave_format(WAVE_FORMAT_IEEE_FLOAT as u16, 32, 48_000, 2);
        // SAFETY: all of these point to a complete WAVEFORMATEX on the stack.
        unsafe {
            assert!(Format::read(&float).is_ok());
            // A rate of zero would make the resampling loop spin forever.
            assert!(Format::read(&wave_format(WAVE_FORMAT_IEEE_FLOAT as u16, 32, 0, 2)).is_err());
            // 24 bit would silently decode as zeros.
            assert!(Format::read(&wave_format(WAVE_FORMAT_PCM as u16, 24, 48_000, 2)).is_err());
            assert!(
                Format::read(&wave_format(0xfffe, 32, 48_000, 2)).is_err(),
                "extensible without a sub-format"
            );
        }
    }

    /// Records two seconds from the real microphone. Run manually:
    /// `cargo test -- --ignored records_from_the_microphone`
    #[test]
    #[ignore = "records from the real microphone"]
    fn records_from_the_microphone() {
        let recorder = Recorder::default();
        recorder.start().unwrap();
        std::thread::sleep(Duration::from_secs(2));
        recorder.request_stop();
        std::thread::sleep(POLL * 3);
        let file = recorder.take().unwrap();
        let samples = (file.len() - 44) / 2;
        let seconds = samples as f32 / TARGET_RATE as f32;
        assert!((1.5..2.6).contains(&seconds), "{seconds} s recorded");

        // Starting again right away must not leave the old recording running.
        recorder.start().unwrap();
        recorder.start().unwrap();
        std::thread::sleep(Duration::from_millis(500));
        recorder.request_stop();
        std::thread::sleep(POLL * 3);
        let again = recorder.take().unwrap();
        let seconds = ((again.len() - 44) / 2) as f32 / TARGET_RATE as f32;
        assert!((0.3..0.9).contains(&seconds), "{seconds} s recorded by one thread");
    }
}
