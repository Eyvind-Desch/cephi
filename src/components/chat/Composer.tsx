import { ArrowUp, Square } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { onWindowShown } from "../../lib/ipc";
import { useChat } from "../../store/chat";
import { IconButton } from "../ui/IconButton";

interface ComposerProps {
  /** Compact bar: single line, no border. */
  compact?: boolean;
}

const MAX_HEIGHT_PX = 160;

export function Composer({ compact = false }: ComposerProps) {
  const { t } = useTranslation();
  const [text, setText] = useState("");
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const running = useChat((s) => s.running);
  const send = useChat((s) => s.send);
  const cancel = useChat((s) => s.cancel);

  // Focus when the window is summoned via hotkey/tray, and on mount (mode switch).
  useEffect(() => {
    inputRef.current?.focus();
    const unlisten = onWindowShown(() => inputRef.current?.focus());
    return () => void unlisten.then((fn) => fn());
  }, []);

  // Auto-grow up to a maximum height.
  useEffect(() => {
    const el = inputRef.current;
    if (!el || compact) return;
    el.style.height = "auto";
    el.style.height = `${Math.min(el.scrollHeight, MAX_HEIGHT_PX)}px`;
  }, [text, compact]);

  const submit = () => {
    if (running || !text.trim()) return;
    void send(text);
    setText("");
  };

  return (
    <div
      className={
        compact
          ? "flex min-w-0 flex-1 items-center gap-2"
          : "flex items-end gap-2 rounded-2xl border border-line bg-raised p-2 pl-3.5 shadow-sm transition-colors focus-within:border-accent/50"
      }
    >
      <textarea
        ref={inputRef}
        value={text}
        rows={1}
        spellCheck={false}
        placeholder={running ? t("composer.placeholderRunning") : t("composer.placeholder")}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && !e.shiftKey && !e.nativeEvent.isComposing) {
            e.preventDefault();
            submit();
          }
        }}
        className={
          compact
            ? "h-7 min-w-0 flex-1 resize-none overflow-hidden bg-transparent text-[16px] leading-7 text-fg outline-none placeholder:text-subtle"
            : "max-h-40 min-h-7 flex-1 resize-none bg-transparent py-1 text-[14px] leading-6 text-fg outline-none placeholder:text-subtle"
        }
      />
      {running ? (
        <IconButton icon={Square} label={t("composer.stop")} onClick={cancel} className="text-danger" />
      ) : (
        <IconButton
          icon={ArrowUp}
          label={t("composer.send")}
          variant="accent"
          onClick={submit}
          disabled={!text.trim()}
        />
      )}
    </div>
  );
}
