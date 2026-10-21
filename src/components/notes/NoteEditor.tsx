import clsx from "clsx";
import { ListChecks, Palette, Pin, Plus, Square, SquareCheck, Text, Trash2, X } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { formatWhen } from "../../lib/dates";
import type { Note, NoteItem } from "../../lib/ipc";
import { toChecklist, toText, useNotes } from "../../store/notes";
import { IconButton } from "../ui/IconButton";
import { EDITOR_CLASS, NOTE_COLORS, SWATCH_CLASS } from "./colors";

/** Pause after typing before the note is written to disk. */
const AUTOSAVE_MS = 500;
/** How long "Really delete?" waits for the confirming second click. */
const CONFIRM_DELETE_MS = 3000;

/**
 * Edits one note on top of the list. Changes are saved while typing and when the editor closes;
 * a note emptied completely is removed, like in Google Keep.
 */
export function NoteEditor({ initial }: { initial: Note }) {
  const { t, i18n } = useTranslation();
  const save = useNotes((s) => s.save);
  const remove = useNotes((s) => s.remove);
  const close = useNotes((s) => s.close);
  const [note, setNote] = useState(initial);
  const [palette, setPalette] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  /** Item to focus after the list changed (Enter adds one, Backspace removes one). */
  const [focusItem, setFocusItem] = useState<number | null>(null);
  const itemRefs = useRef<(HTMLInputElement | null)[]>([]);
  const textRef = useRef<HTMLTextAreaElement>(null);

  // Saves run one after another, so a new note gets its id before the next save uses it.
  const latest = useRef(note);
  latest.current = note;
  const id = useRef(initial.id);
  const dirty = useRef(false);
  const chain = useRef<Promise<void>>(Promise.resolve());

  const persist = useCallback(() => {
    if (!dirty.current) return chain.current;
    dirty.current = false;
    chain.current = chain.current.then(async () => {
      try {
        const saved = await save({ ...latest.current, id: id.current });
        id.current = saved?.id ?? "";
      } catch {
        // The store shows the error; the next change tries again.
        dirty.current = true;
      }
    });
    return chain.current;
  }, [save]);

  const update = (change: (note: Note) => Note) => {
    dirty.current = true;
    setNote(change);
  };

  const finish = useCallback(() => {
    void persist();
    close();
  }, [persist, close]);

  useEffect(() => {
    if (!dirty.current) return;
    const timer = window.setTimeout(() => void persist(), AUTOSAVE_MS);
    return () => window.clearTimeout(timer);
  }, [note, persist]);

  // Leaving the tab or hiding Cephi must not lose the last keystrokes.
  useEffect(() => () => void persist(), [persist]);

  // Escape closes the note; it must not also hide the window.
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.preventDefault();
      e.stopPropagation();
      finish();
    };
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, [finish]);

  useEffect(() => {
    if (!initial.id) {
      if (initial.checklist) itemRefs.current[0]?.focus();
      else textRef.current?.focus();
    }
  }, [initial]);

  useEffect(() => {
    if (focusItem === null) return;
    const input = itemRefs.current[focusItem];
    input?.focus();
    input?.setSelectionRange(input.value.length, input.value.length);
    setFocusItem(null);
  }, [focusItem]);

  useEffect(() => {
    if (!confirmDelete) return;
    const timer = setTimeout(() => setConfirmDelete(false), CONFIRM_DELETE_MS);
    return () => clearTimeout(timer);
  }, [confirmDelete]);

  const setItems = (change: (items: NoteItem[]) => NoteItem[]) =>
    update((n) => ({ ...n, items: change(n.items) }));

  const deleteNote = () => {
    dirty.current = false;
    // Wait for a save in flight: a brand-new note only has its id afterwards.
    void chain.current.then(() => (id.current ? remove(id.current) : undefined));
    close();
  };

  return (
    <div
      className="absolute inset-0 z-10 flex items-start justify-center bg-black/25 p-3 backdrop-blur-[2px]"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) finish();
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label={note.title || t("notes.text")}
        className={clsx(
          "flex max-h-full w-full flex-col overflow-hidden rounded-2xl border border-line shadow-xl shadow-black/25",
          EDITOR_CLASS[note.color],
        )}
      >
        <div className="flex items-start gap-1 pt-2 pr-2 pl-3.5">
          <input
            value={note.title}
            onChange={(e) => update((n) => ({ ...n, title: e.target.value }))}
            onKeyDown={(e) => {
              if (e.key !== "Enter") return;
              e.preventDefault();
              if (note.checklist) itemRefs.current[0]?.focus();
              else textRef.current?.focus();
            }}
            placeholder={t("notes.title")}
            spellCheck={false}
            className="h-9 min-w-0 flex-1 bg-transparent text-title font-medium text-fg outline-none placeholder:text-subtle"
          />
          <IconButton
            icon={Pin}
            label={t(note.pinned ? "notes.unpin" : "notes.pin")}
            active={note.pinned}
            onClick={() => update((n) => ({ ...n, pinned: !n.pinned }))}
          />
        </div>

        <div className="min-h-0 flex-1 overflow-y-auto px-3.5 pb-2">
          {note.checklist ? (
            <ChecklistEditor
              items={note.items}
              itemRefs={itemRefs}
              setItems={setItems}
              focus={setFocusItem}
            />
          ) : (
            <textarea
              ref={textRef}
              value={note.text}
              onChange={(e) => update((n) => ({ ...n, text: e.target.value }))}
              placeholder={t("notes.text")}
              spellCheck={false}
              className="block min-h-28 w-full resize-none bg-transparent text-body leading-relaxed text-fg outline-none field-sizing-content placeholder:text-subtle"
            />
          )}
        </div>

        {palette && (
          <div
            role="radiogroup"
            aria-label={t("notes.color")}
            className="flex flex-wrap items-center gap-1.5 border-t border-line px-3 py-2"
          >
            {NOTE_COLORS.map((color) => (
              <button
                key={color}
                type="button"
                role="radio"
                aria-checked={note.color === color}
                title={t(`notes.colors.${color}`)}
                aria-label={t(`notes.colors.${color}`)}
                onClick={() => update((n) => ({ ...n, color }))}
                className={clsx(
                  "size-6 rounded-full border transition-transform hover:scale-110",
                  SWATCH_CLASS[color],
                  note.color === color ? "border-fg ring-2 ring-accent/40" : "border-line",
                )}
              />
            ))}
          </div>
        )}

        <div className="flex items-center gap-0.5 border-t border-line px-1.5 py-1.5">
          <IconButton
            icon={Palette}
            label={t("notes.color")}
            active={palette}
            onClick={() => setPalette((open) => !open)}
          />
          <IconButton
            icon={note.checklist ? Text : ListChecks}
            label={t(note.checklist ? "notes.plainText" : "notes.checklist")}
            onClick={() => {
              update((n) => (n.checklist ? toText(n) : toChecklist(n)));
              if (!note.checklist) setFocusItem(0);
            }}
          />
          <button
            type="button"
            onClick={() => (confirmDelete ? deleteNote() : setConfirmDelete(true))}
            title={t("notes.delete")}
            aria-label={t("notes.delete")}
            className={clsx(
              "flex h-8 items-center gap-1 rounded-lg px-2 text-hint transition-colors hover:bg-sunken",
              confirmDelete ? "text-danger" : "text-muted hover:text-danger",
            )}
          >
            {confirmDelete ? t("notes.confirmDelete") : <Trash2 size={16} />}
          </button>
          <span className="ml-auto truncate px-2 text-hint text-subtle">
            {note.updatedAt && t("notes.edited", { when: formatWhen(note.updatedAt, i18n.language) })}
          </span>
          <button
            type="button"
            onClick={finish}
            className="h-8 shrink-0 rounded-lg px-3 text-body font-medium text-fg transition-colors hover:bg-sunken"
          >
            {t("notes.close")}
          </button>
        </div>
      </div>
    </div>
  );
}

