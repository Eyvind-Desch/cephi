import { Check, Copy } from "lucide-react";
import { useState, type ComponentType, type ReactNode } from "react";

/** Step-by-step help per service, keyed by group id (`secrets/catalog.rs`). */
export const guides: Record<string, ComponentType> = {
  spotify: SpotifyGuide,
};

function SpotifyGuide() {
  return (
    <Guide title="So bekommst du die Schlüssel (2 Minuten, Premium nötig)">
      <li>
        Im <b>Spotify Developer Dashboard</b> (Button unten) mit deinem Premium-Konto anmelden.
      </li>
      <li>
        <b>Create app</b>: Name „Orbit“, Redirect URI <CopyValue value="http://127.0.0.1:43821/callback" />, bei APIs{" "}
        <b>Web API</b> anhaken, speichern.
      </li>
      <li>
        In der App unter <b>Settings</b> Client ID und Client Secret kopieren und hier einfügen.
      </li>
    </Guide>
  );
}

function Guide({ title, children }: { title: string; children: ReactNode }) {
  return (
    <details className="group rounded-xl bg-sunken px-3 py-2 text-[12.5px] leading-relaxed">
      <summary className="cursor-pointer font-medium text-muted select-none marker:text-subtle hover:text-fg">
        {title}
      </summary>
      <ol className="mt-1.5 mb-0.5 list-decimal space-y-1.5 pl-5 text-muted [&_b]:font-semibold [&_b]:text-fg">
        {children}
      </ol>
    </details>
  );
}

/** Inline code with a copy button. */
function CopyValue({ value }: { value: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <span className="inline-flex items-center gap-1 rounded-md border border-line bg-raised py-px pr-0.5 pl-1.5 align-middle">
      <code className="font-mono text-[11.5px] text-fg">{value}</code>
      <button
        type="button"
        title="Kopieren"
        onClick={async () => {
          await navigator.clipboard.writeText(value);
          setCopied(true);
          setTimeout(() => setCopied(false), 1500);
        }}
        className="grid size-5 place-items-center rounded text-subtle hover:bg-sunken hover:text-fg"
      >
        {copied ? <Check size={12} className="text-success" /> : <Copy size={12} />}
      </button>
    </span>
  );
}
