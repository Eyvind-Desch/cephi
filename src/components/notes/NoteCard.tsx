import clsx from "clsx";
import { Pin, Square, SquareCheck } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { Note } from "../../lib/ipc";
import { useNotes } from "../../store/notes";
import { CARD_CLASS } from "./colors";

/** Open list items a card shows before "+ n more". */
const PREVIEW_ITEMS = 6;

/** One note in the grid: click to edit; pin and checkboxes work right on the card. */
export function NoteCard({ note }: { note: Note }) {
  const { t } = useTranslation();
  const save = useNotes((s) => s.save);
  const open = useNotes((s) => s.open);

  const togglePin = () => void save({ ...note, pinned: !note.pinned }).catch(() => undefined);
  const toggleItem = (index: number) =>
    void save({
      ...note,
      items: note.items.map((item, i) => (i === index ? { ...item, done: !item.done } : item)),
    }).catch(() => undefined);

  const openItems = note.items.map((item, index) => ({ item, index })).filter(({ item }) => !item.done);
  const doneCount = note.items.length - openItems.length;
  const hidden = openItems.length - PREVIEW_ITEMS;

  return (
    <article
      role="button"
      tabIndex={0}
      onClick={() => open(note)}
      onKeyDown={(e) => {
        if (e.target === e.currentTarget && (e.key === "Enter" || e.key === " ")) {
          e.preventDefault();
          open(note);
        }
      }}
      className={clsx(
        "group relative mb-2 cursor-pointer break-inside-avoid rounded-xl border px-3 py-2.5 outline-none transition-shadow",
        "hover:shadow-md hover:shadow-black/10 focus-visible:ring-2 focus-visible:ring-accent/50",
        CARD_CLASS[note.color],
      )}
    >
      <button
        type="button"
        onClick={(e) => {
          e.stopPropagation();
          togglePin();
        }}
        title={t(note.pinned ? "notes.unpin" : "notes.pin")}
        aria-label={t(note.pinned ? "notes.unpin" : "notes.pin")}
        aria-pressed={note.pinned}
        className={clsx(
          "absolute top-1.5 right-1.5 grid size-6 place-items-center rounded-md transition-opacity hover:bg-sunken",
          note.pinned
            ? "text-fg"
            : "text-subtle opacity-0 group-hover:opacity-100 hover:text-fg focus-visible:opacity-100",
        )}
      >
        <Pin size={13} className={clsx(note.pinned && "fill-current")} />
      </button>

      {note.title && <h3 className="mb-1 pr-6 text-body font-medium break-words text-fg">{note.title}</h3>}

      {note.checklist ? (
        <ul className={clsx("space-y-0.5", !note.title && "pr-5")}>
          {openItems.slice(0, PREVIEW_ITEMS).map(({ item, index }) => (
            <li key={index} className="flex items-start gap-1.5 text-body text-fg">
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  toggleItem(index);
                }}
                aria-label={item.text}
                className="mt-0.5 shrink-0 text-subtle hover:text-fg"
              >
                <Square size={13} />
              </button>
              <span className="min-w-0 break-words">{item.text}</span>
            </li>
          ))}
          {hidden > 0 && <li className="pl-5 text-hint text-muted">{t("notes.more", { count: hidden })}</li>}
          {doneCount > 0 && (
            <li className="flex items-center gap-1.5 text-hint text-muted">
              <SquareCheck size={13} className="shrink-0" />
              {t("notes.done", { count: doneCount })}
            </li>
          )}
        </ul>
      ) : (
        <p
          className={clsx(
            "line-clamp-[12] text-body leading-relaxed break-words whitespace-pre-wrap text-fg",
            !note.title && "pr-5",
          )}
        >
          {note.text}
        </p>
      )}
    </article>
  );
}
