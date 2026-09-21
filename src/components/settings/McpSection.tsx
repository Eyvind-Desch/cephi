import clsx from "clsx";
import { ChevronRight, Loader2, LogIn, Plus } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { errorText } from "../../lib/errors";
import { api, type McpServerConfig, type McpServerStatus, type ToolInfo } from "../../lib/ipc";
import { useUi } from "../../store/ui";
import { primaryButtonClass, textButtonClass, Toggle } from "./controls";
import { formFrom, referencedSecrets } from "./mcpForm";
import { ServerEditor } from "./ServerEditor";

const EMPTY_TOOLS: ToolInfo[] = [];
const EMPTY_SERVERS: McpServerStatus[] = [];
const EMPTY_CONFIG: Record<string, McpServerConfig> = {};
/** How long "Really remove?" waits for the confirming second click. */
const CONFIRM_DELETE_MS = 3000;

const STATE_STYLE = {
  running: "bg-success",
  starting: "bg-warning animate-pulse",
  failed: "bg-danger",
  disabled: "border border-subtle",
} as const;

/** Settings → MCP: connect servers by form; `mcp.json` is written for you. */
export function McpSection() {
  const { t } = useTranslation();
  const servers = useUi((s) => s.config?.mcpConfig ?? EMPTY_CONFIG);
  // `null`: no editor; `""`: a new server; otherwise the name being edited.
  const [editing, setEditing] = useState<string | null>(null);
  const [openName, setOpenName] = useState<string | null>(null);
  const names = Object.keys(servers);

  return (
    <div className="space-y-4">
      <p className="px-1 text-[12px] leading-snug text-subtle">{t("settings.mcp.intro")}</p>

      {editing === "" ? (
        <ServerEditor initial={null} onDone={() => setEditing(null)} />
      ) : (
        <button
          type="button"
          onClick={() => setEditing("")}
          className="flex w-full items-center justify-center gap-1.5 rounded-xl border border-dashed border-line py-2.5 text-[12.5px] text-muted transition-colors hover:border-accent/40 hover:text-fg"
        >
          <Plus size={14} /> {t("settings.mcp.add")}
        </button>
      )}

      {names.length > 0 && (
        <div className="divide-y divide-line overflow-hidden rounded-xl border border-line bg-raised">
          {names.map((name) =>
            editing === name ? (
              <div key={name} className="p-2">
                <ServerEditor initial={{ name, server: servers[name] }} onDone={() => setEditing(null)} />
              </div>
            ) : (
              <ServerRow
                key={name}
                name={name}
                server={servers[name]}
                open={openName === name}
                onToggle={() => setOpenName((n) => (n === name ? null : name))}
                onEdit={() => setEditing(name)}
              />
            ),
          )}
        </div>
      )}
    </div>
  );
}

