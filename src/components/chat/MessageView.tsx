import { AlertCircle, Check, CircleSlash, Copy, Info, Sparkles } from "lucide-react";
import { motion } from "motion/react";
import { Fragment, memo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useChat } from "../../store/chat";
import { citationSources, hasUnansweredResults, type AssistantMessage, type Message } from "../../store/chatModel";
import { Markdown } from "./Markdown";
import { ToolCard } from "./ToolCard";
import { TranslationCard } from "../translate/TranslationCard";
import { WebResults } from "./WebResults";

/**
 * Memoized: while a reply streams only the message being updated gets a new object,
 * so all other messages skip re-rendering.
 */
export const MessageView = memo(function MessageView({ message, isLast }: { message: Message; isLast: boolean }) {
  const { t } = useTranslation();
  const running = useChat((s) => s.running);
  const summarize = useChat((s) => s.summarize);

  if (message.role === "user") {
    return (
      <motion.div initial={{ opacity: 0, y: 6 }} animate={{ opacity: 1, y: 0 }} className="flex justify-end">
        <div className="max-w-[85%] space-y-2 rounded-2xl rounded-br-md border border-line bg-raised px-3.5 py-2 leading-relaxed whitespace-pre-wrap text-fg">
          {message.image && (
            <img
              src={`data:image/png;base64,${message.image}`}
              alt={t("screenshot.alt")}
              className="max-h-40 w-full rounded-lg border border-line object-cover object-top"
            />
          )}
          {message.text}
        </div>
      </motion.div>
    );
  }

  const lastPart = message.parts.at(-1);
  // Nothing yet, or a tool finished and the model is about to answer.
  const waitingForModel =
    message.status === "streaming" && (!lastPart || (lastPart.kind === "tool" && lastPart.status === "ok"));
  const text = plainText(message);
  const sources = citationSources(message.parts);

  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="group space-y-2.5">
      {message.parts.map((part, index) =>
        part.kind === "text" ? (
          <Markdown key={index} text={part.text} sources={sources[index]} />
        ) : (
          <Fragment key={part.callId}>
            <ToolCard part={part} />
            {part.display?.kind === "webResults" && <WebResults results={part.display.results} />}
            {part.display?.kind === "translation" && <TranslationCard translation={part.display} />}
          </Fragment>
        ),
      )}

      {waitingForModel && <TypingIndicator />}

      {message.notice && (
        <p className="flex items-start gap-1.5 text-hint leading-snug text-subtle">
          <Info size={13} className="mt-px shrink-0" /> <span>{message.notice}</span>
        </p>
      )}

      {isLast && !running && hasUnansweredResults(message) && (
        <button
          type="button"
          onClick={() => void summarize(message.id)}
          className="flex items-center gap-1.5 rounded-lg border border-line bg-raised px-2.5 py-1.5 text-body text-muted select-none hover:bg-sunken hover:text-fg"
        >
          <Sparkles size={13} /> {t("webResults.summarize")}
        </button>
      )}

      {message.status === "cancelled" && (
        <p className="flex items-center gap-1.5 text-hint text-subtle select-none">
          <CircleSlash size={13} /> {t("chat.cancelled")}
        </p>
      )}

      {message.status === "error" && (
        <div className="flex gap-2 rounded-xl border border-danger/30 bg-danger/10 px-3 py-2 text-body text-danger">
          <AlertCircle size={14} className="mt-0.5 shrink-0" />
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
      className="-mt-1 flex items-center gap-1.5 rounded-md px-1.5 py-1 text-hint text-subtle opacity-0 transition-opacity select-none group-hover:opacity-100 hover:text-fg focus-visible:opacity-100"
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
