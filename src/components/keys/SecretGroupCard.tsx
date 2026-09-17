import clsx from "clsx";
import {
  Check,
  ChevronDown,
  ExternalLink,
  KeyRound,
  Loader2,
  Music,
  PlugZap,
  Sparkles,
  Wrench,
  X,
  type LucideIcon,
} from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { api, type SecretGroup } from "../../lib/ipc";
import { SecretField } from "./SecretField";
import { guides } from "./guides";

/** Keyed by the `icon` name sent by the backend (`secrets/catalog.rs`). */
const icons: Record<string, LucideIcon> = {
  music: Music,
  wrench: Wrench,
  anthropic: Sparkles,
  gemini: Sparkles,
  openai: Sparkles,
  ai: Sparkles,
};

/** Brand colours for well-known services; others use the accent colour. */
const brand: Record<string, string> = {
  music: "#1DB954",
  anthropic: "#D97757",
  gemini: "#4285F4",
  openai: "#10A37F",
};

type TestState = { status: "idle" } | { status: "testing" } | { status: "ok" | "error"; message: string };

export function SecretGroupCard({ group, onChange }: { group: SecretGroup; onChange: () => void }) {
  const { t } = useTranslation();
  const complete = group.secrets.every((s) => s.isSet);
  const missing = group.secrets.filter((s) => !s.isSet).length;
  const [open, setOpen] = useState(!complete);
  const [test, setTest] = useState<TestState>({ status: "idle" });

  const Icon = icons[group.icon] ?? KeyRound;
  const color = brand[group.icon];
  const Guide = guides[group.id];

  const runTest = async () => {
    setTest({ status: "testing" });
    try {
      setTest({ status: "ok", message: await api.secretsTest(group.id) });
    } catch (error) {
      setTest({ status: "error", message: String(error) });
    }
  };

  return (
    <section className="overflow-hidden rounded-2xl border border-line bg-raised">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        aria-expanded={open}
        className="flex w-full items-center gap-3 px-3.5 py-3 text-left transition-colors hover:bg-sunken"
      >
        <span
          className="grid size-9 shrink-0 place-items-center rounded-xl"
          style={color ? { background: `${color}24`, color } : undefined}
        >
          <Icon size={18} className={clsx(!color && "text-accent")} />
        </span>
        <span className="min-w-0 flex-1">
          <span className="block text-[14px] font-semibold">{group.title}</span>
          <span className="block truncate text-[12px] text-subtle">{group.description}</span>
        </span>
        <span
          className={clsx(
            "flex shrink-0 items-center gap-1 rounded-full px-2 py-0.5 text-[11.5px] font-medium",
            complete ? "bg-success/15 text-success" : "bg-warning/15 text-warning",
          )}
        >
          {complete ? <Check size={12} /> : null}
          {complete ? t("keys.group.complete") : t("keys.group.missing", { count: missing })}
        </span>
        <ChevronDown size={16} className={clsx("shrink-0 text-subtle transition-transform", open && "rotate-180")} />
      </button>

      <AnimatePresence initial={false}>
        {open && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.16, ease: "easeOut" }}
          >
            <div className="space-y-3 border-t border-line px-3.5 pt-3 pb-3.5">
              {Guide && <Guide />}

              <div className="space-y-2.5">
                {group.secrets.map((secret) => (
                  <SecretField key={secret.name} secret={secret} onChange={onChange} />
                ))}
              </div>

              {(group.testable || group.docsUrl) && (
                <div className="flex flex-wrap items-center gap-2 pt-1">
                  {group.testable && (
                    <button
                      type="button"
                      disabled={!complete || test.status === "testing"}
                      onClick={() => void runTest()}
                      className="flex items-center gap-1.5 rounded-lg border border-line px-2.5 py-1.5 text-[12.5px] font-medium transition-colors hover:bg-sunken disabled:opacity-40"
                    >
                      {test.status === "testing" ? <Loader2 size={14} className="animate-spin" /> : <PlugZap size={14} />}
                      {t("keys.group.testConnection")}
                    </button>
                  )}
                  {group.docsUrl && (
                    <button
                      type="button"
                      onClick={() => void api.openExternal(group.docsUrl!)}
                      className="flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-[12.5px] font-medium text-muted transition-colors hover:bg-sunken hover:text-fg"
                    >
                      <ExternalLink size={14} /> {t("keys.group.dashboard")}
                    </button>
                  )}
                  {(test.status === "ok" || test.status === "error") && (
                    <span
                      className={clsx(
                        "flex items-center gap-1 text-[12.5px]",
                        test.status === "ok" ? "text-success" : "text-danger",
                      )}
                    >
                      {test.status === "ok" ? <Check size={14} /> : <X size={14} />}
                      {test.message}
                    </span>
                  )}
                </div>
              )}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </section>
  );
}