function ChecklistEditor({
  items,
  itemRefs,
  setItems,
  focus,
}: {
  items: NoteItem[];
  itemRefs: React.RefObject<(HTMLInputElement | null)[]>;
  setItems: (change: (items: NoteItem[]) => NoteItem[]) => void;
  focus: (index: number) => void;
}) {
  const { t } = useTranslation();

  const change = (index: number, item: Partial<NoteItem>) =>
    setItems((list) => list.map((current, i) => (i === index ? { ...current, ...item } : current)));

  return (
    <ul className="space-y-0.5">
      {items.map((item, index) => (
        <li key={index} className="group flex items-center gap-2">
          <button
            type="button"
            onClick={() => change(index, { done: !item.done })}
            aria-label={item.text || t("notes.item")}
            aria-pressed={item.done}
            className="shrink-0 text-subtle hover:text-fg"
          >
            {item.done ? <SquareCheck size={15} /> : <Square size={15} />}
          </button>
          <input
            ref={(el) => {
              itemRefs.current[index] = el;
            }}
            value={item.text}
            onChange={(e) => change(index, { text: e.target.value })}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                setItems((list) => [...list.slice(0, index + 1), { text: "", done: false }, ...list.slice(index + 1)]);
                focus(index + 1);
              } else if (e.key === "Backspace" && !item.text && items.length > 0) {
                e.preventDefault();
                setItems((list) => list.filter((_, i) => i !== index));
                focus(Math.max(0, index - 1));
              }
            }}
            placeholder={t("notes.item")}
            spellCheck={false}
            className={clsx(
              "h-7 min-w-0 flex-1 bg-transparent text-body outline-none placeholder:text-subtle",
              item.done ? "text-muted line-through" : "text-fg",
            )}
          />
          <button
            type="button"
            onClick={() => setItems((list) => list.filter((_, i) => i !== index))}
            title={t("notes.removeItem")}
            aria-label={t("notes.removeItem")}
            className="grid size-6 shrink-0 place-items-center rounded-md text-subtle opacity-0 group-focus-within:opacity-100 group-hover:opacity-100 hover:text-fg"
          >
            <X size={13} />
          </button>
        </li>
      ))}
      <li>
        <button
          type="button"
          onClick={() => {
            setItems((list) => [...list, { text: "", done: false }]);
            focus(items.length);
          }}
          className="flex h-7 items-center gap-2 text-body text-subtle hover:text-fg"
        >
          <Plus size={15} /> {t("notes.item")}
        </button>
      </li>
    </ul>
  );
}
