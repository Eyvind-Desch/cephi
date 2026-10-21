import clsx from "clsx";
import { Globe, Languages, MessageSquare, StickyNote, type LucideIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { TAB_VIEWS, useUi, type TabView } from "../../store/ui";

const TABS: Record<TabView, { icon: LucideIcon; label: `tabs.${TabView}` }> = {
  chat: { icon: MessageSquare, label: "tabs.chat" },
  web: { icon: Globe, label: "tabs.web" },
  translate: { icon: Languages, label: "tabs.translate" },
  notes: { icon: StickyNote, label: "tabs.notes" },
};

/** Icon switch between the main views. Sub-views like the keys page select none. */
export function ViewTabs() {
  const { t } = useTranslation();
  const view = useUi((s) => s.view);
  const setView = useUi((s) => s.setView);

  return (
    <div role="tablist" className="flex items-center rounded-lg bg-sunken p-0.5">
      {TAB_VIEWS.map((tabView, index) => {
        const tab = TABS[tabView];
        const selected = view === tabView;
        const Icon = tab.icon;
        return (
          <button
            key={tabView}
            type="button"
            role="tab"
            aria-selected={selected}
            aria-label={t(tab.label)}
            title={`${t(tab.label)} (Ctrl+${index + 1})`}
            onClick={() => setView(tabView)}
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
