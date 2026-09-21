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
