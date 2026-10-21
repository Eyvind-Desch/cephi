//! The notes tab: short notes and checklists, kept in one file next to the chats.
//!
//! The UI edits notes directly; the agent adds and searches them through the `note_add` and
//! `notes_search` builtins. Every change emits [`EVENT_NOTES_CHANGED`] so the tab stays current.

use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

const FILE: &str = "notes.json";
/// Bump when the file format changes incompatibly; older files are then set aside.
const VERSION: u32 = 1;

pub const EVENT_NOTES_CHANGED: &str = "cephi://notes-changed";

/// Background colors the UI offers; anything else is shown as `default`.
pub const COLORS: &[&str] = &["default", "red", "orange", "yellow", "green", "teal", "blue", "purple"];

/// One note. Mirrored in `src/lib/ipc.ts`.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    /// Empty for a new note; the store assigns one.
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub text: String,
    /// A checklist note shows `items` instead of `text`.
    #[serde(default)]
    pub checklist: bool,
    #[serde(default)]
    pub items: Vec<NoteItem>,
    #[serde(default = "default_color")]
    pub color: String,
    #[serde(default)]
    pub pinned: bool,
    /// RFC 3339; set by the store.
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NoteItem {
    pub text: String,
    #[serde(default)]
    pub done: bool,
}

fn default_color() -> String {
    "default".into()
}

impl Note {
    pub fn is_empty(&self) -> bool {
        self.title.trim().is_empty()
            && self.text.trim().is_empty()
            && self.items.iter().all(|item| item.text.trim().is_empty())
    }

    fn matches(&self, needle: &str) -> bool {
        let needle = needle.to_lowercase();
        self.title.to_lowercase().contains(&needle)
            || self.text.to_lowercase().contains(&needle)
            || self.items.iter().any(|item| item.text.to_lowercase().contains(&needle))
    }

    /// Plain text for the model.
    fn for_model(&self) -> String {
        let mut out = String::new();
        if !self.title.trim().is_empty() {
            out.push_str(&format!("# {}\n", self.title.trim()));
        }
        if self.checklist {
            for item in &self.items {
                out.push_str(&format!("- [{}] {}\n", if item.done { "x" } else { " " }, item.text));
            }
        } else {
            out.push_str(self.text.trim());
            out.push('\n');
        }
        out
    }
}

#[derive(Deserialize, Serialize)]
struct NotesFile {
    version: u32,
    notes: Vec<Note>,
}

pub struct NoteStore {
    path: PathBuf,
    /// Serializes read-modify-write; the UI and the agent may write at the same time.
    lock: Mutex<()>,
}

impl NoteStore {
    pub fn new(dir: PathBuf) -> Self {
        Self {
            path: dir.join(FILE),
            lock: Mutex::new(()),
        }
    }

    /// All notes, pinned first, then most recently changed.
    pub fn list(&self) -> Vec<Note> {
        let _guard = self.lock.lock().expect("notes lock poisoned");
        let mut notes = self.read();
        sort(&mut notes);
        notes
    }

    /// Creates the note (empty `id`) or replaces the one with its id. An empty note is removed.
    pub fn save(&self, mut note: Note) -> io::Result<Option<Note>> {
        let _guard = self.lock.lock().expect("notes lock poisoned");
        let mut notes = self.read();
        let now = chrono::Local::now().to_rfc3339();
        if !COLORS.contains(&note.color.as_str()) {
            note.color = default_color();
        }
        note.updated_at = now.clone();
        let existing = notes.iter().position(|n| !note.id.is_empty() && n.id == note.id);

        if note.is_empty() {
            if let Some(index) = existing {
                notes.remove(index);
                self.write(&notes)?;
            }
            return Ok(None);
        }
        match existing {
            Some(index) => {
                note.created_at = notes[index].created_at.clone();
                notes[index] = note.clone();
            }
            None => {
                note.id = uuid::Uuid::new_v4().to_string();
                note.created_at = now;
                notes.push(note.clone());
            }
        }
        self.write(&notes)?;
        Ok(Some(note))
    }

    pub fn delete(&self, id: &str) -> io::Result<()> {
        let _guard = self.lock.lock().expect("notes lock poisoned");
        let mut notes = self.read();
        notes.retain(|n| n.id != id);
        self.write(&notes)
    }

    /// Notes containing `query` (all notes for an empty query), as text for the model.
    pub fn search_for_model(&self, query: &str) -> Vec<String> {
        let query = query.trim();
        self.list()
            .into_iter()
            .filter(|note| query.is_empty() || note.matches(query))
            .map(|note| note.for_model())
            .collect()
    }

