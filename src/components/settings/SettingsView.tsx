import clsx from "clsx";
import { ArrowLeft } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useUi, type SettingsTab } from "../../store/ui";
import { KeysSection } from "../keys/KeysSection";
import { GeneralSection } from "./GeneralSection";
import { McpSection } from "./McpSection";

const TABS: SettingsTab[] = ["general", "keys", "mcp"];

/** Settings: general options, API keys and MCP servers, all without editing JSON. */
export function SettingsView() {
  const { t } = useTranslation();
  const tab = useUi((s) => s.settingsTab);
  const openSettings = useUi((s) => s.openSettings);
  const setView = useUi((s) => s.setView);

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex items-center gap-2 px-2 pt-2">
        <button
          type="button"
          onClick={() => setView("chat")}
          title={t("keys.backTitle")}
          className="flex items-center gap-1.5 rounded-md px-2 py-1 text-[12.5px] text-muted transition-colors hover:bg-sunken hover:text-fg"
        >
          <ArrowLeft size={14} /> {t("keys.back")}
        </button>
        <div role="tablist" className="ml-auto flex gap-0.5 rounded-lg bg-sunken p-0.5">
          {TABS.map((id) => (
            <button
              key={id}
              type="button"
              role="tab"
              aria-selected={tab === id}
              onClick={() => openSettings(id)}
              className={clsx(
                "rounded-md px-2.5 py-1 text-[12.5px] transition-colors",
                tab === id ? "bg-raised text-fg shadow-sm" : "text-muted hover:text-fg",
              )}
            >
              {t(`settings.tabs.${id}`)}
            </button>
          ))}
        </div>
      </div>

      <div className="selectable flex-1 overflow-y-auto px-4 pt-3 pb-5">
        {tab === "general" && <GeneralSection />}
        {tab === "keys" && <KeysSection />}
        {tab === "mcp" && <McpSection />}
      </div>
    </div>
  );
}
