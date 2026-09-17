import { AlertCircle, CircleSlash } from "lucide-react";
import { motion } from "motion/react";
import type { Message } from "../../store/chatModel";
import { Markdown } from "./Markdown";
import { ToolCard } from "./ToolCard";

export function MessageView({ message }: { message: Message }) {
  if (message.role === "user") {
    return (
      <motion.div initial={{ opacity: 0, y: 6 }} animate={{ opacity: 1, y: 0 }} className="flex justify-end">
        <div className="max-w-[85%] rounded-2xl rounded-br-md bg-accent px-3.5 py-2 leading-relaxed whitespace-pre-wrap text-accent-fg select-text">
          {message.text}
        </div>
      </motion.div>
    );
  }

  const isEmpty = message.parts.length === 0;

  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="space-y-2.5">
      {message.parts.map((part, index) =>
        part.kind === "text" ? (
          <Markdown key={index} text={part.text} />
        ) : (
          <ToolCard key={part.callId} part={part} />
        ),
      )}

      {isEmpty && message.status === "streaming" && <TypingIndicator />}

      {message.status === "cancelled" && (
        <p className="flex items-center gap-1.5 text-[12px] text-subtle">
          <CircleSlash size={13} /> Abgebrochen
        </p>
      )}

      {message.status === "error" && (
        <div className="flex gap-2 rounded-xl border border-danger/30 bg-danger/10 px-3 py-2 text-[13px] text-danger select-text">
          <AlertCircle size={15} className="mt-0.5 shrink-0" />
          <span>{message.error}</span>
        </div>
      )}
    </motion.div>
  );
}

function TypingIndicator() {
  return (
    <div className="flex h-6 items-center gap-1" aria-label="Orbit denkt nach">
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
