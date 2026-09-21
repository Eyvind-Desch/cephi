import { History } from "lucide-react";
import { motion } from "motion/react";
import { useEffect, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { api, type ArchivedChat } from "../../lib/ipc";
import { useChat } from "../../store/chat";
import { useUi } from "../../store/ui";
import { CephiMark } from "../ui/CephiMark";

const SUGGESTIONS = ["spotify", "calculator", "weather", "memory"] as const;
/** Recent chats offered to continue on the empty screen. */
const RECENT_CHATS = 2;

export function EmptyState() {
  const { t } = useTranslation();
  const send = useChat((s) => s.send);
  const hotkey = useUi((s) => s.config?.hotkey);
  const toolCount = useUi((s) => s.config?.tools.length ?? 0);
  const setView = useUi((s) => s.setView);
  const openArchived = useChat((s) => s.openArchived);
  const [recent, setRecent] = useState<ArchivedChat[]>([]);

  useEffect(() => {
    api
      .chatArchiveList()
      .then((chats) => setRecent(chats.slice(0, RECENT_CHATS)))
      .catch(() => undefined);
  }, []);

  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-6 px-6 text-center">
      <motion.div initial={{ scale: 0.9, opacity: 0 }} animate={{ scale: 1, opacity: 1 }}>
        <CephiMark size={56} />
      </motion.div>
      <div className="space-y-1.5">
        <h1 className="text-hero font-medium">{t("emptyState.title")}</h1>
        <p className="text-body text-muted">
          {t("emptyState.toolsReady", { count: toolCount })}
          {hotkey && (
            <>
              {" · "}
              <Kbd>{hotkey}</Kbd> {t("emptyState.hotkeyHint")}
            </>
          )}
        </p>
      </div>
      <div className="flex w-full flex-col gap-2">
        {SUGGESTIONS.map((key, i) => {
          const suggestion = t(`emptyState.suggestions.${key}`);
          return (
            <motion.button
              key={key}
              type="button"
              initial={{ opacity: 0, y: 6 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.05 * i }}
              onClick={() => void send(suggestion)}
              className="rounded-xl border border-line bg-raised px-3.5 py-2.5 text-left text-body text-muted transition-colors hover:border-accent/40 hover:text-fg"
            >
              {suggestion}
            </motion.button>
          );
        })}
      </div>
      {recent.length > 0 && (
        <div className="w-full space-y-1 text-left">
          {recent.map((chat) => (
            <button
              key={chat.id}
              type="button"
              onClick={() => void openArchived(chat.id)}
              className="flex w-full items-center gap-2 rounded-lg px-2 py-1 text-body text-subtle transition-colors hover:bg-sunken hover:text-fg"
            >
              <History size={13} className="shrink-0" />
              <span className="truncate">{chat.title || t("history.untitled")}</span>
            </button>
          ))}
          <button
            type="button"
            onClick={() => setView("history")}
            className="px-2 text-hint text-subtle underline-offset-2 hover:text-fg hover:underline"
          >
            {t("history.showAll")} →
          </button>
        </div>
      )}
    </div>
  );
}

export function Kbd({ children }: { children: ReactNode }) {
  return (
    <kbd className="rounded-md border border-line bg-raised px-1.5 py-0.5 font-sans text-caption text-muted">
      {children}
    </kbd>
  );
}
