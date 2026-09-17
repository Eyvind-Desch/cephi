import clsx from "clsx";
import { useTranslation } from "react-i18next";
import type { ProfileInfo } from "../../lib/ipc";
import { useChat } from "../../store/chat";
import { useUi } from "../../store/ui";

const NO_PROFILES: ProfileInfo[] = [];

/** Segmented control for switching the model profile. */
export function ProfileSwitch() {
  const { t } = useTranslation();
  const profiles = useUi((s) => s.config?.profiles ?? NO_PROFILES);
  const profileId = useUi((s) => s.profileId);
  const setProfile = useUi((s) => s.setProfile);
  const running = useChat((s) => s.running);

  if (profiles.length < 2) return null;

  return (
    <div role="radiogroup" aria-label={t("profile.label")} className="flex rounded-lg bg-sunken p-0.5">
      {profiles.map((profile) => {
        const selected = profile.id === profileId;
        return (
          <button
            key={profile.id}
            type="button"
            role="radio"
            aria-checked={selected}
            disabled={running}
            title={profile.model}
            onClick={() => setProfile(profile.id)}
            className={clsx(
              "rounded-md px-2.5 py-1 text-[12px] font-medium transition-all disabled:cursor-not-allowed",
              selected ? "bg-raised text-fg shadow-sm" : "text-muted hover:text-fg",
            )}
          >
            {profile.label}
          </button>
        );
      })}
    </div>
  );
}
