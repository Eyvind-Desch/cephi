import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { api, type SecretGroup } from "../../lib/ipc";
import { useUi } from "../../store/ui";
import { SecretGroupRow } from "./SecretGroupRow";

const SECTIONS = ["ai", "integration", "custom"] as const;

/** Settings → Keys: all API keys at a glance, grouped into sections of compact rows. */
export function KeysSection() {
  const { t } = useTranslation();
  const [groups, setGroups] = useState<SecretGroup[] | null>(null);
  const [openId, setOpenId] = useState<string | null>(null);
  const config = useUi((s) => s.config);

  const load = useCallback(() => void api.secretsList().then(setGroups), []);
  // Tools and providers define which keys exist (and labels follow the language): reload on config changes.
  useEffect(load, [load, config]);

  return (
    <div className="space-y-5">
      <p className="px-1 text-[12px] text-subtle">{t("keys.storageNote")}</p>
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
  );
}
