import clsx from "clsx";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { api, type SecretInfo } from "../../lib/ipc";

/** How long "Really remove?" waits for the confirming second click. */
const CONFIRM_DELETE_MS = 3000;

export function SecretField({ secret, onChange }: { secret: SecretInfo; onChange: () => void }) {
  const { t } = useTranslation();
  const [editing, setEditing] = useState(!secret.isSet);
  const [value, setValue] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);

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

  const textButton = "text-[12px] text-subtle transition-colors hover:text-fg disabled:opacity-40";

  return (
    <div className="space-y-1">
      <label htmlFor={secret.name} title={secret.name} className="block text-[12px] text-muted">
        {secret.label}
      </label>

      {editing ? (
        <form
          className="relative"
          onSubmit={(e) => {
            e.preventDefault();
            if (value.trim()) void run(() => api.secretsSet(secret.name, value));
          }}
        >
          <input
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
            className="h-8 w-full rounded-lg border border-line bg-sunken pr-24 pl-2.5 font-mono text-[12px] text-fg outline-none transition-colors placeholder:font-sans placeholder:text-subtle focus:border-accent/40"
          />
          <div className="absolute inset-y-0 right-1.5 flex items-center gap-2">
            {secret.isSet && !value && (
              <button type="button" onClick={() => setEditing(false)} className={textButton}>
                {t("common.cancel")}
              </button>
            )}
            {value.trim() && (
              <button
                type="submit"
                disabled={busy}
                className="rounded-md px-2 py-0.5 text-[12px] font-medium text-accent transition-colors hover:bg-accent-soft disabled:opacity-40"
              >
                {t("common.save")}
              </button>
            )}
          </div>
        </form>
      ) : (
        <div className="flex h-8 items-center gap-3 rounded-lg bg-sunken pr-2.5 pl-2.5">
          <span className="flex-1 font-mono text-[12px] tracking-[0.2em] text-subtle select-none">••••••••••</span>
          <button type="button" onClick={() => setEditing(true)} className={textButton}>
            {t("keys.field.change")}
          </button>
          <button
            type="button"
            disabled={busy}
            onClick={() => (confirmDelete ? void run(() => api.secretsDelete(secret.name)) : setConfirmDelete(true))}
            className={clsx(textButton, confirmDelete && "text-danger hover:text-danger")}
          >
            {confirmDelete ? t("keys.field.confirmRemove") : t("keys.field.remove")}
          </button>
        </div>
      )}

      {error && <p className="text-[12px] text-danger">{error}</p>}
    </div>
  );
}
