import { ArrowLeft, ShieldCheck } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { api, type SecretGroup } from "../../lib/ipc";
import { useUi } from "../../store/ui";
import { SecretGroupCard } from "./SecretGroupCard";

/** All API keys at a glance, grouped by service. */
export function KeysView() {
  const { t } = useTranslation();
  const [groups, setGroups] = useState<SecretGroup[] | null>(null);
  const config = useUi((s) => s.config);
  const setView = useUi((s) => s.setView);

  const load = useCallback(() => void api.secretsList().then(setGroups), []);
  // Tools define which keys are needed (and labels follow the language), so reload on config changes.
  useEffect(load, [load, config]);

  const secrets = groups?.flatMap((g) => g.secrets) ?? [];
  const missing = secrets.filter((s) => !s.isSet).length;

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex items-center gap-2 px-3 pt-3 pb-1">
        <button
          type="button"
          onClick={() => setView("chat")}
          title={t("keys.backTitle")}
          className="flex items-center gap-1.5 rounded-lg py-1 pr-2.5 pl-1.5 text-[13px] font-medium text-muted transition-colors hover:bg-sunken hover:text-fg"
        >
          <ArrowLeft size={16} /> {t("keys.back")}
        </button>
        <div className="flex-1" />
        {groups && secrets.length > 0 && (
          <span className={missing === 0 ? "text-[12px] text-success" : "text-[12px] text-warning"}>
            {missing === 0 ? t("keys.allSet") : t("keys.missing", { count: missing })}
          </span>
        )}
      </div>

      <div className="selectable flex-1 space-y-3 overflow-y-auto px-4 pt-2 pb-4">
        <div className="space-y-1">
          <h2 className="text-[17px] font-semibold tracking-tight">{t("keys.title")}</h2>
          <p className="flex items-start gap-1.5 text-[12.5px] leading-relaxed text-muted">
            <ShieldCheck size={14} className="mt-0.5 shrink-0 text-success" />
            {t("keys.storageNote")}
          </p>
        </div>

        {groups?.length === 0 && (
          <p className="rounded-xl border border-line bg-raised px-3.5 py-3 text-[13px] text-muted">{t("keys.empty")}</p>
        )}

        {groups?.map((group) => <SecretGroupCard key={group.id} group={group} onChange={load} />)}
      </div>
    </div>
  );
}
