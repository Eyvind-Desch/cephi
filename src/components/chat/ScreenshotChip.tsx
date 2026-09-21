import { AlertTriangle, Eye, Sparkles, X } from "lucide-react";
import { motion } from "motion/react";
import { useTranslation } from "react-i18next";
import type { ProfileInfo } from "../../lib/ipc";
import { useChat } from "../../store/chat";
import { useUi } from "../../store/ui";

const EMPTY_PROFILES: ProfileInfo[] = [];

/** The screenshot waiting for a question: preview, quick action and a hint if the model cannot see. */
export function ScreenshotChip() {
  const { t } = useTranslation();
  const screenshot = useUi((s) => s.screenshot);
  const setScreenshot = useUi((s) => s.setScreenshot);
  const profile = useUi((s) => s.config?.profiles.find((p) => p.id === s.profileId) ?? null);
  const profiles = useUi((s) => s.config?.profiles ?? EMPTY_PROFILES);
  const setProfile = useUi((s) => s.setProfile);
  const send = useChat((s) => s.send);
  const running = useChat((s) => s.running);
  if (!screenshot) return null;

  const blind = profile != null && !profile.vision;
  // Switching is one click instead of a hunt through the model picker.
  const seeing = profiles.find((p) => p.vision && p.available);

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
          <p className="text-body text-muted">{t("screenshot.ready")}</p>
          {blind ? (
            <div className="space-y-1.5">
              <p className="flex items-start gap-1.5 text-hint leading-snug text-warning">
                <AlertTriangle size={13} className="mt-0.5 shrink-0" />
                <span>{t("screenshot.noVision", { model: profile?.label })}</span>
              </p>
              {seeing && (
                <button
                  type="button"
                  onClick={() => setProfile(seeing.id)}
                  className="flex items-center gap-1.5 rounded-lg border border-line px-2.5 py-1 text-body text-muted select-none hover:bg-sunken hover:text-fg"
                >
                  <Eye size={13} /> {t("screenshot.switchModel", { model: seeing.label })}
                </button>
              )}
            </div>
          ) : (
            <button
              type="button"
              disabled={running}
              onClick={() => void send(t("screenshot.describePrompt"))}
              className="flex items-center gap-1.5 rounded-lg border border-line px-2.5 py-1 text-body text-muted select-none hover:bg-sunken hover:text-fg disabled:opacity-40"
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
