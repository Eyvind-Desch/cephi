//! Keeps the current conversation across restarts.
//!
//! One file holds both halves of a chat: what the UI shows (`ui`, opaque to the backend: tool cards,
//! search results, …) and what the model sees (`history`). "New chat" moves the file to the archive,
//! which keeps the most recent [`ARCHIVE_LIMIT`] chats.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::llm::ChatMessage;

const CURRENT_FILE: &str = "current.json";
const ARCHIVE_DIR: &str = "archive";
const ARCHIVE_LIMIT: usize = 50;
/// Bump when the file format changes incompatibly; older files are then ignored.
const VERSION: u32 = 1;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedChat {
    version: u32,
    pub saved_at: String,
    /// The messages as the frontend renders them.
    pub ui: Value,
    /// The conversation as sent to the model.
    pub history: Vec<ChatMessage>,
}

pub struct ChatStore {
    dir: PathBuf,
}

impl ChatStore {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    fn current(&self) -> PathBuf {
        self.dir.join(CURRENT_FILE)
    }

    /// The saved chat, if there is a readable one. A broken file is set aside, not deleted.
    pub fn load(&self) -> Option<SavedChat> {
        let path = self.current();
        let bytes = fs::read(&path).ok()?;
        match serde_json::from_slice::<SavedChat>(&bytes) {
            Ok(chat) if chat.version == VERSION => Some(chat),
            Ok(chat) => {
                log::warn!("chat file has version {}, expected {VERSION}; starting fresh", chat.version);
                set_aside(&path);
                None
            }
            Err(e) => {
                log::warn!("chat file unreadable ({e}); starting fresh");
                set_aside(&path);
                None
            }
        }
    }

    pub fn save(&self, ui: Value, history: &[ChatMessage]) -> io::Result<()> {
        let chat = SavedChat {
            version: VERSION,
            saved_at: chrono::Local::now().to_rfc3339(),
            ui,
            history: history.to_vec(),
        };
        let json = serde_json::to_vec(&chat).map_err(io::Error::other)?;
        fs::create_dir_all(&self.dir)?;
        // Write to a temporary file first, so a crash never leaves a half-written chat.
        let temp = self.dir.join(format!("{CURRENT_FILE}.tmp"));
        fs::write(&temp, json)?;
        fs::rename(temp, self.current())
    }

    /// Moves the current chat to the archive (for "new chat") and prunes old archived chats.
    pub fn archive(&self) -> io::Result<()> {
        let current = self.current();
        if !current.exists() {
            return Ok(());
        }
        let archive = self.dir.join(ARCHIVE_DIR);
        fs::create_dir_all(&archive)?;
        let name = format!("{}.json", chrono::Local::now().format("%Y-%m-%d_%H-%M-%S"));
        fs::rename(&current, archive.join(name))?;
        prune(&archive, ARCHIVE_LIMIT)
    }
}

/// Keeps the newest `keep` files (names start with a sortable timestamp).
fn prune(dir: &Path, keep: usize) -> io::Result<()> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    files.sort();
    let excess = files.len().saturating_sub(keep);
    for old in &files[..excess] {
        fs::remove_file(old)?;
    }
    Ok(())
}

fn set_aside(path: &Path) {
    let _ = fs::rename(path, path.with_extension("json.broken"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::ToolCall;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            Self(std::env::temp_dir().join(format!("cephi-chat-test-{}", uuid::Uuid::new_v4())))
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn conversation() -> Vec<ChatMessage> {
        let call = ToolCall {
            id: "call_1".into(),
            name: "web_search".into(),
            arguments: serde_json::json!({ "query": "tauri" }),
        };
        vec![
            ChatMessage::user("? tauri"),
            ChatMessage::assistant_tool_calls(vec![call.clone()]),
            ChatMessage::tool_result(&call, "[1] Tauri", true),
            ChatMessage::assistant("Tauri is a framework [1]."),
        ]
    }

    #[test]
    fn saves_and_loads_both_halves() {
        let dir = TempDir::new();
        let store = ChatStore::new(dir.0.clone());
        assert!(store.load().is_none());

        let ui = serde_json::json!([{ "id": "1", "role": "user", "text": "? tauri" }]);
        store.save(ui.clone(), &conversation()).unwrap();

        let loaded = store.load().unwrap();
        assert_eq!(loaded.ui, ui);
        assert_eq!(loaded.history.len(), 4);
        assert_eq!(loaded.history[1].tool_calls[0].name, "web_search");
        assert_eq!(loaded.history[2].tool_call_id.as_deref(), Some("call_1"));
    }

    #[test]
    fn archive_starts_a_new_chat_and_keeps_the_newest() {
        let dir = TempDir::new();
        let store = ChatStore::new(dir.0.clone());
        store.save(Value::Null, &conversation()).unwrap();
        store.archive().unwrap();
        assert!(store.load().is_none());
        assert_eq!(fs::read_dir(dir.0.join(ARCHIVE_DIR)).unwrap().count(), 1);

        let archive = dir.0.join(ARCHIVE_DIR);
        for i in 0..5 {
            fs::write(archive.join(format!("2020-01-0{i}.json")), "{}").unwrap();
        }
        prune(&archive, 3).unwrap();
        let mut left: Vec<String> = fs::read_dir(&archive)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        left.sort();
        assert_eq!(left.len(), 3);
        assert!(!left.contains(&"2020-01-00.json".to_string()), "oldest removed first");
    }

    #[test]
    fn broken_file_is_set_aside() {
        let dir = TempDir::new();
        fs::create_dir_all(&dir.0).unwrap();
        fs::write(dir.0.join(CURRENT_FILE), "not json").unwrap();
        let store = ChatStore::new(dir.0.clone());
        assert!(store.load().is_none());
        assert!(dir.0.join("current.json.broken").exists());
    }
}
