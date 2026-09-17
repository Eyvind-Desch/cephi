import { AlertTriangle, Maximize2 } from "lucide-react";
import { useChat } from "../../store/chat";
import { useUi } from "../../store/ui";
import { Composer } from "../chat/Composer";
import { IconButton } from "../ui/IconButton";
import { OrbitMark } from "../ui/OrbitMark";

/** Compact mode: a single input line, Spotlight-style. */
export function CompactBar() {
  const running = useChat((s) => s.running);
  const setMode = useUi((s) => s.setMode);
  const profile = useUi((s) => s.config?.profiles.find((p) => p.id === s.profileId));
  const problemCount = useUi((s) => s.config?.diagnostics.length ?? 0);

  return (
    <div className="flex h-full items-center gap-3 pr-3 pl-4">
      <OrbitMark size={26} active={running} />
      <Composer compact />
      {problemCount > 0 && (
        <IconButton
          icon={AlertTriangle}
          label={`${problemCount} Konfigurationsproblem(e) – Details im Panel`}
          className="text-warning"
          onClick={() => setMode("expanded")}
        />
      )}
      {profile && (
        <span title={profile.model} className="rounded-md bg-sunken px-2 py-1 text-[11.5px] font-medium text-muted">
          {profile.label}
        </span>
      )}
      <IconButton icon={Maximize2} label="Erweitern (Strg+E)" onClick={() => setMode("expanded")} />
    </div>
  );
}
