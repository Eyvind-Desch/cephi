//! LibreTranslate API client: `POST /translate`.

use std::time::Duration;

use rust_i18n::t;
use serde::{Deserialize, Serialize};

use super::Translation;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
const ALTERNATIVES: u8 = 3;

pub struct Client<'a> {
    http: &'a reqwest::Client,
    base: &'a str,
}

#[derive(Serialize)]
struct Request<'a> {
    q: &'a str,
    source: &'a str,
    target: &'a str,
    format: &'static str,
    alternatives: u8,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Response {
    translated_text: String,
    #[serde(default)]
    alternatives: Vec<String>,
    detected_language: Option<DetectedLanguage>,
}

#[derive(Deserialize)]
struct DetectedLanguage {
    language: String,
}

#[derive(Deserialize)]
struct ErrorResponse {
    error: String,
}

impl<'a> Client<'a> {
    pub fn new(http: &'a reqwest::Client, base: &'a str) -> Self {
        Self {
            http,
            base: base.trim_end_matches('/'),
        }
    }

    pub async fn translate(&self, text: &str, source: &str, target: &str) -> Result<Translation, String> {
        let request = Request {
            q: text,
            source,
            target,
            format: "text",
            alternatives: ALTERNATIVES,
        };
        let response = self
            .http
            .post(format!("{}/translate", self.base))
            .json(&request)
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() || e.is_timeout() {
                    t!("translate.unreachable", url = self.base).to_string()
                } else {
                    t!("translate.failed", error = e.without_url()).to_string()
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            // e.g. 400 "zh is not supported" when the language model is not loaded.
            let details = response
                .json::<ErrorResponse>()
                .await
                .map_or_else(|_| status.to_string(), |e| e.error);
            return Err(t!("translate.failed", error = details).into());
        }
        let body: Response = response
            .json()
            .await
            .map_err(|e| t!("translate.failed", error = e.without_url()).to_string())?;

        let from = match (source, body.detected_language) {
            ("auto", Some(detected)) => detected.language,
            (source, _) => source.to_string(),
        };
        Ok(Translation {
            text: body.translated_text.trim().to_string(),
            alternatives: body.alternatives.into_iter().map(|a| a.trim().to_string()).collect(),
            from,
            to: target.to_string(),
        })
    }
}
