import clsx from "clsx";
import { ChevronDown } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { ProfileInfo } from "../../lib/ipc";
import { useChat } from "../../store/chat";
import { useUi } from "../../store/ui";
import { BrandIcon } from "../ui/BrandIcon";
import { Menu, type MenuEntry, type MenuIcon } from "../ui/Menu";

const NO_PROFILES: ProfileInfo[] = [];

const brandIcons = new Map<string, MenuIcon>();
/** Stable component per logo, so React doesn't remount icons on every render. */
function brandIcon(name: string): MenuIcon {
  let icon = brandIcons.get(name);
  if (!icon) {
    icon = (props) => <BrandIcon name={name} {...props} />;
    brandIcons.set(name, icon);
  }
  return icon;
}

/** Model picker: local and cloud profiles; cloud profiles without a key link to the keys view. */
export function ProfileSwitch() {
  const { t } = useTranslation();
  const profiles = useUi((s) => s.config?.profiles ?? NO_PROFILES);
  const profileId = useUi((s) => s.profileId);
  const setProfile = useUi((s) => s.setProfile);
  const openSettings = useUi((s) => s.openSettings);
  const running = useChat((s) => s.running);

  const current = profiles.find((p) => p.id === profileId);
  if (!current || profiles.length < 2) return null;

  const toEntry = (profile: ProfileInfo): MenuEntry => ({
    id: profile.id,
    label: profile.label,
    description: profile.local ? profile.model : `${profile.providerLabel} · ${profile.model}`,
    icon: brandIcon(profile.providerIcon),
    checked: profile.id === profileId,
    hint: profile.available ? undefined : t("profile.keyMissing"),
    disabled: running,
    onSelect: () => (profile.available ? setProfile(profile.id) : openSettings("keys")),
  });

  const local = profiles.filter((p) => p.local);
  const cloud = profiles.filter((p) => !p.local);
  const entries: MenuEntry[] = [
    ...(local.length > 0 ? [{ kind: "label", text: t("profile.local") } as const, ...local.map(toEntry)] : []),
    ...(cloud.length > 0 ? [{ kind: "label", text: t("profile.cloud") } as const, ...cloud.map(toEntry)] : []),
  ];

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
            "flex h-7 items-center gap-1.5 rounded-lg pr-1.5 pl-2 text-body transition-colors hover:bg-sunken hover:text-fg",
            open ? "bg-sunken text-fg" : "text-muted",
          )}
        >
          <BrandIcon name={current.providerIcon} size={13} />
          <span className="max-w-32 truncate">{current.label}</span>
          <ChevronDown size={13} className={clsx("shrink-0 text-subtle transition-transform", open && "rotate-180")} />
        </button>
      )}
    />
  );
}
