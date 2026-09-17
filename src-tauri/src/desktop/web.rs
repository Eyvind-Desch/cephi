//! The web tab: a child webview inside the Orbit window that shows Google and other pages.
//!
//! The frontend owns the layout. It reports where the page area is (`set_bounds`) and whether the
//! tab is visible; the webview is created lazily on the first navigation. Remote pages get no IPC
//! access: the capability only covers the `main` webview.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::webview::{NewWindowResponse, PageLoadEvent};
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Rect, Theme, Webview, WebviewBuilder, WebviewUrl,
};
use tauri_plugin_opener::OpenerExt;
use url::Url;

use super::window::{self, MAIN_LABEL};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub const WEB_LABEL: &str = "web";
/// `WebState` changed (URL, title, loading).
pub const EVENT_WEB_STATE: &str = "orbit://web-state";
/// A tool opened a page: the frontend switches to the web tab.
pub const EVENT_WEB_OPEN: &str = "orbit://web-open";

const SEARCH_URL: &str = "https://www.google.com/search";

/// Mirrored in `src/lib/ipc.ts` (`WebState`).
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebState {
    pub url: Option<String>,
    pub title: String,
    pub loading: bool,
}

/// Page area in logical pixels, relative to the window.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Default)]
pub struct WebController {
    state: Mutex<WebState>,
    layout: Mutex<Layout>,
    /// Serializes webview creation.
    create: tokio::sync::Mutex<()>,
}

#[derive(Clone, Copy, Default)]
struct Layout {
    bounds: Bounds,
    visible: bool,
}

impl WebController {
    pub fn state(&self) -> WebState {
        self.state.lock().expect("web state lock poisoned").clone()
    }

    fn update(&self, app: &AppHandle, change: impl FnOnce(&mut WebState)) {
        let snapshot = {
            let mut state = self.state.lock().expect("web state lock poisoned");
            change(&mut state);
            state.clone()
        };
        if let Err(e) = app.emit_to(MAIN_LABEL, EVENT_WEB_STATE, snapshot) {
            log::warn!("emit web state: {e}");
        }
    }

    fn layout(&self) -> Layout {
        *self.layout.lock().expect("web layout lock poisoned")
    }
}

/// Opens a page in the web tab and switches to it (used by tools with `"target": "orbit"`).
pub async fn open(app: &AppHandle, url: Url) -> AppResult<()> {
    navigate(app, url).await?;
    app.emit_to(MAIN_LABEL, EVENT_WEB_OPEN, ())?;
    Ok(())
}

/// Address bar input: a URL, a domain or search terms.
pub async fn navigate_input(app: &AppHandle, input: &str) -> AppResult<()> {
    match resolve_input(input) {
        Some(url) => navigate(app, url).await,
        None => Ok(()),
    }
}

async fn navigate(app: &AppHandle, url: Url) -> AppResult<()> {
    if !is_web_url(&url) {
        return Err(AppError::msg(rust_i18n::t!("external.web_only")));
    }
    match app.get_webview(WEB_LABEL) {
        Some(webview) => webview.navigate(url)?,
        None => create(app, url).await?,
    }
    Ok(())
}

async fn create(app: &AppHandle, url: Url) -> AppResult<()> {
    let state = app.state::<AppState>();
    let _guard = state.web.create.lock().await;
    if let Some(webview) = app.get_webview(WEB_LABEL) {
        webview.navigate(url)?;
        return Ok(());
    }

    let window = app
        .get_window(MAIN_LABEL)
        .ok_or_else(|| AppError::msg("main window not found"))?;
    // Avoid a white flash before the first page paints.
    let background = match window.theme() {
        Ok(Theme::Light) => tauri::window::Color(250, 250, 252, 255),
        _ => tauri::window::Color(24, 24, 28, 255),
    };

    let popup_app = app.clone();
    let load_app = app.clone();
    let title_app = app.clone();
    let builder = WebviewBuilder::new(WEB_LABEL, WebviewUrl::External(url))
        .background_color(background)
        .on_navigation(move |url| {
            let allowed = is_web_url(url) || url.scheme() == "about";
            if !allowed {
                log::info!("web tab: blocked navigation to a {} link", url.scheme());
            }
            allowed
        })
        // Links that want a new window open in the same tab instead.
        .on_new_window(move |url, _features| {
            if is_web_url(&url) {
                let app = popup_app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Some(webview) = app.get_webview(WEB_LABEL) {
                        let _ = webview.navigate(url);
                    }
                });
            }
            NewWindowResponse::Deny
        })
        .on_page_load(move |_webview, payload| {
            let url = payload.url().to_string();
            let loading = matches!(payload.event(), PageLoadEvent::Started);
            load_app.state::<AppState>().web.update(&load_app, |state| {
                state.url = Some(url);
                state.loading = loading;
            });
        })
        .on_document_title_changed(move |_webview, title| {
            title_app
                .state::<AppState>()
                .web
                .update(&title_app, |state| state.title = title);
        });

    let layout = state.web.layout();
    let (position, size) = logical(layout.bounds);
    // `add_child` waits for the main thread; keep that off the async runtime threads.
    let webview = tauri::async_runtime::spawn_blocking(move || window.add_child(builder, position, size))
        .await
        .map_err(|e| AppError::msg(format!("internal error: {e}")))??;
    if !layout.visible {
        webview.hide()?;
    }
    log::info!("web tab created");
    Ok(())
}

