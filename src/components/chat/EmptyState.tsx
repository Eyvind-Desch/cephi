import { motion } from "motion/react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { useChat } from "../../store/chat";
import { useUi } from "../../store/ui";
import { CephiMark } from "../ui/CephiMark";

const SUGGESTIONS = ["spotify", "calculator", "weather", "memory"] as const;

export function EmptyState() {
  const { t } = useTranslation();
  const send = useChat((s) => s.send);
  const hotkey = useUi((s) => s.config?.hotkey);
  const toolCount = useUi((s) => s.config?.tools.length ?? 0);

  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-6 px-6 text-center">
      <motion.div initial={{ scale: 0.9, opacity: 0 }} animate={{ scale: 1, opacity: 1 }}>
        <CephiMark size={56} />
      </motion.div>
      <div className="space-y-1.5">
        <h1 className="text-[17px] font-medium">{t("emptyState.title")}</h1>
        <p className="text-[13px] text-muted">
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
              className="rounded-xl border border-line bg-raised px-3.5 py-2.5 text-left text-[13px] text-muted transition-colors hover:border-accent/40 hover:text-fg"
            >
              {suggestion}
            </motion.button>
          );
        })}
      </div>
    </div>
  );
}

export function Kbd({ children }: { children: ReactNode }) {
  return (
    <kbd className="rounded-md border border-line bg-raised px-1.5 py-0.5 font-sans text-[11px] text-muted">
      {children}
    </kbd>
  );
}