function ServerRow({
  name,
  server,
  open,
  onToggle,
  onEdit,
}: {
  name: string;
  server: McpServerConfig;
  open: boolean;
  onToggle: () => void;
  onEdit: () => void;
}) {
  const { t } = useTranslation();
  const applyConfig = useUi((s) => s.applyConfig);
  const status = useUi((s) => (s.config?.mcpServers ?? EMPTY_SERVERS).find((x) => x.name === name));
  const allTools = useUi((s) => s.config?.tools ?? EMPTY_TOOLS);
  const tools = allTools.filter((tool) => tool.source === `mcp.json › ${name}`);
  const [error, setError] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);
  /** A browser login is running (the window waits for the redirect). */
  const [busy, setBusy] = useState(false);
  const state = status?.state ?? (server.disabled ? "disabled" : "starting");

  /** Logs in through the browser, or forgets an existing login. */
  const login = async () => {
    setBusy(true);
    setError(null);
    try {
      applyConfig(await (status?.loggedIn ? api.mcpLogout(name) : api.mcpLogin(name)));
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    if (!confirmDelete) return;
    const timer = setTimeout(() => setConfirmDelete(false), CONFIRM_DELETE_MS);
    return () => clearTimeout(timer);
  }, [confirmDelete]);

  const update = (changes: Partial<McpServerConfig>) => {
    setError(null);
    api
      .mcpSaveServer(name, name, { ...server, ...changes })
      .then(applyConfig)
      .catch((e: unknown) => setError(errorText(e)));
  };

  const remove = async () => {
    setError(null);
    try {
      applyConfig(await api.mcpRemoveServer(name));
      // Keys entered for this server in the form are of no use anymore.
      for (const secret of referencedSecrets(server).filter((s) => s.startsWith(`mcp_`))) {
        await api.secretsDelete(secret).catch(() => undefined);
      }
    } catch (e) {
      setError(errorText(e));
    }
  };

  const toolName = (tool: ToolInfo) => tool.name.slice(tool.name.indexOf("__") + 2);
  const setApproved = (tool: string, approved: boolean) =>
    update({
      autoApprove: approved ? [...server.autoApprove, tool] : server.autoApprove.filter((x) => x !== tool),
    });

  const summary = [
    state === "running" ? t("tools.title", { count: status?.tools ?? 0 }) : t(`tools.serverState.${state}`),
    status?.loggedIn ? t("settings.mcp.loggedIn") : null,
  ]
    .filter(Boolean)
    .join(" · ");

  return (
    <div>
      <div className="flex items-center gap-2 pr-3 transition-colors hover:bg-sunken">
        <button
          type="button"
          onClick={onToggle}
          aria-expanded={open}
          className="flex min-w-0 flex-1 items-center gap-2.5 py-2.5 pl-3 text-left"
        >
          <span className={clsx("size-2 shrink-0 rounded-full", STATE_STYLE[state])} />
          <span className="min-w-0 flex-1 truncate text-[13px] text-fg">{name}</span>
          <span className="shrink-0 text-[12px] text-subtle">{summary}</span>
          <ChevronRight size={14} className={clsx("shrink-0 text-subtle transition-transform", open && "rotate-90")} />
        </button>
        <Toggle
          label={t("settings.mcp.enabled")}
          checked={!server.disabled}
          onChange={(enabled) => update({ disabled: !enabled })}
        />
      </div>

      {(open || error) && (
        <div className="space-y-2.5 border-t border-line px-3 py-2.5">
          {error && <p className="text-[12px] leading-snug text-danger select-text">{error}</p>}
          {open && status?.error && <p className="text-[12px] leading-snug text-danger select-text">{status.error}</p>}
          {open && (
            <>
              <p className="truncate font-mono text-[11.5px] text-subtle" title={describe(server)}>
                {describe(server)}
              </p>

              {tools.length > 0 && (
                <div className="space-y-1">
                  <div className="text-[11px] font-medium tracking-wide text-subtle uppercase">
                    {server.permission === "auto" ? t("settings.mcp.toolsAuto") : t("settings.mcp.toolsApprove")}
                  </div>
                  <ul className="space-y-0.5">
                    {tools.map((tool) => {
                      const original = toolName(tool);
                      return (
                        <li key={tool.name} className="flex items-center gap-2 py-0.5">
                          <span className="min-w-0 flex-1">
                            <span className="block truncate text-[12.5px] text-fg">{tool.title}</span>
                            <span className="block truncate text-[11.5px] text-subtle" title={tool.description}>
                              {tool.description.replace(/^\[[^\]]+\]\s*/, "")}
                            </span>
                          </span>
                          {server.permission !== "auto" && (
                            <Toggle
                              label={t("settings.mcp.noConfirm")}
                              checked={server.autoApprove.includes(original)}
                              onChange={(approved) => setApproved(original, approved)}
                            />
                          )}
                        </li>
                      );
                    })}
                  </ul>
                </div>
              )}

              <div className="flex items-center justify-end gap-1">
                {server.url && (
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => void login()}
                    className={clsx(textButtonClass, "flex items-center gap-1.5")}
                  >
                    {busy ? <Loader2 size={13} className="animate-spin" /> : <LogIn size={13} />}
                    {status?.loggedIn ? t("settings.mcp.logout") : t("settings.mcp.login")}
                  </button>
                )}
                <button
                  type="button"
                  onClick={() => (confirmDelete ? void remove() : setConfirmDelete(true))}
                  className={clsx(textButtonClass, confirmDelete && "text-danger hover:text-danger")}
                >
                  {confirmDelete ? t("keys.field.confirmRemove") : t("keys.field.remove")}
                </button>
                <button type="button" onClick={onEdit} className={primaryButtonClass}>
                  {t("settings.mcp.edit")}
                </button>
              </div>
            </>
          )}
        </div>
      )}
    </div>
  );
}

/** `npx -y @modelcontextprotocol/server-github` or the URL. */
function describe(server: McpServerConfig): string {
  const form = formFrom("", server);
  return form.kind === "remote" ? form.url : [form.command, ...server.args].join(" ");
}
