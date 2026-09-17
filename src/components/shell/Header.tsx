import clsx from "clsx";
import { FolderCog, KeyRound, Minimize2, MoreHorizontal, Pin, RefreshCw, SquarePen } from "lucide-react";
import { api } from "../../lib/ipc";
import { useChat } from "../../store/chat";
import { useUi } from "../../store/ui";
import { IconButton } from "../ui/IconButton";
import { Menu, type MenuEntry } from "../ui/Menu";
import { OrbitMark } from "../ui/OrbitMark";
import { ProfileSwitch } from "./ProfileSwitch";

export function Header() {
  const running = useChat((s) => s.running);
  const reset = useChat((s) => s.reset);
  const pinned = useUi((s) => s.pinned);
  const view = useUi((s) => s.view);
  const setView = useUi((s) => s.setView);
  const togglePinned = useUi((s) => s.togglePinned);
  const setMode = useUi((s) => s.setMode);
  const applyConfig = useUi((s) => s.applyConfig);

  const entries: MenuEntry[] = [
    {
      label: "Neuer Chat",
      icon: SquarePen,
      shortcut: "Strg+N",
      onSelect: () => {
        setView("chat");
        void reset();
      },
    },
    { label: "Anpinnen", icon: Pin, checked: pinned, onSelect: togglePinned },
    { kind: "separator" },
    {
      label: "Schlüssel & API-Keys",
      icon: KeyRound,
      disabled: view === "keys",
      onSelect: () => setView("keys"),
    },
    { label: "Konfigurationsordner", icon: FolderCog, onSelect: () => void api.configOpenFolder() },
    { label: "Konfiguration neu laden", icon: RefreshCw, onSelect: () => void api.configReload().then(applyConfig) },
  ];

  return (
    <header className="flex items-center gap-1.5 border-b border-line py-2 pr-2 pl-3">
      <OrbitMark size={22} active={running} />
      <span className="mr-1.5 text-[14px] font-semibold tracking-tight">Orbit</span>
      {pinned && (
        <span title="Angepinnt: bleibt offen" className="text-accent">
          <Pin size={13} className="fill-current" />
        </span>
      )}
      <div className="flex-1" />
      <ProfileSwitch />
      <Menu
        entries={entries}
        trigger={({ open, toggle, id }) => (
          <IconButton
            icon={MoreHorizontal}
            label="Menü"
            aria-haspopup="menu"
            aria-expanded={open}
            aria-controls={id}
            onClick={toggle}
            className={clsx(open && "bg-sunken text-fg")}
          />
        )}
      />
      <IconButton icon={Minimize2} label="Verkleinern (Strg+E)" onClick={() => setMode("compact")} />
    </header>
  );
}