    /// A broken file is set aside, not deleted, so nothing is lost by accident.
    fn read(&self) -> Vec<Note> {
        let Ok(bytes) = fs::read(&self.path) else {
            return Vec::new();
        };
        match serde_json::from_slice::<NotesFile>(&bytes) {
            Ok(file) if file.version == VERSION => file.notes,
            Ok(file) => {
                log::warn!("notes file has version {}, expected {VERSION}; setting it aside", file.version);
                self.set_aside();
                Vec::new()
            }
            Err(e) => {
                log::warn!("notes file unreadable ({e}); setting it aside");
                self.set_aside();
                Vec::new()
            }
        }
    }

    fn write(&self, notes: &[Note]) -> io::Result<()> {
        let file = NotesFile {
            version: VERSION,
            notes: notes.to_vec(),
        };
        let json = serde_json::to_vec_pretty(&file).map_err(io::Error::other)?;
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        // Write to a temporary file first, so a crash never leaves half-written notes.
        let temp = self.path.with_extension("json.tmp");
        fs::write(&temp, json)?;
        fs::rename(temp, &self.path)
    }

    fn set_aside(&self) {
        let _ = fs::rename(&self.path, self.path.with_extension("json.broken"));
    }
}

fn sort(notes: &mut [Note]) {
    notes.sort_by(|a, b| b.pinned.cmp(&a.pinned).then_with(|| b.updated_at.cmp(&a.updated_at)));
}

/// Tells the notes tab to reload (after the agent changed something).
pub fn notify(app: &AppHandle) {
    if let Err(e) = app.emit(EVENT_NOTES_CHANGED, ()) {
        log::error!("emit notes change: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            Self(std::env::temp_dir().join(format!("cephi-notes-test-{}", uuid::Uuid::new_v4())))
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn note(title: &str, text: &str) -> Note {
        Note {
            id: String::new(),
            title: title.into(),
            text: text.into(),
            checklist: false,
            items: Vec::new(),
            color: "default".into(),
            pinned: false,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn saves_updates_and_deletes() {
        let dir = TempDir::new();
        let store = NoteStore::new(dir.0.clone());
        assert!(store.list().is_empty());

        let saved = store.save(note("Shopping", "milk")).unwrap().unwrap();
        assert!(!saved.id.is_empty());
        assert_eq!(store.list().len(), 1);

        let mut changed = saved.clone();
        changed.text = "milk, bread".into();
        changed.color = "not-a-color".into();
        let updated = store.save(changed).unwrap().unwrap();
        assert_eq!(updated.id, saved.id);
        assert_eq!(updated.created_at, saved.created_at);
        assert_eq!(updated.color, "default", "unknown colors fall back");
        assert_eq!(store.list()[0].text, "milk, bread");

        store.delete(&saved.id).unwrap();
        assert!(store.list().is_empty());
    }

    #[test]
    fn emptied_note_is_removed_and_pinned_come_first() {
        let dir = TempDir::new();
        let store = NoteStore::new(dir.0.clone());
        let first = store.save(note("", "first")).unwrap().unwrap();
        let mut pinned = note("", "pinned");
        pinned.pinned = true;
        store.save(pinned).unwrap();
        store.save(note("", "newest")).unwrap();
        assert_eq!(store.list()[0].text, "pinned");

        assert!(store.save(note("", "  ")).unwrap().is_none(), "empty new note is not stored");
        let mut emptied = first;
        emptied.text.clear();
        store.save(emptied).unwrap();
        assert_eq!(store.list().len(), 2);
    }

    #[test]
    fn search_covers_title_text_and_checklist() {
        let dir = TempDir::new();
        let store = NoteStore::new(dir.0.clone());
        store.save(note("Ideas", "a game about octopuses")).unwrap();
        let mut list = note("Groceries", "");
        list.checklist = true;
        list.items = vec![NoteItem { text: "Oat milk".into(), done: true }];
        store.save(list).unwrap();

        assert_eq!(store.search_for_model("").len(), 2);
        let found = store.search_for_model("MILK");
        assert_eq!(found, vec!["# Groceries\n- [x] Oat milk\n".to_string()]);
        assert_eq!(store.search_for_model("octopus").len(), 1);
    }

    #[test]
    fn broken_file_is_set_aside() {
        let dir = TempDir::new();
        fs::create_dir_all(&dir.0).unwrap();
        fs::write(dir.0.join(FILE), "not json").unwrap();
        let store = NoteStore::new(dir.0.clone());
        assert!(store.list().is_empty());
        assert!(dir.0.join("notes.json.broken").exists());
    }
}
