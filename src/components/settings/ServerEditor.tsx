import clsx from "clsx";
import { ClipboardPaste, Eye, EyeOff, Lock, Plus, X } from "lucide-react";
import { useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { errorText } from "../../lib/errors";
import { api, type McpServerConfig } from "../../lib/ipc";
import { useUi } from "../../store/ui";
import { inputClass, primaryButtonClass, textButtonClass, Toggle } from "./controls";
import {
  emptyForm,
  emptyRow,
  formFrom,
  missingSecrets,
  parseSnippet,
  SENSITIVE_KEY,
  toSaveRequest,
  type PairRow,
  type ServerForm,
} from "./mcpForm";

const NAME_PATTERN = /^[A-Za-z0-9_-]{1,40}$/;

/** Add or edit one MCP server. Keys typed here go to the Windows Credential Manager. */
export function ServerEditor({
  initial,
  onDone,
}: {
  initial: { name: string; server: McpServerConfig } | null;
  onDone: () => void;
}) {
  const { t } = useTranslation();
  const applyConfig = useUi((s) => s.applyConfig);
  const [form, setForm] = useState<ServerForm>(() => (initial ? formFrom(initial.name, initial.server) : emptyForm()));
  const [snippetOpen, setSnippetOpen] = useState(false);
  const [snippet, setSnippet] = useState("");
  const [snippetError, setSnippetError] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const patch = (changes: Partial<ServerForm>) => setForm((f) => ({ ...f, ...changes }));
  const pairsKey = form.kind === "local" ? "env" : "headers";

  const applySnippet = () => {
    const parsed = parseSnippet(snippet);
    if (!parsed) {
      setSnippetError(true);
      return;
    }
    const next = formFrom(parsed.name, parsed.server);
    // Keep what the user already chose that a snippet doesn't say.
    setForm({ ...next, name: parsed.name || form.name, confirm: form.confirm, autoApprove: form.autoApprove });
    setSnippet("");
    setSnippetOpen(false);
    setSnippetError(false);
  };

  const nameError = form.name.trim() && !NAME_PATTERN.test(form.name.trim()) ? t("settings.mcp.nameInvalid") : null;
  const missing = missingSecrets(form);
  const complete =
    NAME_PATTERN.test(form.name.trim()) &&
    (form.kind === "local" ? form.command.trim() !== "" : /^https?:\/\//.test(form.url.trim())) &&
    missing.length === 0;

  const save = async () => {
    setBusy(true);
    setError(null);
    try {
      const { server, secrets } = toSaveRequest(form);
      // Keys first: the server starts right after saving and needs them.
      for (const secret of secrets) await api.secretsSet(secret.name, secret.value);
      applyConfig(await api.mcpSaveServer(form.name.trim(), initial?.name ?? null, server));
      onDone();
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <form
      className="space-y-3 rounded-xl border border-accent/30 bg-raised p-3"
      onSubmit={(e) => {
        e.preventDefault();
        if (complete && !busy) void save();
      }}
      onKeyDown={(e) => {
        if (e.key === "Escape") {
          e.stopPropagation();
          onDone();
        }
      }}
    >
      <div className="flex items-center gap-2">
        <h3 className="flex-1 text-title font-medium text-fg">
          {initial ? t("settings.mcp.editTitle", { name: initial.name }) : t("settings.mcp.addTitle")}
        </h3>
        <button
          type="button"
          onClick={() => setSnippetOpen((v) => !v)}
          className={clsx(textButtonClass, "flex items-center gap-1.5", snippetOpen && "bg-sunken text-fg")}
        >
          <ClipboardPaste size={13} /> {t("settings.mcp.pasteSnippet")}
        </button>
      </div>

      {snippetOpen && (
        <div className="space-y-1.5 rounded-lg bg-sunken p-2">
          <p className="text-hint leading-snug text-subtle">{t("settings.mcp.snippetHint")}</p>
          <textarea
            value={snippet}
            autoFocus
            spellCheck={false}
            rows={5}
            onChange={(e) => {
              setSnippet(e.target.value);
              setSnippetError(false);
            }}
            placeholder={'{ "mcpServers": { "github": { "command": "npx", "args": [...] } } }'}
            className="w-full resize-y rounded-md border border-line bg-raised px-2 py-1.5 font-mono text-hint text-fg outline-none placeholder:text-subtle focus:border-accent/40"
          />
          <div className="flex items-center justify-end gap-2">
            {snippetError && <span className="flex-1 text-hint text-danger">{t("settings.mcp.snippetInvalid")}</span>}
            <button type="button" onClick={applySnippet} disabled={!snippet.trim()} className={primaryButtonClass}>
              {t("settings.mcp.snippetApply")}
            </button>
          </div>
        </div>
      )}

      <Field label={t("settings.mcp.name")} hint={t("settings.mcp.nameHint")} error={nameError}>
        <input
          value={form.name}
          autoFocus={!initial}
          spellCheck={false}
          placeholder="github"
          onChange={(e) => patch({ name: e.target.value })}
          className={inputClass}
        />
      </Field>

      <div className="grid grid-cols-2 gap-1 rounded-lg bg-sunken p-0.5">
        {(["local", "remote"] as const).map((kind) => (
          <button
            key={kind}
            type="button"
            onClick={() => patch({ kind })}
            className={clsx(
              "rounded-md py-1 text-body transition-colors",
              form.kind === kind ? "bg-raised text-fg shadow-sm" : "text-muted hover:text-fg",
            )}
          >
            {t(`settings.mcp.kind.${kind}`)}
          </button>
        ))}
      </div>

      {form.kind === "local" ? (
        <>
          <Field label={t("settings.mcp.command")} hint={t("settings.mcp.commandHint")}>
            <input
              value={form.command}
              spellCheck={false}
              placeholder="npx"
              onChange={(e) => patch({ command: e.target.value })}
              className={clsx(inputClass, "font-mono")}
            />
          </Field>
          <Field label={t("settings.mcp.args")} hint={t("settings.mcp.argsHint")}>
            <textarea
              value={form.args}
              spellCheck={false}
              rows={Math.min(6, Math.max(2, form.args.split("\n").length))}
              placeholder={"-y\n@modelcontextprotocol/server-github"}
              onChange={(e) => patch({ args: e.target.value })}
              className={clsx(inputClass, "h-auto resize-y py-1.5 font-mono leading-relaxed")}
            />
          </Field>
        </>
      ) : (
        <Field label={t("settings.mcp.url")}>
          <input
            value={form.url}
            spellCheck={false}
            placeholder="https://example.com/mcp"
            onChange={(e) => patch({ url: e.target.value })}
            className={clsx(inputClass, "font-mono")}
          />
        </Field>
      )}

      <PairList
        label={t(form.kind === "local" ? "settings.mcp.env" : "settings.mcp.headers")}
        hint={t(form.kind === "local" ? "settings.mcp.envHint" : "settings.mcp.headersHint")}
        keyPlaceholder={form.kind === "local" ? "GITHUB_TOKEN" : "Authorization"}
        rows={form[pairsKey]}
        onChange={(rows) => patch({ [pairsKey]: rows })}
      />

      <label className="flex items-center gap-3 rounded-lg bg-sunken px-2.5 py-2">
        <span className="min-w-0 flex-1">
          <span className="block text-body text-fg">{t("settings.mcp.confirm")}</span>
          <span className="block text-hint leading-snug text-subtle">{t("settings.mcp.confirmHint")}</span>
        </span>
        <Toggle label={t("settings.mcp.confirm")} checked={form.confirm} onChange={(confirm) => patch({ confirm })} />
      </label>

      {error && <p className="text-hint leading-snug text-danger select-text">{error}</p>}
      {missing.length > 0 && (
        <p className="text-hint text-warning">{t("settings.mcp.secretMissing", { names: missing.join(", ") })}</p>
      )}

      <div className="flex justify-end gap-2">
        <button type="button" onClick={onDone} className={textButtonClass}>
          {t("common.cancel")}
        </button>
        <button type="submit" disabled={!complete || busy} className={primaryButtonClass}>
          {initial ? t("common.save") : t("settings.mcp.addButton")}
        </button>
      </div>
    </form>
  );
}

function Field({
  label,
  hint,
  error,
  children,
}: {
  label: string;
  hint?: string;
  error?: string | null;
  children: ReactNode;
}) {
  return (
    <label className="block space-y-1">
      <span className="block text-hint text-muted">{label}</span>
      {children}
      {error ? (
        <span className="block text-hint text-danger">{error}</span>
      ) : (
        hint && <span className="block text-hint leading-snug text-subtle">{hint}</span>
      )}
    </label>
  );
}

/** Environment variables or HTTP headers; rows marked secret go to the vault. */
function PairList({
  label,
  hint,
  keyPlaceholder,
  rows,
  onChange,
}: {
  label: string;
  hint: string;
  keyPlaceholder: string;
  rows: PairRow[];
  onChange: (rows: PairRow[]) => void;
}) {
  const { t } = useTranslation();
  const [revealed, setRevealed] = useState<number | null>(null);
  const update = (index: number, changes: Partial<PairRow>) =>
    onChange(rows.map((row, i) => (i === index ? { ...row, ...changes } : row)));

  return (
    <div className="space-y-1">
      <div className="flex items-center justify-between">
        <span className="text-hint text-muted">{label}</span>
        <button
          type="button"
          onClick={() => onChange([...rows, emptyRow()])}
          className={clsx(textButtonClass, "flex items-center gap-1")}
        >
          <Plus size={13} /> {t("settings.mcp.addPair")}
        </button>
      </div>
      {rows.map((row, index) => (
        <div key={index} className="flex items-center gap-1">
          <input
            value={row.key}
            spellCheck={false}
            placeholder={keyPlaceholder}
            aria-label={t("settings.mcp.pairName")}
            // Credentials go to the vault by default.
            onChange={(e) => update(index, { key: e.target.value, secret: row.secret || SENSITIVE_KEY.test(e.target.value) })}
            className={clsx(inputClass, "w-[38%] shrink-0 font-mono text-hint")}
          />
          <input
            value={row.value}
            type={row.secret && revealed !== index ? "password" : "text"}
            autoComplete="off"
            spellCheck={false}
            placeholder={row.stored ? t("settings.mcp.secretKept") : t("settings.mcp.pairValue")}
            aria-label={t("settings.mcp.pairValue")}
            onChange={(e) => update(index, { value: e.target.value })}
            className={clsx(inputClass, "min-w-0 flex-1 font-mono text-hint placeholder:font-sans")}
          />
          {row.secret && row.value && (
            <button
              type="button"
              title={t("settings.mcp.reveal")}
              aria-label={t("settings.mcp.reveal")}
              onClick={() => setRevealed((r) => (r === index ? null : index))}
              className="grid size-7 shrink-0 place-items-center rounded-md text-subtle hover:bg-sunken hover:text-fg"
            >
              {revealed === index ? <EyeOff size={13} /> : <Eye size={13} />}
            </button>
          )}
          <button
            type="button"
            title={t(row.secret ? "settings.mcp.secretOn" : "settings.mcp.secretOff")}
            aria-label={t(row.secret ? "settings.mcp.secretOn" : "settings.mcp.secretOff")}
            aria-pressed={row.secret}
            // A stored secret stays secret: turning it off would need the value, which never leaves the vault.
            disabled={row.stored != null && !row.value}
            onClick={() => update(index, { secret: !row.secret })}
            className={clsx(
              "grid size-7 shrink-0 place-items-center rounded-md transition-colors hover:bg-sunken disabled:opacity-60",
              row.secret ? "text-accent" : "text-subtle hover:text-fg",
            )}
          >
            <Lock size={13} />
          </button>
          <button
            type="button"
            title={t("keys.field.remove")}
            aria-label={t("keys.field.remove")}
            onClick={() => onChange(rows.filter((_, i) => i !== index))}
            className="grid size-7 shrink-0 place-items-center rounded-md text-subtle hover:bg-sunken hover:text-danger"
          >
            <X size={13} />
          </button>
        </div>
      ))}
      <span className="block text-hint leading-snug text-subtle">{hint}</span>
    </div>
  );
}
