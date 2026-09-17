//! Spotify: find music via the Web API, play it through the desktop app.
//!
//! Only *search* uses the API (Client Credentials flow: no user login needed). Playback happens
//! by opening a `spotify:track:<id>` URI, which the desktop app starts playing immediately.

use std::time::{Duration, Instant};

use serde::Deserialize;
use serde_json::Value;
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;
use tokio::sync::Mutex;

use crate::secrets;

pub const CLIENT_ID_SECRET: &str = "spotify_client_id";
pub const CLIENT_SECRET_SECRET: &str = "spotify_client_secret";

const TOKEN_URL: &str = "https://accounts.spotify.com/api/token";
const SEARCH_URL: &str = "https://api.spotify.com/v1/search";
/// Renew tokens a bit before they expire.
const TOKEN_SAFETY_MARGIN: Duration = Duration::from_secs(60);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SearchKind {
    Track,
    Album,
    Artist,
    Playlist,
}

impl SearchKind {
    pub fn parse(value: Option<&str>) -> Result<Self, String> {
        match value.unwrap_or("track") {
            "track" => Ok(Self::Track),
            "album" => Ok(Self::Album),
            "artist" => Ok(Self::Artist),
            "playlist" => Ok(Self::Playlist),
            other => Err(format!("Unbekannter Spotify-Typ \"{other}\".")),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Track => "track",
            Self::Album => "album",
            Self::Artist => "artist",
            Self::Playlist => "playlist",
        }
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
}

#[derive(Default)]
pub struct SpotifyClient {
    token: Mutex<Option<(String, Instant)>>,
}

impl SpotifyClient {
    pub fn is_configured() -> bool {
        matches!(secrets::get(CLIENT_ID_SECRET), Ok(Some(_)))
            && matches!(secrets::get(CLIENT_SECRET_SECRET), Ok(Some(_)))
    }

    /// Forgets the cached token, e.g. after the credentials changed.
    pub async fn invalidate(&self) {
        *self.token.lock().await = None;
    }

    /// Fetches a token ahead of time so the first request is fast.
    pub async fn warm_up(&self, http: &reqwest::Client) {
        if Self::is_configured() {
            match self.access_token(http).await {
                Ok(_) => log::info!("spotify: token ready"),
                Err(e) => log::warn!("spotify warm-up: {e}"),
            }
        }
    }

    pub async fn play(
        &self,
        app: &AppHandle,
        http: &reqwest::Client,
        query: &str,
        kind: SearchKind,
    ) -> Result<String, String> {
        let query = query.trim();
        if query.is_empty() {
            return Err("Wonach soll ich in Spotify suchen?".into());
        }

        let item = self.search_first(http, query, kind).await?;
        let uri = item["uri"].as_str().ok_or("Spotify-Antwort ohne URI.")?;
        app.opener()
            .open_url(uri, None::<&str>)
            .map_err(|e| format!("Spotify konnte nicht geöffnet werden: {e}"))?;

        let name = item["name"].as_str().unwrap_or(query);
        let by = item["artists"]
            .as_array()
            .map(|artists| {
                artists
                    .iter()
                    .filter_map(|a| a["name"].as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .filter(|s| !s.is_empty())
            .or_else(|| item["owner"]["display_name"].as_str().map(str::to_string));

        let title = match by {
            Some(by) => format!("{name} – {by}"),
            None => name.to_string(),
        };
        Ok(match kind {
            SearchKind::Track => format!("Spielt in Spotify: {title}"),
            _ => format!("In Spotify geöffnet ({}): {title}", kind.as_str()),
        })
    }

    async fn search_first(&self, http: &reqwest::Client, query: &str, kind: SearchKind) -> Result<Value, String> {
        let mut retried = false;
        loop {
            let token = self.access_token(http).await?;
            let response = http
                .get(SEARCH_URL)
                .bearer_auth(&token)
                .query(&[("q", query), ("type", kind.as_str()), ("limit", "5")])
                .timeout(REQUEST_TIMEOUT)
                .send()
                .await
                .map_err(|e| format!("Spotify nicht erreichbar: {e}"))?;

            if response.status() == reqwest::StatusCode::UNAUTHORIZED && !retried {
                *self.token.lock().await = None;
                retried = true;
                continue;
            }
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(format!("Spotify-Suche fehlgeschlagen (HTTP {status}): {body}"));
            }

            let body: Value = response
                .json()
                .await
                .map_err(|e| format!("Spotify-Antwort unlesbar: {e}"))?;
            // Playlist results may contain `null` entries.
            return body[format!("{}s", kind.as_str())]["items"]
                .as_array()
                .and_then(|items| items.iter().find(|item| item["uri"].is_string()).cloned())
                .ok_or_else(|| format!("Nichts gefunden in Spotify für \"{query}\"."));
        }
    }

    async fn access_token(&self, http: &reqwest::Client) -> Result<String, String> {
        let mut cached = self.token.lock().await;
        if let Some((token, expires)) = cached.as_ref() {
            if Instant::now() < *expires {
                return Ok(token.clone());
            }
        }

        let (Some(client_id), Some(client_secret)) =
            (secrets::get(CLIENT_ID_SECRET)?, secrets::get(CLIENT_SECRET_SECRET)?)
        else {
            return Err(
                "Spotify ist noch nicht eingerichtet: Client-ID und Client-Secret in Orbit unter \
                „Schlüssel“ hinterlegen (Anleitung: docs/SPOTIFY.md)."
                    .into(),
            );
        };

        let response = http
            .post(TOKEN_URL)
            .basic_auth(client_id, Some(client_secret))
            .form(&[("grant_type", "client_credentials")])
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await
            .map_err(|e| format!("Spotify nicht erreichbar: {e}"))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(
                if status == reqwest::StatusCode::BAD_REQUEST || status == reqwest::StatusCode::UNAUTHORIZED {
                    "Spotify lehnt die Zugangsdaten ab. Client-ID und Client-Secret prüfen.".into()
                } else {
                    format!("Spotify-Anmeldung fehlgeschlagen (HTTP {status}).")
                },
            );
        }

        let token: TokenResponse = response
            .json()
            .await
            .map_err(|e| format!("Spotify-Token unlesbar: {e}"))?;
        let expires = Instant::now() + Duration::from_secs(token.expires_in).saturating_sub(TOKEN_SAFETY_MARGIN);
        *cached = Some((token.access_token.clone(), expires));
        Ok(token.access_token)
    }
}
