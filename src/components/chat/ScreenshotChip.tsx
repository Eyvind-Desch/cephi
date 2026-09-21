import { AlertTriangle, Sparkles, X } from "lucide-react";
import { motion } from "motion/react";
import { useTranslation } from "react-i18next";
import { useChat } from "../../store/chat";
import { useUi } from "../../store/ui";

/** The screenshot waiting for a question: preview, quick action and a hint if the model can't see. */
export function ScreenshotChip() {
  const { t } = useTranslation();
  const screenshot = useUi((s) => s.screenshot);
  const setScreenshot = useUi((s) => s.setScreenshot);
  const profile = useUi((s) => s.config?.profiles.find((p) => p.id === s.profileId) ?? null);
  const send = useChat((s) => s.send);
  const running = useChat((s) => s.running);
  if (!screenshot) return null;

  const blind = profile != null && !profile.vision;

  return (
    <motion.div
      initial={{ opacity: 0, y: 4 }}
      animate={{ opacity: 1, y: 0 }}
      className="mb-2 rounded-xl border border-line bg-raised p-2"
    >
      <div className="flex items-start gap-2.5">
        <img
          src={`data:image/png;base64,${screenshot.data}`}
          alt={t("screenshot.alt")}
          className="h-14 w-24 shrink-0 rounded-lg border border-line object-cover object-top"
        />
        <div className="min-w-0 flex-1 space-y-1.5">
          <p className="text-[12.5px] text-muted">{t("screenshot.ready")}</p>
          {blind ? (
            <p className="flex items-start gap-1.5 text-[11.5px] leading-snug text-warning">
              <AlertTriangle size={12} className="mt-0.5 shrink-0" />
              <span>{t("screenshot.noVision", { model: profile?.label })}</span>
            </p>
          ) : (
            <button
              type="button"
              disabled={running}
              onClick={() => void send(t("screenshot.describePrompt"))}
              className="flex items-center gap-1.5 rounded-lg border border-line px-2.5 py-1 text-[12.5px] text-muted select-none hover:bg-sunken hover:text-fg disabled:opacity-40"
            >
              <Sparkles size={13} /> {t("screenshot.describe")}
            </button>
          )}
        </div>
        <button
          type="button"
          onClick={() => setScreenshot(null)}
          title={t("selection.dismiss")}
          className="grid size-6 shrink-0 place-items-center rounded-md text-subtle hover:bg-sunken hover:text-fg"
        >
          <X size={13} />
        </button>
      </div>
    </motion.div>
  );
}
