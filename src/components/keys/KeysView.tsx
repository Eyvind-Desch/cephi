import clsx from "clsx";
import { Check, ExternalLink, KeyRound, Trash2 } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { api, type SecretInfo } from "../../lib/ipc";
import { useUi } from "../../store/ui";

const SPOTIFY_DASHBOARD = "https://developer.spotify.com/dashboard";

/** Manage API keys stored in the Windows Credential Manager. Values are write-only. */
export function KeysView() {
  const [secrets, setSecrets] = useState<SecretInfo[] | null>(null);
  const config = useUi((s) => s.config);

  const load = useCallback(() => void api.secretsList().then(setSecrets), []);
  // Reload when tools change: they define which keys are needed.
  useEffect(load, [load, config]);

  const hasSpotify = secrets?.some((s) => s.name.startsWith("spotify_"));

  return (
    <div className="flex-1 space-y-4 overflow-y-auto px-4 py-4">
      <div className="space-y-1">
        <h2 className="flex items-center gap-2 text-[15px] font-semibold">
          <KeyRound size={16} className="text-accent" /> Schlüssel
        </h2>
        <p className="text-[12.5px] leading-relaxed text-muted">
          API-Keys liegen sicher in der Windows-Anmeldeinformationsverwaltung, nicht in Dateien. Orbit zeigt sie
          nie wieder an.
        </p>
      </div>

      {hasSpotify && <SpotifySetupHint />}

      {secrets?.length === 0 && (
        <p className="rounded-xl border border-line bg-raised px-3.5 py-3 text-[13px] text-muted">
          Keins deiner Tools braucht gerade einen Schlüssel.
        </p>
      )}

      <div className="space-y-2.5">
        {secrets?.map((secret) => <SecretRow key={secret.name} secret={secret} onChange={load} />)}
      </div>
    </div>
  );
}

function SecretRow({ secret, onChange }: { secret: SecretInfo; onChange: () => void }) {
  const [value, setValue] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const run = async (action: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    try {
      await action();
      setValue("");
      onChange();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="space-y-2 rounded-xl border border-line bg-raised px-3.5 py-3">
      <div className="flex items-center gap-2">
        <div className="min-w-0 flex-1">
          <div className="text-[13px] font-medium">{secret.label}</div>
          <div className="truncate text-[11.5px] text-subtle">
            <span className="font-mono">{secret.name}</span> · für {secret.usedBy.join(", ")}
          </div>
        </div>
        <span
          className={clsx(
            "flex shrink-0 items-center gap-1 rounded-md px-2 py-0.5 text-[11.5px] font-medium",
            secret.isSet ? "bg-success/15 text-success" : "bg-warning/15 text-warning",
          )}
        >
          {secret.isSet && <Check size={12} />}
          {secret.isSet ? "Gespeichert" : "Fehlt"}
        </span>
      </div>

      <form
        className="flex gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          if (value.trim()) void run(() => api.secretsSet(secret.name, value));
        }}
      >
        <input
          type="password"
          value={value}
          autoComplete="off"
          spellCheck={false}
          placeholder={secret.isSet ? "Neuen Wert eingeben zum Ersetzen" : "Wert einfügen"}
          onChange={(e) => setValue(e.target.value)}
          className="min-w-0 flex-1 rounded-lg border border-line bg-sunken px-2.5 py-1.5 font-mono text-[12.5px] outline-none placeholder:font-sans placeholder:text-subtle focus:border-accent/50"
        />
        <button
          type="submit"
          disabled={busy || !value.trim()}
          className="rounded-lg bg-accent px-3 text-[12.5px] font-medium text-accent-fg hover:brightness-110 disabled:opacity-40"
        >
          Speichern
        </button>
        {secret.isSet && (
          <button
            type="button"
            title="Löschen"
            disabled={busy}
            onClick={() => void run(() => api.secretsDelete(secret.name))}
            className="grid w-8 place-items-center rounded-lg text-muted hover:bg-sunken hover:text-danger"
          >
            <Trash2 size={14} />
          </button>
        )}
      </form>

      {error && <p className="text-[12px] text-danger select-text">{error}</p>}
    </div>
  );
}

function SpotifySetupHint() {
  return (
    <details className="group rounded-xl border border-line bg-raised px-3.5 py-3 text-[12.5px] leading-relaxed">
      <summary className="cursor-pointer font-medium marker:text-subtle">Spotify einrichten (2 Minuten)</summary>
      <ol className="mt-2 list-decimal space-y-1 pl-5 text-muted">
        <li>
          <button
            type="button"
            onClick={() => void api.openExternal(SPOTIFY_DASHBOARD)}
            className="inline-flex items-center gap-1 font-medium text-accent hover:underline"
          >
            Spotify Developer Dashboard <ExternalLink size={12} />
          </button>{" "}
          öffnen und mit deinem Premium-Konto anmelden.
        </li>
        <li>
          <b className="text-fg">Create app</b>: Name „Orbit“, Redirect URI{" "}
          <code className="rounded bg-sunken px-1 font-mono">http://127.0.0.1:43821/callback</code>, API{" "}
          <b className="text-fg">Web API</b> anhaken, speichern.
        </li>
        <li>
          In der App unter <b className="text-fg">Settings</b> die <b className="text-fg">Client ID</b> und das{" "}
          <b className="text-fg">Client secret</b> kopieren und unten einfügen.
        </li>
      </ol>
    </details>
  );
}
