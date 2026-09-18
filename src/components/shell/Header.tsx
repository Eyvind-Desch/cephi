import clsx from "clsx";
import { FolderCog, KeyRound, Minimize2, MoreHorizontal, Pin, Power, RefreshCw, SquarePen } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { api, type AutostartState } from "../../lib/ipc";
import { useChat } from "../../store/chat";
import { useUi } from "../../store/ui";
import { IconButton } from "../ui/IconButton";
import { Menu, type MenuEntry } from "../ui/Menu";
import { CephiMark } from "../ui/CephiMark";
import { ProfileSwitch } from "./ProfileSwitch";
import { ViewTabs } from "./ViewTabs";

export function Header() {
  const { t } = useTranslation();
  const running = useChat((s) => s.running);
  const reset = useChat((s) => s.reset);
  const pinned = useUi((s) => s.pinned);
  const view = useUi((s) => s.view);
  const setView = useUi((s) => s.setView);
  const togglePinned = useUi((s) => s.togglePinned);
  const collapse = useUi((s) => s.collapse);
  const applyConfig = useUi((s) => s.applyConfig);

  // The registry is the source of truth; read it once when the header appears.
  const [autostart, setAutostart] = useState<AutostartState | null>(null);
  useEffect(() => {
    void api.autostartState().then(setAutostart);
  }, []);

  const entries: MenuEntry[] = [
    {
      label: t("menu.newChat"),
      icon: SquarePen,
      shortcut: t("menu.newChatShortcut"),
      onSelect: () => {
        setView("chat");
        void reset();
      },
    },
    { label: t("menu.pin"), icon: Pin, checked: pinned, onSelect: togglePinned },
    {
      label: t("menu.autostart"),
      icon: Power,
      checked: autostart?.enabled ?? false,
      disabled: !autostart?.available,
      description: autostart && !autostart.available ? t("menu.autostartUnavailable") : undefined,
      onSelect: () => void api.autostartSet(!autostart?.enabled).then(setAutostart),
    },
    { kind: "separator" },
    {
      label: t("menu.keys"),
      icon: KeyRound,
      disabled: view === "keys",
      onSelect: () => setView("keys"),
    },
    { label: t("menu.configFolder"), icon: FolderCog, onSelect: () => void api.configOpenFolder() },
    {
      label: t("menu.reloadConfig"),
      icon: RefreshCw,
      onSelect: () => void api.configReload().then(applyConfig),
    },
  ];

  return (
    <header className="flex items-center gap-1.5 border-b border-line py-2 pr-2 pl-3">
      <CephiMark size={22} active={running} />
      <span className="mr-1.5 text-[13.5px] font-medium tracking-tight">Cephi</span>
      {pinned && (
        <span title={t("header.pinned")} className="text-accent">
          <Pin size={13} className="fill-current" />
        </span>
      )}
      <div className="flex-1" />
      {/* The model only matters in the chat. */}
      {view === "chat" && <ProfileSwitch />}
      <ViewTabs />
      <Menu
        entries={entries}
        trigger={({ open, toggle, id }) => (
          <IconButton
            icon={MoreHorizontal}
            label={t("header.menu")}
            aria-haspopup="menu"
            aria-expanded={open}
            aria-controls={id}
            onClick={toggle}
            className={clsx(open && "bg-sunken text-fg")}
          />
        )}
      />
      <IconButton icon={Minimize2} label={t("header.collapse")} onClick={collapse} />
    </header>
  );
}
