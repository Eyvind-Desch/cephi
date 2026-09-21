import { AlertTriangle } from "lucide-react";
import { useVoice } from "../../store/voice";
import { ServiceStartButton } from "../ui/ServiceStartButton";
import { Chip } from "./Chip";

/** Why dictation didn't work — with a start button when the speech service is simply not running. */
export function VoiceError() {
  const error = useVoice((s) => s.error);
  const serviceDown = useVoice((s) => s.serviceDown);
  const dismiss = useVoice((s) => s.dismissError);
  if (!error) return null;

  return (
    <Chip onDismiss={dismiss} footer={serviceDown && <ServiceStartButton service="voice" onStarted={dismiss} />}>
      <AlertTriangle size={13} className="mt-0.5 shrink-0 text-warning" />
      <p className="min-w-0 flex-1 text-body leading-snug text-muted select-text">{error}</p>
    </Chip>
  );
}
