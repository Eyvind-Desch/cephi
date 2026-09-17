import clsx from "clsx";
import { AlertTriangle, CheckCircle2, Maximize2, XCircle } from "lucide-react";
import { motion } from "motion/react";
import { useEffect } from "react";
import { onWindowShown } from "../../lib/ipc";
import { useChat } from "../../store/chat";
import { useUi, type Flash } from "../../store/ui";
import { Composer } from "../chat/Composer";
import { IconButton } from "../ui/IconButton";
import { OrbitMark } from "../ui/OrbitMark";

/** Compact mode: a single input line, Spotlight-style. */
export function CompactBar() {
  const running = useChat((s) => s.running);
  const setMode = useUi((s) => s.setMode);
  const flash = useUi((s) => s.flash);
  const showFlash = useUi((s) => s.showFlash);
  const profile = useUi((s) => s.config?.profiles.find((p) => p.id === s.profileId));
  const problemCount = useUi((s) => s.config?.diagnostics.length ?? 0);

  // A fast-path result is only relevant until the next interaction.
  useEffect(() => {
    if (!flash) return;
    const dismiss = () => showFlash(null);
    const unlisten = onWindowShown(dismiss);
    window.addEventListener("keydown", dismiss);
    return () => {
      window.removeEventListener("keydown", dismiss);
      void unlisten.then((fn) => fn());
    };
  }, [flash, showFlash]);

  return (
    <div className="flex h-full items-center gap-3 pr-3 pl-4">
      <OrbitMark size={26} active={running} />
      {flash ? <FlashRow flash={flash} /> : <Composer compact />}
      {problemCount > 0 && (
        <IconButton
          icon={AlertTriangle}
          label={`${problemCount} Konfigurationsproblem(e) – Details im Panel`}
          className="text-warning"
          onClick={() => setMode("expanded")}
        />
      )}
      {profile && !flash && (
        <span title={profile.model} className="rounded-md bg-sunken px-2 py-1 text-[11.5px] font-medium text-muted">
          {profile.label}
        </span>
      )}
      <IconButton icon={Maximize2} label="Erweitern (Strg+E)" onClick={() => setMode("expanded")} />
    </div>
  );
}

function FlashRow({ flash }: { flash: Flash }) {
  const Icon = flash.ok ? CheckCircle2 : XCircle;
  return (
    <motion.div
      initial={{ opacity: 0, y: 4 }}
      animate={{ opacity: 1, y: 0 }}
      className="flex min-w-0 flex-1 items-center gap-2 text-[15px]"
      role="status"
    >
      <Icon size={18} className={clsx("shrink-0", flash.ok ? "text-success" : "text-danger")} />
      <span className={clsx("truncate", !flash.ok && "text-danger")} title={flash.text}>
        {flash.text}
      </span>
    </motion.div>
  );
}
