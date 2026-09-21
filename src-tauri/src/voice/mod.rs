//! Speech to text: the recording goes to the local Whisper service (`infra/compose.yaml`).
//!
//! The service speaks the well-known `whisper-asr-webservice` API, so a different host (another PC
//! in the network, a stronger machine) only needs a different URL in `settings.json`.

use std::time::Duration;

use rust_i18n::t;

use crate::config::VoiceSettings;

/// Transcribing runs on the CPU and may take a moment for a long dictation.
const TIMEOUT: Duration = Duration::from_secs(120);

/// Turns a WAV recording into text. An empty result means nothing was understood.
pub async fn transcribe(http: &reqwest::Client, settings: &VoiceSettings, wav: Vec<u8>) -> Result<String, String> {
    let base = settings.whisper_url.trim_end_matches('/');
    let part = reqwest::multipart::Part::bytes(wav)
        .file_name("speech.wav")
        .mime_str("audio/wav")
        .map_err(|e| t!("voice.failed", error = e).to_string())?;
    let form = reqwest::multipart::Form::new().part("audio_file", part);

    let mut query: Vec<(&str, &str)> = vec![("task", "transcribe"), ("output", "txt"), ("vad_filter", "true")];
    let language = settings.language.trim();
    // "auto" lets Whisper detect the language itself.
    if !language.is_empty() && !language.eq_ignore_ascii_case("auto") {
        query.push(("language", language));
    }

    let response = http
        .post(format!("{base}/asr"))
        .query(&query)
        .multipart(form)
        .timeout(TIMEOUT)
        .send()
        .await
        .map_err(|e| {
            if e.is_connect() || e.is_timeout() {
                t!("voice.unreachable", url = base).to_string()
            } else {
                t!("voice.failed", error = e.without_url()).to_string()
            }
        })?;

    if !response.status().is_success() {
        return Err(t!("voice.failed", error = response.status()).into());
    }
    let text = response
        .text()
        .await
        .map_err(|e| t!("voice.failed", error = e.without_url()).to_string())?;
    Ok(text.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// Answers one request with `Hallo Welt` and hands back what it received.
    async fn fake_whisper() -> (String, tokio::task::JoinHandle<String>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut buffer = [0u8; 4096];
            // Read until the multipart end marker; the body is small.
            loop {
                let read = stream.read(&mut buffer).await.unwrap();
                request.extend_from_slice(&buffer[..read]);
                if read == 0 || request.windows(4).any(|w| w == b"--\r\n") {
                    break;
                }
            }
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 12\r\nConnection: close\r\n\r\n Hallo Welt\n")
                .await
                .unwrap();
            let _ = stream.shutdown().await;
            String::from_utf8_lossy(&request).to_string()
        });
        (url, handle)
    }

    #[tokio::test]
    async fn sends_the_recording_as_audio_file_and_trims_the_answer() {
        let (url, server) = fake_whisper().await;
        let settings = VoiceSettings {
            whisper_url: url,
            language: "de".into(),
            send_directly: false,
        };
        let text = transcribe(&reqwest::Client::new(), &settings, b"RIFFfake".to_vec())
            .await
            .unwrap();
        assert_eq!(text, "Hallo Welt");

        let request = server.await.unwrap();
        assert!(request.starts_with("POST /asr?"), "{request}");
        assert!(request.contains("task=transcribe") && request.contains("output=txt"));
        assert!(request.contains("language=de"), "the configured language is sent");
        assert!(request.contains(r#"name="audio_file""#) && request.contains("filename=\"speech.wav\""));
        assert!(request.contains("RIFFfake"), "the recording itself");
    }

    #[tokio::test]
    async fn automatic_language_is_left_to_whisper() {
        let (url, server) = fake_whisper().await;
        let settings = VoiceSettings {
            whisper_url: url,
            language: "auto".into(),
            send_directly: false,
        };
        transcribe(&reqwest::Client::new(), &settings, b"RIFF".to_vec())
            .await
            .unwrap();
        assert!(!server.await.unwrap().contains("language="));
    }

    #[tokio::test]
    async fn a_service_that_is_not_running_says_so() {
        let settings = VoiceSettings {
            // Nothing listens here.
            whisper_url: "http://127.0.0.1:1".into(),
            ..VoiceSettings::default()
        };
        let error = transcribe(&reqwest::Client::new(), &settings, b"RIFF".to_vec())
            .await
            .unwrap_err();
        assert!(error.contains("127.0.0.1:1"), "{error}");
    }
}
