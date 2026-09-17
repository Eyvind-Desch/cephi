import clsx from "clsx";
import { Check, Pencil, Trash2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { api, type SecretInfo } from "../../lib/ipc";

/** How long "Entfernen?" waits for the confirming second click. */
const CONFIRM_DELETE_MS = 3000;

export function SecretField({ secret, onChange }: { secret: SecretInfo; onChange: () => void }) {
  const [editing, setEditing] = useState(!secret.isSet);
  const [value, setValue] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => setEditing(!secret.isSet), [secret.isSet]);

  useEffect(() => {
    if (!confirmDelete) return;
    const timer = setTimeout(() => setConfirmDelete(false), CONFIRM_DELETE_MS);
    return () => clearTimeout(timer);
  }, [confirmDelete]);

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
    <div className="space-y-1.5">
      <div className="flex items-baseline gap-2">
        <label htmlFor={secret.name} className="text-[12.5px] font-medium">
          {secret.label}
        </label>
        <span className="truncate font-mono text-[11px] text-subtle">{secret.name}</span>
      </div>

      {editing ? (
        <form
          className="flex gap-2"
          onSubmit={(e) => {
            e.preventDefault();
            if (value.trim()) void run(() => api.secretsSet(secret.name, value));
          }}
        >
          <input
            ref={inputRef}
            id={secret.name}
            type={secret.sensitive ? "password" : "text"}
            value={value}
            autoComplete="off"
            spellCheck={false}
            autoFocus={secret.isSet}
            placeholder={secret.placeholder}
            onChange={(e) => setValue(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape" && secret.isSet) {
                e.stopPropagation();
                setEditing(false);
              }
            }}
            className="min-w-0 flex-1 rounded-lg border border-line bg-sunken px-2.5 py-1.5 font-mono text-[12.5px] outline-none placeholder:font-sans placeholder:text-subtle focus:border-accent/60"
          />
          <button
            type="submit"
            disabled={busy || !value.trim()}
            className="rounded-lg bg-accent px-3 text-[12.5px] font-medium text-accent-fg transition hover:brightness-110 disabled:opacity-40"
          >
            Speichern
          </button>
          {secret.isSet && (
            <button
              type="button"
              onClick={() => setEditing(false)}
              className="rounded-lg px-2 text-[12.5px] text-muted hover:bg-sunken hover:text-fg"
            >
              Abbrechen
            </button>
          )}
        </form>
      ) : (
        <div className="flex items-center gap-2 rounded-lg border border-line bg-sunken py-1 pr-1 pl-2.5">
          <Check size={14} className="shrink-0 text-success" />
          <span className="flex-1 font-mono text-[12.5px] tracking-widest text-subtle select-none">••••••••••••</span>
          <button
            type="button"
            onClick={() => setEditing(true)}
            className="flex items-center gap-1 rounded-md px-2 py-1 text-[12px] text-muted hover:bg-raised hover:text-fg"
          >
            <Pencil size={12} /> Ändern
          </button>
          <button
            type="button"
            disabled={busy}
            onClick={() => (confirmDelete ? void run(() => api.secretsDelete(secret.name)) : setConfirmDelete(true))}
            className={clsx(
              "flex items-center gap-1 rounded-md px-2 py-1 text-[12px] transition-colors",
              confirmDelete ? "bg-danger/15 text-danger" : "text-muted hover:bg-raised hover:text-danger",
            )}
          >
            <Trash2 size={12} /> {confirmDelete ? "Wirklich entfernen?" : "Entfernen"}
          </button>
        </div>
      )}

      {error && <p className="text-[12px] text-danger">{error}</p>}
    </div>
  );
}
