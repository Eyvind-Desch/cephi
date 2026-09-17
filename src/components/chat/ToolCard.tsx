import clsx from "clsx";
import { Check, ChevronRight, KeyRound, Loader2, ShieldQuestion, Wrench, X, Zap } from "lucide-react";
import { motion } from "motion/react";
import { useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { formatDuration } from "../../i18n";
import { useChat } from "../../store/chat";
import type { ToolPart } from "../../store/chatModel";
import { useUi } from "../../store/ui";

const statusStyles = {
  running: { icon: Loader2, className: "text-accent", spin: true },
  awaiting: { icon: ShieldQuestion, className: "text-warning", spin: false },
  ok: { icon: Check, className: "text-success", spin: false },
  error: { icon: X, className: "text-danger", spin: false },
} as const;

export function ToolCard({ part }: { part: ToolPart }) {
  const { t } = useTranslation();
  const confirm = useChat((s) => s.confirm);
  const setView = useUi((s) => s.setView);
  const [open, setOpen] = useState(false);
  const status = statusStyles[part.status];
  const StatusIcon = status.icon;
  const awaiting = part.status === "awaiting";
  const failed = part.status === "error";
  const hasArgs = part.args != null && Object.keys(part.args as object).length > 0;

  return (
    <motion.div
      initial={{ opacity: 0, y: 4 }}
      animate={{ opacity: 1, y: 0 }}
      className={clsx(
        "overflow-hidden rounded-xl border bg-raised text-[13px]",
        awaiting ? "border-warning/50" : "border-line",
      )}
    >
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="flex w-full items-center gap-2.5 px-3 py-2 text-left hover:bg-sunken"
      >
        {part.fast ? (
          <span title={t("toolCard.fastPath")}>
            <Zap size={14} className="shrink-0 text-accent" />
          </span>
        ) : (
          <Wrench size={14} className="shrink-0 text-subtle" />
        )}
        <span className="min-w-0 flex-1 truncate font-medium">{part.title}</span>
        {part.durationMs != null && (
          <span className="shrink-0 text-[11.5px] text-subtle">{formatDuration(part.durationMs)}</span>
        )}
        <span title={t(`toolCard.status.${part.status}`)} className={clsx("shrink-0", status.className)}>
          <StatusIcon size={15} className={clsx(status.spin && "animate-spin")} />
        </span>
        <ChevronRight size={14} className={clsx("shrink-0 text-subtle transition-transform", open && "rotate-90")} />
      </button>

      {awaiting && (
        <div className="space-y-2.5 border-t border-line px-3 py-2.5">
          <p className="text-muted">{t("toolCard.confirmPrompt")}</p>
          {part.preview && <CodeText>{part.preview}</CodeText>}
          <div className="flex justify-end gap-2">
            <button
              type="button"
              onClick={() => confirm(part.callId, false)}
              className="rounded-lg px-3 py-1.5 font-medium text-muted hover:bg-sunken hover:text-fg"
            >
              {t("toolCard.deny")}
            </button>
            <button
              type="button"
              autoFocus
              onClick={() => confirm(part.callId, true)}
              className="rounded-lg bg-accent px-3 py-1.5 font-medium text-accent-fg hover:brightness-110"
            >
              {t("toolCard.run")}
            </button>
          </div>
        </div>
      )}

      {failed && part.output && (
        <div className="space-y-2 border-t border-line px-3 py-2.5">
          <p className="leading-relaxed text-danger select-text">{part.output}</p>
          {part.missingSecrets && part.missingSecrets.length > 0 && (
            <button
              type="button"
              onClick={() => setView("keys")}
              className="inline-flex items-center gap-1.5 rounded-lg bg-accent px-3 py-1.5 font-medium text-accent-fg hover:brightness-110"
            >
              <KeyRound size={14} /> {t("toolCard.enterKeys")}
            </button>
          )}
        </div>
      )}

      {open && (
        <div className="space-y-2 border-t border-line px-3 py-2.5">
          <Detail label={t("toolCard.details.tool")}>
            <span className="font-mono text-[12px]">{part.name}</span>
          </Detail>
          {hasArgs && (
            <Detail label={t("toolCard.details.arguments")}>
              <CodeText>{JSON.stringify(part.args, null, 2)}</CodeText>
            </Detail>
          )}
          {part.output && !failed && (
            <Detail label={t("toolCard.details.result")}>
              <CodeText>{part.output}</CodeText>
            </Detail>
          )}
        </div>
      )}
    </motion.div>
  );
}

function Detail({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="space-y-1">
      <div className="text-[11px] font-medium tracking-wide text-subtle uppercase">{label}</div>
      {children}
    </div>
  );
}

function CodeText({ children }: { children: string }) {
  return (
    <pre className="max-h-48 overflow-auto rounded-lg bg-sunken px-2.5 py-2 font-mono text-[12px] leading-relaxed whitespace-pre-wrap select-text">
      {children}
    </pre>
  );
}
