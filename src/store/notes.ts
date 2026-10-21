import { create } from "zustand";
import { errorText } from "../lib/errors";
import { api, type Note } from "../lib/ipc";
import { useUi } from "./ui";

interface NotesState {
  /** `null` until loaded. Pinned first, then most recently changed (as the backend sorts them). */
  notes: Note[] | null;
  error: string | null;
  /** The note open in the editor; a new one has an empty id until its first save. */
  editing: Note | null;

  load: () => Promise<void>;
  /** Saves and puts the result into the list; resolves with the stored note (`null` = removed as empty). */
  save: (note: Note) => Promise<Note | null>;
  remove: (id: string) => Promise<void>;
  open: (note: Note) => void;
  close: () => void;
}

export function emptyNote(): Note {
  return {
    id: "",
    title: "",
    text: "",
    checklist: false,
    items: [],
    color: "default",
    pinned: false,
    createdAt: "",
    updatedAt: "",
  };
}

/** The backend's order: pinned first, then most recently changed. */
function sorted(notes: Note[]): Note[] {
  return [...notes].sort(
    (a, b) => Number(b.pinned) - Number(a.pinned) || b.updatedAt.localeCompare(a.updatedAt),
  );
}

export const useNotes = create<NotesState>()((set, get) => ({
  notes: null,
  error: null,
  editing: null,

  load: async () => {
    try {
      set({ notes: await api.notesList(), error: null });
    } catch (e) {
      set({ error: errorText(e) });
    }
  },

  save: async (note) => {
    try {
      const saved = await api.notesSave(note);
      const others = (get().notes ?? []).filter((n) => n.id !== note.id && n.id !== saved?.id);
      set({ notes: sorted(saved ? [...others, saved] : others), error: null });
      return saved;
    } catch (e) {
      set({ error: errorText(e) });
      throw e;
    }
  },

  remove: async (id) => {
    try {
      await api.notesDelete(id);
      set((s) => ({ notes: s.notes?.filter((n) => n.id !== id) ?? null, error: null }));
    } catch (e) {
      set({ error: errorText(e) });
    }
  },

  open: (editing) => set({ editing }),
  close: () => set({ editing: null }),
}));

// Leaving the tab closes the editor (it saves on its way out). Kept open, it would come back later
// with the note's text from before the edit.
useUi.subscribe((state, previous) => {
  if (previous.view === "notes" && state.view !== "notes") useNotes.getState().close();
});

/** Lines of text become list items and back, so switching a note to a checklist loses nothing. */
export function toChecklist(note: Note): Note {
  const items = note.text
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean)
    .map((text) => ({ text, done: false }));
  return { ...note, checklist: true, items, text: "" };
}

export function toText(note: Note): Note {
  const text = note.items
    .map((item) => item.text)
    .filter((line) => line.trim())
    .join("\n");
  return { ...note, checklist: false, items: [], text };
}
