//! Cephi's local helper services: SearXNG (web search) and LibreTranslate (translation).
//!
//! They run as Docker containers inside WSL. Cephi ships their configuration ([`COMPOSE`]), writes it
//! to `~/.cephi/infra` in the default distro and starts them on demand, so nobody has to run a script.

use std::process::Stdio;
use std::time::Duration;

use rust_i18n::t;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

use crate::config::Settings;

/// Shipped container configuration (same files as in `infra/`).
const COMPOSE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../infra/compose.yaml"));
const SEARXNG_SETTINGS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../infra/searxng/settings.yml"));
/// Folder inside WSL that holds the configuration.
const TARGET_DIR: &str = "~/.cephi/infra";
/// Containers of the app's old name "Orbit"; removed before starting the new ones.
const LEGACY_CONTAINERS: [&str; 2] = ["orbit-searxng", "orbit-libretranslate"];

const PROBE_TIMEOUT: Duration = Duration::from_secs(2);
/// Starting may include pulling images and downloading translation models.
const START_TIMEOUT: Duration = Duration::from_secs(600);
const READY_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Service {
    Search,
    Translate,
    Voice,
}

/// Mirrored in `src/lib/ipc.ts` (`ServiceStatus`).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceStatus {
    pub id: Service,
    pub title: String,
    pub running: bool,
}

impl Service {
    pub const ALL: [Service; 3] = [Service::Search, Service::Translate, Service::Voice];

    /// Compose service name in `infra/compose.yaml`.
    fn container(self) -> &'static str {
        match self {
            Service::Search => "searxng",
            Service::Translate => "libretranslate",
            Service::Voice => "whisper",
        }
    }

    pub fn title(self) -> String {
        match self {
            Service::Search => t!("services.search").into(),
            Service::Translate => t!("services.translate").into(),
            Service::Voice => t!("services.voice").into(),
        }
    }

    /// URL that answers as soon as the service is usable.
    fn health_url(self, settings: &Settings) -> String {
        match self {
            Service::Search => format!("{}/healthz", settings.search.searxng_url.trim_end_matches('/')),
            Service::Translate => format!(
                "{}/languages",
                settings.translate.libretranslate_url.trim_end_matches('/')
            ),
            // The API docs answer as soon as the server is up (the model loads on the first request).
            Service::Voice => format!("{}/docs", settings.voice.whisper_url.trim_end_matches('/')),
        }
    }

    pub async fn is_running(self, http: &reqwest::Client, settings: &Settings) -> bool {
        http.get(self.health_url(settings))
            .timeout(PROBE_TIMEOUT)
            .send()
            .await
            .is_ok_and(|response| response.status().is_success())
    }
}

pub async fn status(http: &reqwest::Client, settings: &Settings) -> Vec<ServiceStatus> {
    let (search, translate, voice) = tokio::join!(
        Service::Search.is_running(http, settings),
        Service::Translate.is_running(http, settings),
        Service::Voice.is_running(http, settings)
    );
    Service::ALL
        .into_iter()
        .zip([search, translate, voice])
        .map(|(id, running)| ServiceStatus {
            id,
            title: id.title(),
            running,
        })
        .collect()
}

/// Which of the services a failing tool needs are currently down.
pub async fn stopped(http: &reqwest::Client, settings: &Settings, services: &[Service]) -> Vec<Service> {
    let mut stopped = Vec::new();
    for service in services {
        if !service.is_running(http, settings).await {
            stopped.push(*service);
        }
    }
    stopped
}

/// Writes the configuration into WSL, starts the container and waits until it answers.
pub async fn start(http: &reqwest::Client, settings: &Settings, service: Service) -> Result<(), String> {
    write_config().await?;

    // Containers from before the rename hold the same ports.
    let legacy = LEGACY_CONTAINERS.join(" ");
    wsl(&format!("docker rm -f {legacy} >/dev/null 2>&1 || true"), None).await?;

    let up = format!("cd {TARGET_DIR} && docker compose up -d {}", service.container());
    let output = tokio::time::timeout(START_TIMEOUT, wsl(&up, None))
        .await
        .map_err(|_| t!("services.timeout", service = service.title()).to_string())??;
    log::info!("service {}: {}", service.container(), output.trim());

    let deadline = tokio::time::Instant::now() + READY_TIMEOUT;
    while tokio::time::Instant::now() < deadline {
        if service.is_running(http, settings).await {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    Err(t!("services.timeout", service = service.title()).into())
}

async fn write_config() -> Result<(), String> {
    wsl(&format!("mkdir -p {TARGET_DIR}/searxng"), None).await?;
    wsl(&format!("cat > {TARGET_DIR}/compose.yaml"), Some(COMPOSE)).await?;

    // Keep an existing SearXNG config (it holds a generated secret).
    let exists = wsl(
        &format!("test -f {TARGET_DIR}/searxng/settings.yml && echo yes || true"),
        None,
    )
    .await?;
    if !exists.trim().eq_ignore_ascii_case("yes") {
        let settings = SEARXNG_SETTINGS.replace("__SECRET__", &random_secret());
        wsl(&format!("cat > {TARGET_DIR}/searxng/settings.yml"), Some(&settings)).await?;
    }
    Ok(())
}

/// Runs a bash command in the default WSL distro; `stdin` is piped into it.
async fn wsl(command: &str, stdin: Option<&str>) -> Result<String, String> {
    let mut child = tokio::process::Command::new("wsl.exe")
        .args(["--", "bash", "-c", command])
        .stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .spawn()
        .map_err(|e| t!("services.wsl_missing", error = e).to_string())?;

    if let Some(stdin) = stdin {
        let mut pipe = child
            .stdin
            .take()
            .ok_or_else(|| t!("services.wsl_missing", error = "stdin").to_string())?;
        pipe.write_all(stdin.as_bytes())
            .await
            .map_err(|e| t!("services.start_failed", error = e).to_string())?;
        // Closing the pipe signals end of input.
        drop(pipe);
    }

    let output = child
        .wait_with_output()
        .await
        .map_err(|e| t!("services.start_failed", error = e).to_string())?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let details = String::from_utf8_lossy(&output.stderr);
        let details = details.trim();
        Err(t!(
            "services.start_failed",
            error = if details.is_empty() { "exit code" } else { details }
        )
        .into())
    }
}

fn random_secret() -> String {
    uuid::Uuid::new_v4().simple().to_string() + &uuid::Uuid::new_v4().simple().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_config_matches_the_repository() {
        assert!(COMPOSE.contains("cephi-searxng") && COMPOSE.contains("cephi-libretranslate"));
        assert!(
            SEARXNG_SETTINGS.contains("__SECRET__"),
            "the secret placeholder is filled in on start"
        );
    }

    /// Needs Docker in WSL. Starts the translator (fine if it is already running).
    #[tokio::test]
    #[ignore]
    async fn live_start_brings_up_the_translator() {
        let http = reqwest::Client::new();
        let settings = Settings::default();
        start(&http, &settings, Service::Translate).await.unwrap();
        assert!(Service::Translate.is_running(&http, &settings).await);
    }

    #[test]
    fn secrets_are_long_enough_for_searxng() {
        assert_eq!(random_secret().len(), 64);
        assert_ne!(random_secret(), random_secret());
    }
}
