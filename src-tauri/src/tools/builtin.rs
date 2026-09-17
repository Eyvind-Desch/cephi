//! Tools implemented in Rust, referenced from manifests via `{"type": "builtin", "id": "…"}`.

use serde::Deserialize;

use super::args::Args;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Builtin {
    Datetime,
    ClipboardRead,
    ClipboardWrite,
}

impl Builtin {
    pub fn as_str(self) -> &'static str {
        match self {
            Builtin::Datetime => "datetime",
            Builtin::ClipboardRead => "clipboard_read",
            Builtin::ClipboardWrite => "clipboard_write",
        }
    }

    /// Parameters the manifest must declare for this builtin.
    pub fn required_params(self) -> &'static [&'static str] {
        match self {
            Builtin::ClipboardWrite => &["text"],
            Builtin::Datetime | Builtin::ClipboardRead => &[],
        }
    }

    pub async fn run(self, args: &Args) -> Result<String, String> {
        match self {
            Builtin::Datetime => Ok(chrono::Local::now().format("%A, %Y-%m-%d %H:%M:%S (%:z)").to_string()),
            Builtin::ClipboardRead => {
                blocking(|| {
                    let text = clipboard()?
                        .get_text()
                        .map_err(|e| format!("Zwischenablage enthält keinen Text: {e}"))?;
                    Ok(if text.is_empty() {
                        "(Zwischenablage ist leer)".into()
                    } else {
                        text
                    })
                })
                .await
            }
            Builtin::ClipboardWrite => {
                let text = args
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                blocking(move || {
                    clipboard()?
                        .set_text(text)
                        .map_err(|e| format!("Schreiben fehlgeschlagen: {e}"))?;
                    Ok("Text in die Zwischenablage kopiert.".into())
                })
                .await
            }
        }
    }
}

fn clipboard() -> Result<arboard::Clipboard, String> {
    arboard::Clipboard::new().map_err(|e| format!("Zwischenablage nicht verfügbar: {e}"))
}

async fn blocking<F>(f: F) -> Result<String, String>
where
    F: FnOnce() -> Result<String, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| format!("Interner Fehler: {e}"))?
}
