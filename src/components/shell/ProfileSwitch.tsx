import clsx from "clsx";
import { ChevronDown, Cloud, Cpu } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { ProfileInfo } from "../../lib/ipc";
import { useChat } from "../../store/chat";
import { useUi } from "../../store/ui";
import { Menu, type MenuEntry } from "../ui/Menu";

const NO_PROFILES: ProfileInfo[] = [];

/** Model picker: local and cloud profiles; cloud profiles without a key link to the keys view. */
export function ProfileSwitch() {
  const { t } = useTranslation();
  const profiles = useUi((s) => s.config?.profiles ?? NO_PROFILES);
  const profileId = useUi((s) => s.profileId);
  const setProfile = useUi((s) => s.setProfile);
  const setView = useUi((s) => s.setView);
  const running = useChat((s) => s.running);

  const current = profiles.find((p) => p.id === profileId);
  if (!current || profiles.length < 2) return null;

  const toEntry = (profile: ProfileInfo): MenuEntry => ({
    id: profile.id,
    label: profile.label,
    description: profile.local ? profile.model : `${profile.providerLabel} · ${profile.model}`,
    icon: profile.local ? Cpu : Cloud,
    checked: profile.id === profileId,
    hint: profile.available ? undefined : t("profile.keyMissing"),
    disabled: running,
    onSelect: () => (profile.available ? setProfile(profile.id) : setView("keys")),
  });

  const local = profiles.filter((p) => p.local);
  const cloud = profiles.filter((p) => !p.local);
  const entries: MenuEntry[] = [
    ...(local.length > 0 ? [{ kind: "label", text: t("profile.local") } as const, ...local.map(toEntry)] : []),
    ...(cloud.length > 0 ? [{ kind: "label", text: t("profile.cloud") } as const, ...cloud.map(toEntry)] : []),
  ];

  const CurrentIcon = current.local ? Cpu : Cloud;

  return (
    <Menu
      width="w-72"
      entries={entries}
      trigger={({ open, toggle, id }) => (
        <button
          type="button"
          onClick={toggle}
          aria-haspopup="menu"
          aria-expanded={open}
          aria-controls={id}
          title={`${t("profile.label")}: ${current.providerLabel} · ${current.model}`}
          className={clsx(
            "flex h-8 items-center gap-1.5 rounded-lg bg-sunken pr-2 pl-2.5 text-[12.5px] font-medium transition-colors hover:text-fg",
            open ? "text-fg" : "text-muted",
          )}
        >
          <CurrentIcon size={13} className="shrink-0 text-accent" />
          <span className="max-w-32 truncate">{current.label}</span>
          <ChevronDown size={13} className={clsx("shrink-0 transition-transform", open && "rotate-180")} />
        </button>
      )}
    />
  );
}
