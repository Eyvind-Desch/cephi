import { FolderCog, Minimize2, Pin, PinOff, SquarePen } from "lucide-react";
import { api } from "../../lib/ipc";
import { useChat } from "../../store/chat";
import { useUi } from "../../store/ui";
import { IconButton } from "../ui/IconButton";
import { OrbitMark } from "../ui/OrbitMark";
import { ProfileSwitch } from "./ProfileSwitch";

export function Header() {
  const running = useChat((s) => s.running);
  const reset = useChat((s) => s.reset);
  const hasMessages = useChat((s) => s.messages.length > 0);
  const pinned = useUi((s) => s.pinned);
  const togglePinned = useUi((s) => s.togglePinned);
  const setMode = useUi((s) => s.setMode);

  return (
    <header className="flex items-center gap-2 border-b border-line px-3 py-2.5">
      <OrbitMark size={22} active={running} />
      <span className="mr-1 text-[14px] font-semibold tracking-tight">Orbit</span>
      <ProfileSwitch />
      <div className="flex-1" />
      <IconButton icon={SquarePen} label="Neuer Chat (Strg+N)" onClick={() => void reset()} disabled={!hasMessages} />
      <IconButton icon={pinned ? PinOff : Pin} label={pinned ? "Lösen" : "Anpinnen"} active={pinned} onClick={togglePinned} />
      <IconButton icon={FolderCog} label="Konfiguration öffnen" onClick={() => void api.configOpenFolder()} />
      <IconButton icon={Minimize2} label="Verkleinern (Strg+E)" onClick={() => setMode("compact")} />
    </header>
  );
}
