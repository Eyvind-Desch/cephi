//! Keeps the current conversation across restarts.
//!
//! One file holds both halves of a chat: what the UI shows (`ui`, opaque to the backend: tool cards,
//! search results, …) and what the model sees (`history`). "New chat" moves the file to the archive,
//! which keeps the most recent [`ARCHIVE_LIMIT`] chats; an archived chat can be opened again.

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

/// One entry of the chat archive (for the history list). Mirrored in `src/lib/ipc.ts`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchivedChat {
    /// File name without extension (the archive time).
    pub id: String,
    pub saved_at: String,
    /// The first question, shortened.
    pub title: String,
    pub messages: usize,
}

/// Just what the list needs; the model history is not read.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ArchiveHeader {
    saved_at: String,
    ui: Value,
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
                log::warn!(
                    "chat file has version {}, expected {VERSION}; starting fresh",
                    chat.version
                );
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
        let stamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
        // Two archives within a second (e.g. opening an old chat right after one) must not overwrite each other.
        let target = (1..)
            .map(|n| {
                archive.join(if n == 1 {
                    format!("{stamp}.json")
                } else {
                    format!("{stamp}_{n}.json")
                })
            })
            .find(|path| !path.exists())
            .expect("some free name");
        fs::rename(&current, target)?;
        prune(&archive, ARCHIVE_LIMIT)
    }

    fn archive_dir(&self) -> PathBuf {
        self.dir.join(ARCHIVE_DIR)
    }

    /// Archived chats, newest first. Unreadable files are skipped.
    pub fn list_archive(&self) -> Vec<ArchivedChat> {
        let Ok(entries) = fs::read_dir(self.archive_dir()) else {
            return Vec::new();
        };
        let mut chats: Vec<ArchivedChat> = entries
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
            .filter_map(|path| {
                let id = path.file_stem()?.to_string_lossy().into_owned();
                let header: ArchiveHeader = serde_json::from_slice(&fs::read(&path).ok()?).ok()?;
                let messages = header.ui.as_array().map(Vec::len).unwrap_or_default();
                let title = header
                    .ui
                    .as_array()
                    .and_then(|messages| messages.iter().find(|m| m["role"] == "user"))
                    .and_then(|m| m["text"].as_str())
                    .map(title_from)
                    .unwrap_or_default();
                Some(ArchivedChat {
                    id,
                    saved_at: header.saved_at,
                    title,
                    messages,
                })
            })
            .collect();
        chats.sort_by(|a, b| b.id.cmp(&a.id));
        chats
    }

    /// Makes an archived chat the current one; the current chat goes to the archive first.
    pub fn open_archived(&self, id: &str) -> io::Result<SavedChat> {
        let path = self.archived_path(id)?;
        let chat: SavedChat = serde_json::from_slice(&fs::read(&path)?).map_err(io::Error::other)?;
        if chat.version != VERSION {
            return Err(io::Error::other(format!("chat file has version {}", chat.version)));
        }
        // Out of the archive first, so pruning after archiving the current chat can't remove it.
        let opening = self.dir.join(format!("{CURRENT_FILE}.opening"));
        fs::rename(&path, &opening)?;
        if let Err(e) = self.archive() {
            let _ = fs::rename(&opening, &path);
            return Err(e);
        }
        fs::rename(&opening, self.current())?;
        Ok(chat)
    }

    pub fn delete_archived(&self, id: &str) -> io::Result<()> {
        fs::remove_file(self.archived_path(id)?)
    }

    /// Only names the archive itself creates, so an id can't point outside it.
    fn archived_path(&self, id: &str) -> io::Result<PathBuf> {
        let valid = !id.is_empty() && id.chars().all(|c| c.is_ascii_digit() || c == '-' || c == '_');
        if !valid {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid chat id"));
        }
        Ok(self.archive_dir().join(format!("{id}.json")))
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

const TITLE_CHARS: usize = 80;

fn title_from(text: &str) -> String {
    let line = text.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or_default();
    if line.chars().count() > TITLE_CHARS {
        format!("{}…", line.chars().take(TITLE_CHARS).collect::<String>().trim_end())
    } else {
        line.to_string()
    }
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
    fn archived_chats_can_be_listed_opened_and_deleted() {
        let dir = TempDir::new();
        let store = ChatStore::new(dir.0.clone());
        let ui = |text: &str| serde_json::json!([{ "id": "1", "role": "user", "text": text }, { "id": "2", "role": "assistant" }]);
        store
            .save(
                ui("first question
more"),
                &conversation(),
            )
            .unwrap();
        store.archive().unwrap();
        store.save(ui("second"), &conversation()).unwrap();
        store.archive().unwrap();

        let list = store.list_archive();
        assert_eq!(list.len(), 2, "same second gets a second name");
        assert_eq!(list[0].title, "second", "newest first");
        assert_eq!(list[1].title, "first question");
        assert_eq!(list[1].messages, 2);

        store.save(ui("current"), &conversation()).unwrap();
        let opened = store.open_archived(&list[1].id).unwrap();
        assert_eq!(opened.history.len(), 4);
        assert_eq!(
            store.load().unwrap().ui,
            ui("first question
more")
        );
        let titles: Vec<String> = store.list_archive().into_iter().map(|c| c.title).collect();
        assert!(titles.contains(&"current".to_string()), "the open chat was archived");
        assert!(!titles.contains(&"first question".to_string()));

        store.delete_archived(&list[0].id).unwrap();
        assert_eq!(store.list_archive().len(), 1);
        assert!(store.open_archived("../current").is_err());
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