pub fn set_bounds(app: &AppHandle, bounds: Bounds) -> AppResult<()> {
    let state = app.state::<AppState>();
    state.web.layout.lock().expect("web layout lock poisoned").bounds = bounds;
    if let Some(webview) = app.get_webview(WEB_LABEL) {
        let (position, size) = logical(bounds);
        webview.set_bounds(Rect {
            position: position.into(),
            size: size.into(),
        })?;
    }
    Ok(())
}

pub fn set_visible(app: &AppHandle, visible: bool) -> AppResult<()> {
    let state = app.state::<AppState>();
    state.web.layout.lock().expect("web layout lock poisoned").visible = visible;
    if let Some(webview) = app.get_webview(WEB_LABEL) {
        if visible {
            webview.show()?;
        } else {
            webview.hide()?;
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WebCommand {
    Back,
    Forward,
    Reload,
    Stop,
    /// Opens the current page in the default browser.
    OpenExternal,
}

pub fn command(app: &AppHandle, command: WebCommand) -> AppResult<()> {
    let Some(webview) = app.get_webview(WEB_LABEL) else {
        return Ok(());
    };
    match command {
        WebCommand::Back => webview.eval("history.back()")?,
        WebCommand::Forward => webview.eval("history.forward()")?,
        WebCommand::Reload => webview.reload()?,
        WebCommand::Stop => webview.eval("window.stop()")?,
        WebCommand::OpenExternal => open_externally(app, &webview)?,
    }
    Ok(())
}

fn open_externally(app: &AppHandle, webview: &Webview) -> AppResult<()> {
    let url = webview.url()?;
    if is_web_url(&url) {
        app.opener()
            .open_url(url.as_str(), None::<&str>)
            .map_err(|e| AppError::msg(e.to_string()))?;
        // The browser takes over; get out of the way.
        window::hide(app)?;
    }
    Ok(())
}

fn logical(bounds: Bounds) -> (LogicalPosition<f64>, LogicalSize<f64>) {
    (
        LogicalPosition::new(bounds.x, bounds.y),
        LogicalSize::new(bounds.width.max(0.0), bounds.height.max(0.0)),
    )
}

fn is_web_url(url: &Url) -> bool {
    matches!(url.scheme(), "http" | "https")
}

/// Turns address bar input into a URL: full web URLs and bare domains are opened,
/// everything else is searched.
pub fn resolve_input(input: &str) -> Option<Url> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }
    if let Ok(url) = Url::parse(input) {
        if is_web_url(&url) {
            return Some(url);
        }
    }
    if looks_like_address(input) {
        if let Ok(url) = Url::parse(&format!("https://{input}")) {
            return Some(url);
        }
    }
    Url::parse_with_params(SEARCH_URL, &[("q", input)]).ok()
}

/// `github.com`, `docs.rs/tauri`, `localhost:1420`: no spaces and a host with a dot or a port.
fn looks_like_address(input: &str) -> bool {
    if input.contains(char::is_whitespace) {
        return false;
    }
    let host = input.split(['/', '?', '#']).next().unwrap_or_default();
    let (name, port) = match host.rsplit_once(':') {
        Some((name, port)) => (name, Some(port)),
        None => (host, None),
    };
    if port.is_some_and(|p| p.is_empty() || !p.chars().all(|c| c.is_ascii_digit())) {
        return false;
    }
    let tld_ok = name.rsplit_once('.').is_some_and(|(label, tld)| {
        !label.is_empty() && tld.len() >= 2 && tld.chars().all(|c| c.is_ascii_alphabetic())
    });
    tld_ok || (name == "localhost" && port.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolved(input: &str) -> String {
        resolve_input(input).map(|u| u.to_string()).unwrap_or_default()
    }

    #[test]
    fn urls_and_domains_are_opened() {
        assert_eq!(resolved("https://tauri.app/start"), "https://tauri.app/start");
        assert_eq!(resolved("github.com"), "https://github.com/");
        assert_eq!(resolved("docs.rs/tauri?x=1"), "https://docs.rs/tauri?x=1");
        assert_eq!(resolved("localhost:1420"), "https://localhost:1420/");
    }

    #[test]
    fn everything_else_is_searched() {
        assert_eq!(
            resolved("tauri child webview"),
            "https://www.google.com/search?q=tauri+child+webview"
        );
        assert_eq!(resolved("v2.0"), "https://www.google.com/search?q=v2.0");
        assert_eq!(
            resolved("javascript:alert(1)"),
            "https://www.google.com/search?q=javascript%3Aalert%281%29"
        );
        assert_eq!(resolved("   "), "");
    }
}
