import { AlertCircle, Check, CircleSlash, Copy } from "lucide-react";
import { motion } from "motion/react";
import { memo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { AssistantMessage, Message } from "../../store/chatModel";
import { Markdown } from "./Markdown";
import { ToolCard } from "./ToolCard";

/**
 * Memoized: while a reply streams only the message being updated gets a new object,
 * so all other messages skip re-rendering.
 */
export const MessageView = memo(function MessageView({ message }: { message: Message }) {
  const { t } = useTranslation();

  if (message.role === "user") {
    return (
      <motion.div initial={{ opacity: 0, y: 6 }} animate={{ opacity: 1, y: 0 }} className="flex justify-end">
        <div className="max-w-[85%] rounded-2xl rounded-br-md bg-accent px-3.5 py-2 leading-relaxed whitespace-pre-wrap text-accent-fg">
          {message.text}
        </div>
      </motion.div>
    );
  }

  const isEmpty = message.parts.length === 0;
  const text = plainText(message);

  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="group space-y-2.5">
      {message.parts.map((part, index) =>
        part.kind === "text" ? <Markdown key={index} text={part.text} /> : <ToolCard key={part.callId} part={part} />,
      )}

      {isEmpty && message.status === "streaming" && <TypingIndicator />}

      {message.status === "cancelled" && (
        <p className="flex items-center gap-1.5 text-[12px] text-subtle select-none">
          <CircleSlash size={13} /> {t("chat.cancelled")}
        </p>
      )}

      {message.status === "error" && (
        <div className="flex gap-2 rounded-xl border border-danger/30 bg-danger/10 px-3 py-2 text-[13px] text-danger">
          <AlertCircle size={15} className="mt-0.5 shrink-0" />
          <span>{message.error}</span>
        </div>
      )}

      {text && message.status !== "streaming" && <CopyButton text={text} />}
    </motion.div>
  );
});

function plainText(message: AssistantMessage) {
  return message.parts
    .flatMap((part) => (part.kind === "text" ? [part.text.trim()] : []))
    .filter(Boolean)
    .join("\n\n");
}

function CopyButton({ text }: { text: string }) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  return (
    <button
      type="button"
      onClick={async () => {
        await navigator.clipboard.writeText(text);
        setCopied(true);
        setTimeout(() => setCopied(false), 1500);
      }}
      className="-mt-1 flex items-center gap-1.5 rounded-md px-1.5 py-1 text-[12px] text-subtle opacity-0 transition-opacity select-none group-hover:opacity-100 hover:text-fg focus-visible:opacity-100"
    >
      {copied ? <Check size={13} className="text-success" /> : <Copy size={13} />}
      {copied ? t("common.copied") : t("common.copy")}
    </button>
  );
}

function TypingIndicator() {
  const { t } = useTranslation();
  return (
    <div className="flex h-6 items-center gap-1 select-none" aria-label={t("chat.thinking")}>
      {[0, 1, 2].map((i) => (
        <motion.span
          key={i}
          className="size-1.5 rounded-full bg-subtle"
          animate={{ opacity: [0.3, 1, 0.3], y: [0, -2, 0] }}
          transition={{ duration: 1, repeat: Infinity, delay: i * 0.15 }}
        />
      ))}
    </div>
  );
}
