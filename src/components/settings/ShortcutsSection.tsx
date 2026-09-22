import { Plus, Trash2 } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { errorText } from "../../lib/errors";
import { api, type Shortcut, type ShortcutAction } from "../../lib/ipc";
import { useUi } from "../../store/ui";
import { Card, inputClass, Row, Select, textButtonClass } from "./controls";
import { HotkeyField } from "./HotkeyField";

/** Everything a shortcut can be bound to; `prompt` additionally needs a text. */
const ACTIONS: ShortcutAction[] = ["show", "newChat", "screenshot", "translate", "web", "history", "prompt"];

/** The keys that work inside Cephi. They are part of how the app is operated, not settings. */
const IN_APP = [
  { keys: "Enter", what: "send" },
  { keys: "Shift+Enter", what: "newLine" },
  { keys: "Ctrl+E", what: "toggleMode" },
  { keys: "Ctrl+N", what: "newChat" },
  { keys: "Esc", what: "back" },
  { keys: "Ctrl+L", what: "address" },
  { keys: "/", what: "commands" },
] as const;

/** Settings → Shortcuts: what every key does, and the ones you bind yourself. */
export function ShortcutsSection() {
  const { t } = useTranslation();
  const config = useUi((s) => s.config);
  const applyConfig = useUi((s) => s.applyConfig);
  const [error, setError] = useState<string | null>(null);

  if (!config) return null;
  const { shortcuts } = config.general;

  const save = (patch: Record<string, unknown>) => {
    setError(null);
    api
      .settingsUpdate(patch)
      .then(applyConfig)
      .catch((e: unknown) => setError(errorText(e)));
  };

  /** The whole list travels at once: an array in settings.json is replaced, not merged. */
  const write = (next: Shortcut[]) => save({ shortcuts: next });
  const replace = (index: number, shortcut: Shortcut) =>
    write(shortcuts.map((current, at) => (at === index ? shortcut : current)));

  return (
    <div className="space-y-5">
      <Card title={t("settings.shortcuts.global")}>
        <Row label={t("settings.general.hotkey")}>
          <HotkeyField
            label={t("settings.general.hotkey")}
            value={config.hotkey}
            onChange={(hotkey) => hotkey && save({ hotkey })}
          />
        </Row>
        <Row label={t("settings.general.selectionHotkey")} hint={t("settings.general.selectionHotkeyHint")}>
          <HotkeyField
            label={t("settings.general.selectionHotkey")}
            value={config.general.selectionHotkey || null}
            clearable
            // An empty value turns it off (a missing one would bring back the default).
            onChange={(key) => save({ selectionHotkey: key ?? "" })}
          />
        </Row>
      </Card>

      <section className="space-y-1.5">
        <div className="space-y-0.5 px-1">
          <h3 className="text-caption font-medium tracking-wider text-subtle uppercase">
            {t("settings.shortcuts.own")}
          </h3>
          <p className="text-hint leading-snug text-subtle">{t("settings.shortcuts.ownHint")}</p>
        </div>

        {shortcuts.length === 0 ? (
          <p className="rounded-xl border border-dashed border-line px-3 py-3 text-body text-muted">
            {t("settings.shortcuts.empty")}
          </p>
        ) : (
          <div className="divide-y divide-line overflow-hidden rounded-xl border border-line bg-raised">
            {shortcuts.map((shortcut, index) => (
              <ShortcutRow
                key={index}
                shortcut={shortcut}
                onChange={(next) => replace(index, next)}
                onRemove={() => write(shortcuts.filter((_, at) => at !== index))}
              />
            ))}
          </div>
        )}

        {error && <p className="px-1 text-hint leading-snug text-danger select-text">{error}</p>}

        <button
          type="button"
          onClick={() => write([...shortcuts, { key: "", action: "show" }])}
          className={`${textButtonClass} flex items-center gap-1.5`}
        >
          <Plus size={13} /> {t("settings.shortcuts.add")}
        </button>
      </section>

      <Card title={t("settings.shortcuts.inApp")}>
        {IN_APP.map((entry) => (
          <Row key={entry.keys} label={t(`settings.shortcuts.keys.${entry.what}`)}>
            <kbd className="rounded-md border border-line bg-sunken px-1.5 py-0.5 font-mono text-caption text-muted">
              {entry.keys}
            </kbd>
          </Row>
        ))}
      </Card>
    </div>
  );
}

function ShortcutRow({
  shortcut,
  onChange,
  onRemove,
}: {
  shortcut: Shortcut;
  onChange: (shortcut: Shortcut) => void;
  onRemove: () => void;
}) {
  const { t } = useTranslation();

  return (
    <div className="space-y-2 px-3 py-2.5">
      <div className="flex items-center gap-2">
        <HotkeyField
          label={t("settings.shortcuts.key")}
          value={shortcut.key || null}
          onChange={(key) => key && onChange({ ...shortcut, key })}
        />
        <div className="min-w-0 flex-1">
          <Select<ShortcutAction>
            label={t("settings.shortcuts.action")}
            value={shortcut.action}
            options={ACTIONS.map((action) => ({
              value: action,
              label: t(`settings.shortcuts.actions.${action}`),
            }))}
            onChange={(action) => onChange({ ...shortcut, action })}
          />
        </div>
        <button
          type="button"
          onClick={onRemove}
          title={t("settings.shortcuts.remove")}
          aria-label={t("settings.shortcuts.remove")}
          className="grid size-7 shrink-0 place-items-center rounded-md text-subtle hover:bg-sunken hover:text-danger"
        >
          <Trash2 size={13} />
        </button>
      </div>
      {shortcut.action === "prompt" && (
        <div className="space-y-1">
          <input
            value={shortcut.text ?? ""}
            spellCheck={false}
            placeholder={t("settings.shortcuts.textPlaceholder")}
            onChange={(e) => onChange({ ...shortcut, text: e.target.value })}
            className={inputClass}
          />
          <p className="text-hint leading-snug text-subtle">{t("settings.shortcuts.textHint")}</p>
        </div>
      )}
    </div>
  );
}
