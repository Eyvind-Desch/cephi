import { ArrowUp, Square } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { matchCommands, parseCommand, type SlashCommand } from "../../lib/commands";
import { onWindowShown } from "../../lib/ipc";
import { useChat } from "../../store/chat";
import { useUi } from "../../store/ui";
import { IconButton } from "../ui/IconButton";
import { CommandMenu, type CommandItem } from "./CommandMenu";

interface ComposerProps {
  /** Compact bar: single line, no border. */
  compact?: boolean;
}

const MAX_HEIGHT_PX = 160;

export function Composer({ compact = false }: ComposerProps) {
  const { t } = useTranslation();
  const text = useChat((s) => s.draft);
  const setText = useChat((s) => s.setDraft);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const running = useChat((s) => s.running);
  const send = useChat((s) => s.send);
  const cancel = useChat((s) => s.cancel);
  const expand = useUi((s) => s.expand);
  const clearDraft = useCallback(() => setText(""), [setText]);
  const items = useCommandItems(text, clearDraft);
  const [active, setActive] = useState(0);

  // Focus when the window is summoned via hotkey/tray, and on mount (mode switch).
  // One frame later: the webview only accepts keyboard focus once the window is active.
  useEffect(() => {
    const focus = () => {
      const el = inputRef.current;
      if (!el) return;
      el.focus();
      // A draft carried over from the other mode continues at its end.
      el.setSelectionRange(el.value.length, el.value.length);
    };
    focus();
    const unlisten = onWindowShown(() => requestAnimationFrame(focus));
    return () => void unlisten.then((fn) => fn());
  }, []);

  // Auto-grow up to a maximum height.
  useEffect(() => {
    const el = inputRef.current;
    if (!el || compact) return;
    el.style.height = "auto";
    el.style.height = `${Math.min(el.scrollHeight, MAX_HEIGHT_PX)}px`;
  }, [text, compact]);

  // The command list needs room: typing "/" in the compact bar opens the panel.
  useEffect(() => {
    if (compact && text.startsWith("/")) expand();
  }, [compact, text, expand]);

  useEffect(() => setActive(0), [items?.length]);

  const submit = () => {
    if (running || !text.trim()) return;
    // Picked-up text travels with the question as context.
    const { selection, setSelection } = useUi.getState();
    const message = selection ? t("selection.withContext", { question: text.trim(), text: selection }) : text;
    setSelection(null);
    void send(message);
    setText("");
  };

  const onCommandKey = (e: React.KeyboardEvent) => {
    if (!items) return false;
    const move = (delta: number) => setActive((i) => (i + delta + items.length) % Math.max(items.length, 1));
    switch (e.key) {
      case "ArrowDown":
        move(1);
        break;
      case "ArrowUp":
        move(-1);
        break;
      case "Tab": {
        const complete = items[active]?.complete;
        if (complete) setText(complete);
        break;
      }
      case "Enter":
        if (e.shiftKey) return false;
        items[active]?.apply();
        break;
      case "Escape":
        // Leave the command, don't hide the window.
        e.stopPropagation();
        setText("");
        break;
      default:
        return false;
    }
    e.preventDefault();
    return true;
  };

  return (
    // Own stacking layer: the command list must cover the (animated) chat content above.
    <div className={compact ? "flex min-w-0 flex-1" : "relative z-20"}>
      {items && !compact && <CommandMenu items={items} active={active} onHover={setActive} />}
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
            if (onCommandKey(e)) return;
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
            disabled={!text.trim() || items !== null}
          />
        )}
      </div>
    </div>
  );
}

/**
 * Suggestions for a slash command being typed; `null` if the input is not a command.
 * `clear` empties the composer before a command runs.
 */
function useCommandItems(text: string, clear: () => void): CommandItem[] | null {
  const { t } = useTranslation();
  const profiles = useUi((s) => s.config?.profiles);

  return useMemo(() => {
    const parsed = parseCommand(text);
    if (!parsed) return null;

    const run = (command: SlashCommand, arg: string) => () => {
      clear();
      command.run(arg);
    };

    // Still typing the name: all matching commands.
    if (!parsed.hasArg) {
      return matchCommands(parsed.name).map((command) => ({
        key: command.name,
        title: `/${command.name}`,
        hint: command.arg ? t(`commands.arg.${command.arg}`) : undefined,
        description: t(command.description),
        complete: `/${command.name}${command.arg ? " " : ""}`,
        // The model needs a choice; everything else runs right away (the argument is optional).
        apply: command.arg === "profile" ? () => useChat.getState().setDraft(`/${command.name} `) : run(command, ""),
      }));
    }

    const command = matchCommands(parsed.name)[0];
    if (!command) return [];
    if (command.arg === "profile") {
      const wanted = parsed.arg.toLowerCase();
      return (profiles ?? [])
        .filter((p) => p.id.startsWith(wanted) || p.label.toLowerCase().startsWith(wanted))
        .map((profile) => ({
          key: profile.id,
          title: profile.label,
          hint: profile.model,
          description: profile.available ? undefined : t("profile.keyMissing"),
          complete: `/${command.name} ${profile.id}`,
          apply: run(command, profile.id),
        }));
    }
    return [
      {
        key: command.name,
        title: `/${command.name}`,
        hint: parsed.arg || (command.arg ? t(`commands.arg.${command.arg}`) : undefined),
        description: t(command.description),
        apply: run(command, parsed.arg),
      },
    ];
  }, [text, profiles, t, clear]);
}
