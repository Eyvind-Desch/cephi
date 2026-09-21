import { FolderCog } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { errorText } from "../../lib/errors";
import { api, type AutostartState, type GeneralSettings } from "../../lib/ipc";
import { useUi } from "../../store/ui";
import { Card, Row, Select, textButtonClass, Toggle } from "./controls";
import { HotkeyField } from "./HotkeyField";
import { ServicesCard } from "./ServicesCard";

/** Settings → General: the everyday options; everything else stays in settings.json. */
export function GeneralSection() {
  const { t } = useTranslation();
  const config = useUi((s) => s.config);
  const applyConfig = useUi((s) => s.applyConfig);
  const [autostart, setAutostart] = useState<AutostartState | null>(null);
  // The setting that failed to save, with the reason.
  const [error, setError] = useState<{ key: string; message: string } | null>(null);

  // The registry is the source of truth for autostart.
  useEffect(() => {
    void api.autostartState().then(setAutostart);
  }, []);

  if (!config) return null;
  const general = config.general;

  const save = (key: string, patch: Record<string, unknown>) => {
    setError(null);
    api
      .settingsUpdate(patch)
      .then(applyConfig)
      .catch((e: unknown) => setError({ key, message: errorText(e) }));
  };
  const errorFor = (key: string) => (error?.key === key ? error.message : null);

  return (
    <div className="space-y-5">
      <Card>
        <Row label={t("settings.general.language")} error={errorFor("language")}>
          <Select<GeneralSettings["language"]>
            label={t("settings.general.language")}
            value={general.language}
            options={[
              { value: "system", label: t("settings.general.languageSystem") },
              { value: "de", label: "Deutsch" },
              { value: "en", label: "English" },
            ]}
            onChange={(language) => save("language", { language })}
          />
        </Row>
        <Row
          label={t("settings.general.defaultModel")}
          hint={t("settings.general.defaultModelHint")}
          error={errorFor("defaultProfile")}
        >
          <Select
            label={t("settings.general.defaultModel")}
            value={config.defaultProfile}
            options={config.profiles.map((p) => ({ value: p.id, label: p.label }))}
            onChange={(defaultProfile) => save("defaultProfile", { defaultProfile })}
          />
        </Row>
        <Row
          label={t("settings.general.autostart")}
          hint={autostart && !autostart.available ? t("menu.autostartUnavailable") : undefined}
        >
          <Toggle
            label={t("settings.general.autostart")}
            checked={autostart?.enabled ?? false}
            disabled={!autostart?.available}
            onChange={(enabled) => void api.autostartSet(enabled).then(setAutostart)}
          />
        </Row>
      </Card>

      <Card title={t("settings.general.hotkeys")}>
        <Row label={t("settings.general.hotkey")} hint={t("settings.general.hotkeyHint")} error={errorFor("hotkey")}>
          <HotkeyField
            label={t("settings.general.hotkey")}
            value={config.hotkey}
            onChange={(hotkey) => hotkey && save("hotkey", { hotkey })}
          />
        </Row>
        <Row
          label={t("settings.general.selectionHotkey")}
          hint={t("settings.general.selectionHotkeyHint")}
          error={errorFor("selectionHotkey")}
        >
          <HotkeyField
            label={t("settings.general.selectionHotkey")}
            value={general.selectionHotkey || null}
            clearable
            // An empty value turns it off (a missing one would bring back the default).
            onChange={(selectionHotkey) => save("selectionHotkey", { selectionHotkey: selectionHotkey ?? "" })}
          />
        </Row>
      </Card>

      <Card title={t("settings.general.behavior")}>
        <Row label={t("settings.general.hideOnBlur")} hint={t("settings.general.hideOnBlurHint")} error={errorFor("window")}>
          <Toggle
            label={t("settings.general.hideOnBlur")}
            checked={general.hideOnBlur}
            onChange={(hideOnBlur) => save("window", { window: { hideOnBlur } })}
          />
        </Row>
        <Row
          label={t("settings.general.summarize")}
          hint={t("settings.general.summarizeHint")}
          error={errorFor("search")}
        >
          <Toggle
            label={t("settings.general.summarize")}
            checked={general.summarizeSearch}
            onChange={(summarize) => save("search", { search: { summarize } })}
          />
        </Row>
        <Row
          label={t("settings.general.voiceSend")}
          hint={t("settings.general.voiceSendHint")}
          error={errorFor("voice")}
        >
          <Toggle
            label={t("settings.general.voiceSend")}
            checked={general.voiceSendDirectly}
            onChange={(sendDirectly) => save("voice", { voice: { sendDirectly } })}
          />
        </Row>
      </Card>

      <ServicesCard />

      <div className="flex items-center justify-between gap-3 px-1">
        <p className="text-[11.5px] leading-snug text-subtle">{t("settings.general.moreInFile")}</p>
        <button
          type="button"
          onClick={() => void api.configOpenFolder()}
          className={`${textButtonClass} flex shrink-0 items-center gap-1.5`}
        >
          <FolderCog size={13} /> {t("tools.openFolder")}
        </button>
      </div>
    </div>
  );
}
