import clsx from "clsx";
import { Loader2, Mic, Square } from "lucide-react";
import { motion } from "motion/react";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useVoice } from "../../store/voice";
import { IconButton } from "../ui/IconButton";

/** Dictation: click to record, click again to turn the recording into text. Escape cancels. */
export function VoiceButton() {
  const { t } = useTranslation();
  const phase = useVoice((s) => s.phase);
  const toggle = useVoice((s) => s.toggle);
  const cancel = useVoice((s) => s.cancel);

  // Escape ends the recording without sending anything; it must not also hide the window.
  useEffect(() => {
    if (phase !== "recording") return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.preventDefault();
      e.stopPropagation();
      cancel();
    };
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, [phase, cancel]);

  if (phase === "transcribing") {
    return <IconButton icon={Loader2} label={t("voice.transcribing")} disabled className="animate-spin" />;
  }

  if (phase === "recording") {
    return (
      <div className="relative">
        <motion.span
          aria-hidden
          animate={{ opacity: [0.35, 0.9, 0.35] }}
          transition={{ duration: 1.4, repeat: Infinity }}
          className="pointer-events-none absolute inset-0 rounded-lg bg-danger/20"
        />
        <IconButton
          icon={Square}
          label={t("voice.stop")}
          onClick={() => void toggle()}
          className={clsx("relative text-danger hover:text-danger")}
        />
      </div>
    );
  }

  return <IconButton icon={Mic} label={t("voice.start")} onClick={() => void toggle()} />;
}
