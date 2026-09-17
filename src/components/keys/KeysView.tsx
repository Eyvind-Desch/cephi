import { ArrowLeft } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { api, type SecretGroup } from "../../lib/ipc";
import { useUi } from "../../store/ui";
import { SecretGroupRow } from "./SecretGroupRow";

const SECTIONS = ["ai", "integration", "custom"] as const;

/** All API keys at a glance, grouped into sections of compact rows. */
export function KeysView() {
  const { t } = useTranslation();
  const [groups, setGroups] = useState<SecretGroup[] | null>(null);
  const [openId, setOpenId] = useState<string | null>(null);
  const config = useUi((s) => s.config);
  const setView = useUi((s) => s.setView);

  const load = useCallback(() => void api.secretsList().then(setGroups), []);
  // Tools and providers define which keys exist (and labels follow the language): reload on config changes.
  useEffect(load, [load, config]);

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="px-2 pt-2">
        <button
          type="button"
          onClick={() => setView("chat")}
          title={t("keys.backTitle")}
          className="flex items-center gap-1.5 rounded-md px-2 py-1 text-[12.5px] text-muted transition-colors hover:bg-sunken hover:text-fg"
        >
          <ArrowLeft size={14} strokeWidth={1.75} /> {t("keys.back")}
        </button>
      </div>

      <div className="selectable flex-1 space-y-5 overflow-y-auto px-4 pt-3 pb-5">
        <div className="space-y-0.5 px-1">
          <h2 className="text-[15px] font-medium text-fg">{t("keys.title")}</h2>
          <p className="text-[12px] text-subtle">{t("keys.storageNote")}</p>
        </div>

        {groups?.length === 0 && <p className="px-1 text-[12.5px] text-muted">{t("keys.empty")}</p>}

        {SECTIONS.map((section) => {
          const items = groups?.filter((g) => g.category === section) ?? [];
          if (items.length === 0) return null;
          return (
            <section key={section} className="space-y-1.5">
              <h3 className="px-1 text-[10.5px] font-medium tracking-wider text-subtle uppercase">
                {t(`keys.sections.${section}`)}
              </h3>
              <div className="divide-y divide-line overflow-hidden rounded-xl border border-line bg-raised">
                {items.map((group) => (
                  <SecretGroupRow
                    key={group.id}
                    group={group}
                    open={openId === group.id}
                    onToggle={() => setOpenId((id) => (id === group.id ? null : group.id))}
                    onChange={load}
                  />
                ))}
              </div>
            </section>
          );
        })}
      </div>
    </div>
  );
}
