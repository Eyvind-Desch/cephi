//! Index of installed apps (Start menu + Store apps) with fuzzy search and launching.
//!
//! The index comes from `Get-StartApps`, which takes a few seconds, so it is cached on disk
//! and refreshed in the background. Apps are launched through `shell:AppsFolder\<AppID>`,
//! which works for classic and Store apps alike.

mod fuzzy;

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// Minimum score for the fast path ("öffne X" without the model).
pub const FAST_PATH_MIN_SCORE: u32 = 75;
/// Minimum score when the model explicitly asks to open an app.
pub const TOOL_MIN_SCORE: u32 = 45;

const REFRESH_COOLDOWN: Duration = Duration::from_secs(60);
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AppEntry {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "AppID")]
    pub app_id: String,
}

pub struct AppIndex {
    apps: RwLock<Arc<Vec<AppEntry>>>,
    cache_path: PathBuf,
    last_refresh: Mutex<Option<Instant>>,
}

impl AppIndex {
    /// Creates the index from the on-disk cache (instant); call [`AppIndex::refresh`] afterwards.
    pub fn load(cache_path: PathBuf) -> Self {
        let apps = std::fs::read(&cache_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Vec<AppEntry>>(&bytes).ok())
            .unwrap_or_default();
        log::info!("launcher: {} app(s) from cache", apps.len());
        Self {
            apps: RwLock::new(Arc::new(apps)),
            cache_path,
            last_refresh: Mutex::new(None),
        }
    }

    /// Re-reads installed apps. Skipped if the last refresh was less than a minute ago.
    pub async fn refresh(&self) -> Result<usize, String> {
        {
            let mut last = self.last_refresh.lock().expect("refresh lock poisoned");
            if last.is_some_and(|t| t.elapsed() < REFRESH_COOLDOWN) {
                return Ok(self.snapshot().len());
            }
            *last = Some(Instant::now());
        }

        let started = Instant::now();
        let output = tokio::process::Command::new("powershell.exe")
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "[Console]::OutputEncoding = [Text.Encoding]::UTF8; @(Get-StartApps) | ConvertTo-Json -Compress",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .kill_on_drop(true)
            .output()
            .await
            .map_err(|e| format!("App-Liste konnte nicht gelesen werden: {e}"))?;

        let mut apps: Vec<AppEntry> = serde_json::from_slice(&output.stdout)
            .map_err(|e| format!("App-Liste hat ein unerwartetes Format: {e}"))?;
        apps.retain(|app| !app.name.trim().is_empty() && !app.app_id.trim().is_empty());
        apps.sort_by_key(|app| app.name.to_lowercase());

        if let Some(parent) = self.cache_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_vec(&apps) {
            let _ = std::fs::write(&self.cache_path, json);
        }

        let count = apps.len();
        *self.apps.write().expect("apps lock poisoned") = Arc::new(apps);
        log::info!(
            "launcher: indexed {count} app(s) in {} ms",
            started.elapsed().as_millis()
        );
        Ok(count)
    }

    /// Best matches for `query`, highest score first.
    pub fn search(&self, query: &str, limit: usize) -> Vec<(u32, AppEntry)> {
        let apps = self.snapshot();
        let mut matches: Vec<(u32, AppEntry)> = apps
            .iter()
            .map(|app| (fuzzy::score(query, &app.name), app))
            .filter(|(score, _)| *score > 0)
            .map(|(score, app)| (score, app.clone()))
            .collect();
        // Higher score first; on ties the shorter (more specific) name wins.
        matches.sort_by(|(sa, a), (sb, b)| sb.cmp(sa).then(a.name.len().cmp(&b.name.len())));
        matches.truncate(limit);
        matches
    }

    pub fn best_match(&self, query: &str, min_score: u32) -> Option<AppEntry> {
        self.search(query, 1)
            .into_iter()
            .next()
            .filter(|(score, _)| *score >= min_score)
            .map(|(_, app)| app)
    }

    pub fn launch(&self, app: &AppEntry) -> Result<(), String> {
        std::process::Command::new("explorer.exe")
            .arg(format!("shell:AppsFolder\\{}", app.app_id))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("\"{}\" konnte nicht gestartet werden: {e}", app.name))
    }

    fn snapshot(&self) -> Arc<Vec<AppEntry>> {
        self.apps.read().expect("apps lock poisoned").clone()
    }
}
