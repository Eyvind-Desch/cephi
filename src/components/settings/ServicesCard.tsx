import clsx from "clsx";
import { RefreshCw } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { api, type ServiceStatus } from "../../lib/ipc";
import { ServiceStartButton } from "../ui/ServiceStartButton";
import { Card, textButtonClass } from "./controls";

/** Settings → General: the local containers (web search, translator, dictation) and their state. */
export function ServicesCard() {
  const { t } = useTranslation();
  const [services, setServices] = useState<ServiceStatus[] | null>(null);
  const [checking, setChecking] = useState(false);

  const refresh = useCallback(() => {
    setChecking(true);
    api
      .servicesStatus()
      .then(setServices)
      .catch(() => setServices([]))
      .finally(() => setChecking(false));
  }, []);

  useEffect(refresh, [refresh]);

  return (
    <Card title={t("settings.services.title")}>
      {services?.map((service) => (
        <div key={service.id} className="flex items-center gap-3 px-3 py-2.5">
          <span
            className={clsx("size-2 shrink-0 rounded-full", service.running ? "bg-success" : "border border-subtle")}
          />
          <div className="min-w-0 flex-1">
            <div className="text-body text-fg">{service.title}</div>
            <div className="text-hint text-subtle">
              {t(service.running ? "settings.services.running" : "settings.services.stopped")}
            </div>
          </div>
          {!service.running && <ServiceStartButton service={service.id} onStarted={refresh} />}
        </div>
      ))}
      <div className="flex items-center justify-between gap-3 px-3 py-2">
        <p className="text-hint leading-snug text-subtle">{t("settings.services.hint")}</p>
        <button
          type="button"
          disabled={checking}
          onClick={refresh}
          className={clsx(textButtonClass, "flex shrink-0 items-center gap-1.5")}
        >
          <RefreshCw size={13} className={clsx(checking && "animate-spin")} /> {t("settings.services.refresh")}
        </button>
      </div>
    </Card>
  );
}
