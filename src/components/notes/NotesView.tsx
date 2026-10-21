import { ListChecks, Search } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { onNotesChanged, type Note } from "../../lib/ipc";
import { emptyNote, useNotes } from "../../store/notes";
import { NoteCard } from "./NoteCard";
import { NoteEditor } from "./NoteEditor";

/** Notes tab: a Keep-like board of notes and checklists, pinned ones on top. */
export function NotesView() {
  const { t } = useTranslation();
  const notes = useNotes((s) => s.notes);
  const error = useNotes((s) => s.error);
  const editing = useNotes((s) => s.editing);
  const load = useNotes((s) => s.load);
  const open = useNotes((s) => s.open);
  const [query, setQuery] = useState("");

  useEffect(() => {
    void load();
    // The agent may add notes from the chat (`note_add`).
    const unlisten = onNotesChanged(() => void load());
    return () => void unlisten.then((fn) => fn());
  }, [load]);

  const needle = query.trim().toLowerCase();
  const visible = notes?.filter((note) => matches(note, needle)) ?? [];
  const pinned = visible.filter((note) => note.pinned);
  const others = visible.filter((note) => !note.pinned);

  return (
    <div className="relative flex min-h-0 flex-1 flex-col">
      <div className="px-3 pt-3">
        <div className="relative">
          <Search size={13} className="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-subtle" />
          <input
            value={query}
            spellCheck={false}
            placeholder={t("notes.search")}
            onChange={(e) => setQuery(e.target.value)}
            className="h-8 w-full rounded-lg border border-line bg-sunken pr-2.5 pl-8 text-body text-fg outline-none placeholder:text-subtle focus:border-accent/40"
          />
        </div>
      </div>

      <div className="flex-1 overflow-y-auto px-3 pt-3 pb-5">
        <div className="mb-4 flex items-center rounded-xl border border-line bg-raised shadow-sm shadow-black/5 transition-colors focus-within:border-accent/50">
          <button
            type="button"
            onClick={() => open(emptyNote())}
            className="h-11 min-w-0 flex-1 px-3.5 text-left text-body text-subtle hover:text-muted"
          >
            {t("notes.takeNote")}
          </button>
          <button
            type="button"
            onClick={() => open({ ...emptyNote(), checklist: true, items: [{ text: "", done: false }] })}
            title={t("notes.checklist")}
            aria-label={t("notes.checklist")}
            className="mr-1.5 grid size-8 place-items-center rounded-lg text-muted transition-colors hover:bg-sunken hover:text-fg"
          >
            <ListChecks size={16} />
          </button>
        </div>

        {error && <p className="mb-2 px-1 text-hint text-danger select-text">{error}</p>}
        {notes?.length === 0 && (
          <p className="rounded-xl border border-dashed border-line px-3 py-3 text-body text-muted">{t("notes.empty")}</p>
        )}
        {notes && notes.length > 0 && visible.length === 0 && (
          <p className="px-1 text-body text-muted">{t("notes.noMatch")}</p>
        )}

        {pinned.length > 0 && <Section label={t("notes.pinned")} notes={pinned} />}
        {others.length > 0 && <Section label={pinned.length > 0 ? t("notes.others") : null} notes={others} />}
      </div>

      {/* A fresh key per note, so the editor never carries one note's draft into the next. */}
      {editing && <NoteEditor key={editing.id || "new"} initial={editing} />}
    </div>
  );
}

function Section({ label, notes }: { label: string | null; notes: Note[] }) {
  return (
    <section className="mb-3">
      {label && <h2 className="mb-1.5 px-1 text-hint font-medium tracking-wide text-subtle uppercase">{label}</h2>}
      {/* Columns give the Keep look: cards keep their height and fill the gaps. */}
      <div className="columns-2 gap-2">
        {notes.map((note) => (
          <NoteCard key={note.id} note={note} />
        ))}
      </div>
    </section>
  );
}

function matches(note: Note, needle: string): boolean {
  if (!needle) return true;
  return (
    note.title.toLowerCase().includes(needle) ||
    note.text.toLowerCase().includes(needle) ||
    note.items.some((item) => item.text.toLowerCase().includes(needle))
  );
}
