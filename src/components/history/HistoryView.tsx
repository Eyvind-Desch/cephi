import clsx from "clsx";
import { ArrowLeft, MessageSquare, Search, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { errorText } from "../../lib/errors";
import { api, type ArchivedChat } from "../../lib/ipc";
import { useChat } from "../../store/chat";
import { useUi } from "../../store/ui";

/** How long "Really delete?" waits for the confirming second click. */
const CONFIRM_DELETE_MS = 3000;

/** Earlier chats (`/history`): pick one to continue it, the current chat is archived meanwhile. */
export function HistoryView() {
  const { t } = useTranslation();
  const setView = useUi((s) => s.setView);
  const openArchived = useChat((s) => s.openArchived);
  const running = useChat((s) => s.running);
  const [chats, setChats] = useState<ArchivedChat[] | null>(null);
  const [query, setQuery] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void api.chatArchiveList().then(setChats);
  }, []);

  const needle = query.trim().toLowerCase();
  const visible = chats?.filter((chat) => chat.title.toLowerCase().includes(needle)) ?? [];

  const open = (id: string) => {
    setError(null);
    openArchived(id).catch((e: unknown) => setError(errorText(e)));
  };

  const remove = (id: string) => {
    setError(null);
    api
      .chatArchiveDelete(id)
      .then(() => setChats((list) => list?.filter((chat) => chat.id !== id) ?? null))
      .catch((e: unknown) => setError(errorText(e)));
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex items-center gap-2 px-2 pt-2">
        <button
          type="button"
          onClick={() => setView("chat")}
          title={t("keys.backTitle")}
          className="flex items-center gap-1.5 rounded-md px-2 py-1 text-[12.5px] text-muted transition-colors hover:bg-sunken hover:text-fg"
        >
          <ArrowLeft size={14} /> {t("keys.back")}
        </button>
        <div className="relative ml-auto w-44">
          <Search size={13} className="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-subtle" />
          <input
            value={query}
            autoFocus
            spellCheck={false}
            placeholder={t("history.search")}
            onChange={(e) => setQuery(e.target.value)}
            className="h-8 w-full rounded-lg border border-line bg-sunken pr-2.5 pl-8 text-[12.5px] text-fg outline-none placeholder:text-subtle focus:border-accent/40"
          />
        </div>
      </div>

      <div className="flex-1 space-y-2 overflow-y-auto px-4 pt-3 pb-5">
        <h2 className="px-1 text-[15px] font-medium text-fg">{t("history.title")}</h2>
        {error && <p className="px-1 text-[12px] text-danger select-text">{error}</p>}
        {running && <p className="px-1 text-[12px] text-subtle">{t("history.busy")}</p>}
        {chats?.length === 0 && (
          <p className="rounded-xl border border-dashed border-line px-3 py-3 text-[12.5px] text-muted">
            {t("history.empty")}
          </p>
        )}
        {chats && chats.length > 0 && visible.length === 0 && (
          <p className="px-1 text-[12.5px] text-muted">{t("history.noMatch")}</p>
        )}
        {visible.length > 0 && (
          <ul className="divide-y divide-line overflow-hidden rounded-xl border border-line bg-raised">
            {visible.map((chat) => (
              <ChatRow
                key={chat.id}
                chat={chat}
                disabled={running}
                onOpen={() => open(chat.id)}
                onDelete={() => remove(chat.id)}
              />
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}

function ChatRow({
  chat,
  disabled,
  onOpen,
  onDelete,
}: {
  chat: ArchivedChat;
  disabled: boolean;
  onOpen: () => void;
  onDelete: () => void;
}) {
  const { t, i18n } = useTranslation();
  const [confirmDelete, setConfirmDelete] = useState(false);

  useEffect(() => {
    if (!confirmDelete) return;
    const timer = setTimeout(() => setConfirmDelete(false), CONFIRM_DELETE_MS);
    return () => clearTimeout(timer);
  }, [confirmDelete]);

  return (
    <li className="group flex items-center transition-colors hover:bg-sunken">
      <button
        type="button"
        disabled={disabled}
        onClick={onOpen}
        className="flex min-w-0 flex-1 items-center gap-2.5 py-2.5 pl-3 text-left disabled:opacity-50"
      >
        <MessageSquare size={14} className="shrink-0 text-subtle" />
        <span className="min-w-0 flex-1 truncate text-[13px] text-fg">{chat.title || t("history.untitled")}</span>
        <span className="shrink-0 text-[11.5px] text-subtle">{formatWhen(chat.savedAt, i18n.language)}</span>
      </button>
      <button
        type="button"
        onClick={() => (confirmDelete ? onDelete() : setConfirmDelete(true))}
        title={t("history.delete")}
        aria-label={t("history.delete")}
        className={clsx(
          "mx-1.5 flex h-7 shrink-0 items-center gap-1 rounded-md px-1.5 text-[12px] transition-colors",
          confirmDelete
            ? "text-danger"
            : "text-subtle opacity-0 group-hover:opacity-100 hover:text-danger focus-visible:opacity-100",
        )}
      >
        {confirmDelete ? t("history.confirmDelete") : <Trash2 size={13} />}
      </button>
    </li>
  );
}

/** Today: the time. This year: day and month. Otherwise the full date. */
function formatWhen(iso: string, language: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return "";
  const now = new Date();
  if (date.toDateString() === now.toDateString()) {
    return new Intl.DateTimeFormat(language, { hour: "2-digit", minute: "2-digit" }).format(date);
  }
  const options: Intl.DateTimeFormatOptions =
    date.getFullYear() === now.getFullYear()
      ? { day: "numeric", month: "short" }
      : { day: "numeric", month: "short", year: "numeric" };
  return new Intl.DateTimeFormat(language, options).format(date);
}
