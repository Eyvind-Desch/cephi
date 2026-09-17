import clsx from "clsx";
import { Globe, MessageSquare, type LucideIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useUi, type PanelView } from "../../store/ui";

const TABS: { view: PanelView; icon: LucideIcon; label: "tabs.chat" | "tabs.web" }[] = [
  { view: "chat", icon: MessageSquare, label: "tabs.chat" },
  { view: "web", icon: Globe, label: "tabs.web" },
];

/** Segmented switch between chat and web tab. Sub-views like the keys page select neither. */
export function ViewTabs() {
  const { t } = useTranslation();
  const view = useUi((s) => s.view);
  const setView = useUi((s) => s.setView);

  return (
    <div role="tablist" className="flex items-center gap-0.5 rounded-lg bg-sunken p-0.5">
      {TABS.map((tab) => {
        const selected = view === tab.view;
        const Icon = tab.icon;
        return (
          <button
            key={tab.view}
            type="button"
            role="tab"
            aria-selected={selected}
            onClick={() => setView(tab.view)}
            className={clsx(
              "flex h-7 items-center gap-1.5 rounded-md px-2.5 text-[12.5px] transition-colors",
              selected ? "bg-raised text-fg shadow-sm shadow-black/10" : "text-muted hover:text-fg",
            )}
          >
            <Icon size={14} />
            {t(tab.label)}
          </button>
        );
      })}
    </div>
  );
}
