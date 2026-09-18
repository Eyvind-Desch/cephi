import clsx from "clsx";
import { X } from "lucide-react";
import { useState, type KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";

const MODIFIER_CODES = ["Control", "Alt", "Shift", "Meta", "OS"];

/** `KeyT` → `T`, `Digit1` → `1`, `Space` stays: the names the global shortcut parser understands. */
function keyName(code: string): string {
  if (code.startsWith("Key")) return code.slice(3);
  if (code.startsWith("Digit")) return code.slice(5);
  return code;
}

/** `Ctrl+Alt+T` from a key press, or null while only modifiers are held (or none is). */
export function shortcutFrom(e: Pick<KeyboardEvent, "ctrlKey" | "altKey" | "shiftKey" | "metaKey" | "code">) {
  if (MODIFIER_CODES.some((m) => e.code.startsWith(m)) || !e.code) return null;
  // Shift alone would swallow normal typing.
  if (!e.ctrlKey && !e.altKey && !e.metaKey) return null;
  const parts = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Super"].filter(
    (p): p is string => Boolean(p),
  );
  return [...parts, keyName(e.code)].join("+");
}

/** Records a key combination: click, press the keys, done. Esc cancels. */
export function HotkeyField({
  value,
  onChange,
  clearable,
  label,
}: {
  value: string | null;
  onChange: (value: string | null) => void;
  clearable?: boolean;
  label: string;
}) {
  const { t } = useTranslation();
  const [recording, setRecording] = useState(false);

  const onKeyDown = (e: KeyboardEvent<HTMLButtonElement>) => {
    if (!recording) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.key === "Escape") {
      setRecording(false);
      return;
    }
    const shortcut = shortcutFrom(e);
    if (shortcut) {
      setRecording(false);
      if (shortcut !== value) onChange(shortcut);
    }
  };

  return (
    <div className="flex items-center gap-1">
      <button
        type="button"
        aria-label={label}
        onClick={() => setRecording(true)}
        onBlur={() => setRecording(false)}
        onKeyDown={onKeyDown}
        className={clsx(
          "h-8 min-w-32 rounded-lg border px-2.5 font-mono text-[12px] transition-colors outline-none",
          recording ? "border-accent/60 bg-accent-soft text-accent" : "border-line bg-sunken text-fg hover:border-accent/30",
        )}
      >
        {recording ? t("settings.hotkey.press") : (value ?? <span className="font-sans text-subtle">{t("settings.hotkey.off")}</span>)}
      </button>
      {clearable && value && !recording && (
        <button
          type="button"
          onClick={() => onChange(null)}
          title={t("settings.hotkey.clear")}
          aria-label={t("settings.hotkey.clear")}
          className="grid size-7 place-items-center rounded-md text-subtle hover:bg-sunken hover:text-fg"
        >
          <X size={13} />
        </button>
      )}
    </div>
  );
}
