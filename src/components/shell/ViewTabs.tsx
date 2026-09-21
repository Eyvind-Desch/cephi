import clsx from "clsx";
import { Globe, Languages, MessageSquare, type LucideIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useUi, type PanelView } from "../../store/ui";

const TABS: { view: PanelView; icon: LucideIcon; label: "tabs.chat" | "tabs.web" | "tabs.translate" }[] = [
  { view: "chat", icon: MessageSquare, label: "tabs.chat" },
  { view: "web", icon: Globe, label: "tabs.web" },
  { view: "translate", icon: Languages, label: "tabs.translate" },
];

/** Icon switch between the main views. Sub-views like the keys page select none. */
export function ViewTabs() {
  const { t } = useTranslation();
  const view = useUi((s) => s.view);
  const setView = useUi((s) => s.setView);

  return (
    <div role="tablist" className="flex items-center rounded-lg bg-sunken p-0.5">
      {TABS.map((tab) => {
        const selected = view === tab.view;
        const Icon = tab.icon;
        return (
          <button
            key={tab.view}
            type="button"
            role="tab"
            aria-selected={selected}
            aria-label={t(tab.label)}
            title={t(tab.label)}
            onClick={() => setView(tab.view)}
            className={clsx(
              "grid size-7 place-items-center rounded-md transition-colors",
              selected ? "bg-raised text-fg shadow-sm shadow-black/10" : "text-subtle hover:text-fg",
            )}
          >
            <Icon size={14} />
          </button>
        );
      })}
    </div>
  );
}
