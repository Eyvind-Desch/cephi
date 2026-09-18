import { Loader2, Play } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { errorText } from "../../lib/errors";
import { api, type ServiceId } from "../../lib/ipc";

interface ServiceStartButtonProps {
  service: ServiceId;
  /** Called once the service answers. */
  onStarted?: () => void;
}

/**
 * Starts a local service (Docker container in WSL) right from the message that reported it missing.
 * The first start may pull an image, so the button stays busy for a while.
 */
export function ServiceStartButton({ service, onStarted }: ServiceStartButtonProps) {
  const { t } = useTranslation();
  const [starting, setStarting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const start = async () => {
    setStarting(true);
    setError(null);
    try {
      await api.serviceStart(service);
      onStarted?.();
    } catch (e) {
      setError(errorText(e));
    } finally {
      setStarting(false);
    }
  };

  return (
    <div className="space-y-1.5">
      <button
        type="button"
        disabled={starting}
        onClick={() => void start()}
        className="inline-flex items-center gap-1.5 rounded-lg bg-accent px-3 py-1.5 text-[12.5px] font-medium text-accent-fg select-none hover:brightness-110 disabled:opacity-60"
      >
        {starting ? <Loader2 size={14} className="animate-spin" /> : <Play size={14} />}
        {starting ? t("services.starting") : t("services.start")}
      </button>
      {starting && <p className="text-[11.5px] text-subtle">{t("services.startingHint")}</p>}
      {error && <p className="text-[12px] leading-snug text-danger">{error}</p>}
    </div>
  );
}
