import clsx from "clsx";
import { AlertTriangle, ChevronRight } from "lucide-react";
import { useState } from "react";
import { api, type Diagnostic } from "../../lib/ipc";
import { useUi } from "../../store/ui";

const NO_DIAGNOSTICS: Diagnostic[] = [];

/** Shows configuration problems (invalid JSON, unknown fields, hotkey conflicts …). */
export function DiagnosticsBanner() {
  const diagnostics = useUi((s) => s.config?.diagnostics ?? NO_DIAGNOSTICS);
  const [open, setOpen] = useState(false);

  if (diagnostics.length === 0) return null;

  return (
    <div className="mx-3 mt-2 overflow-hidden rounded-xl border border-warning/40 bg-warning/10 text-[12.5px]">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="flex w-full items-center gap-2 px-3 py-2 text-left text-warning"
      >
        <AlertTriangle size={14} className="shrink-0" />
        <span className="flex-1 font-medium">
          {diagnostics.length === 1 ? "1 Konfigurationsproblem" : `${diagnostics.length} Konfigurationsprobleme`}
        </span>
        <ChevronRight size={14} className={clsx("transition-transform", open && "rotate-90")} />
      </button>
      {open && (
        <div className="space-y-2 border-t border-warning/30 px-3 py-2.5">
          {diagnostics.map((d, i) => (
            <div key={i} className="select-text">
              <div className="font-mono text-[11.5px] text-muted">{d.source}</div>
              <div className="text-fg">{d.message}</div>
            </div>
          ))}
          <button type="button" onClick={() => void api.configOpenFolder()} className="font-medium text-accent hover:underline">
            Konfigurationsordner öffnen
          </button>
        </div>
      )}
    </div>
  );
}
