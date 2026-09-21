import { AlertTriangle, X } from "lucide-react";
import { motion } from "motion/react";
import { useTranslation } from "react-i18next";
import { useVoice } from "../../store/voice";
import { ServiceStartButton } from "../ui/ServiceStartButton";

/** Why dictation didn't work — with a start button when the speech service is simply not running. */
export function VoiceError() {
  const { t } = useTranslation();
  const error = useVoice((s) => s.error);
  const serviceDown = useVoice((s) => s.serviceDown);
  const dismiss = useVoice((s) => s.dismissError);
  if (!error) return null;

  return (
    <motion.div
      initial={{ opacity: 0, y: 4 }}
      animate={{ opacity: 1, y: 0 }}
      className="mb-2 space-y-2 rounded-xl border border-line bg-raised px-3 py-2"
    >
      <div className="flex items-start gap-2">
        <AlertTriangle size={13} className="mt-0.5 shrink-0 text-warning" />
        <p className="min-w-0 flex-1 text-[12.5px] leading-snug text-muted select-text">{error}</p>
        <button
          type="button"
          onClick={dismiss}
          title={t("selection.dismiss")}
          className="grid size-6 shrink-0 place-items-center rounded-md text-subtle hover:bg-sunken hover:text-fg"
        >
          <X size={13} />
        </button>
      </div>
      {serviceDown && <ServiceStartButton service="voice" onStarted={dismiss} />}
    </motion.div>
  );
}
