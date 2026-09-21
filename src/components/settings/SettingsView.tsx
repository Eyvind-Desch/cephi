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
      <div className="space-y-2 px-3 pt-2">
        <button
          type="button"
          onClick={() => setView("chat")}
          title={t("keys.backTitle")}
          className="-ml-1 flex items-center gap-1.5 rounded-md px-1.5 py-1 text-body text-muted transition-colors hover:bg-sunken hover:text-fg"
        >
          <ArrowLeft size={14} /> {t("keys.back")}
        </button>
        {/* One row across the whole width: calmer than a second cluster of tabs next to the header. */}
        <div role="tablist" className="grid grid-cols-3 gap-0.5 rounded-lg bg-sunken p-0.5">
          {TABS.map((id) => (
            <button
              key={id}
              type="button"
              role="tab"
              aria-selected={tab === id}
              onClick={() => openSettings(id)}
              className={clsx(
                "rounded-md py-1 text-body transition-colors",
                tab === id ? "bg-raised text-fg shadow-sm" : "text-muted hover:text-fg",
              )}
            >
              {t(`settings.tabs.${id}`)}
            </button>
          ))}
        </div>
      </div>

      <div className="selectable flex-1 overflow-y-auto px-3 pt-3 pb-5">
        {tab === "general" && <GeneralSection />}
        {tab === "keys" && <KeysSection />}
        {tab === "mcp" && <McpSection />}
      </div>
    </div>
  );
}
