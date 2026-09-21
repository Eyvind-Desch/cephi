import clsx from "clsx";
import { FolderCog, Plug, ShieldCheck, Zap } from "lucide-react";
import { useTranslation } from "react-i18next";
import { api, type McpServerStatus, type ToolInfo } from "../../lib/ipc";
import { useUi } from "../../store/ui";
import { BackButton } from "../ui/BackButton";

const EMPTY_TOOLS: ToolInfo[] = [];
const EMPTY_SERVERS: McpServerStatus[] = [];

/** All tools (`/tools`): Cephi's own with their quick phrases, then each MCP server with its tools. */
export function ToolsView() {
  const { t } = useTranslation();
  const tools = useUi((s) => s.config?.tools ?? EMPTY_TOOLS);
  const servers = useUi((s) => s.config?.mcpServers ?? EMPTY_SERVERS);
  const own = tools.filter((tool) => tool.kind !== "mcp");

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex items-center justify-between px-2 pt-2">
        <BackButton />
        <button
          type="button"
          onClick={() => void api.configOpenFolder()}
          className="flex items-center gap-1.5 rounded-md px-2 py-1 text-body text-muted transition-colors hover:bg-sunken hover:text-fg"
        >
          <FolderCog size={14} /> {t("tools.openFolder")}
        </button>
      </div>

      <div className="selectable flex-1 space-y-5 overflow-y-auto px-4 pt-3 pb-5">
        <section className="space-y-2">
          <div className="space-y-0.5 px-1">
            <h2 className="text-title font-medium text-fg">{t("tools.title", { count: own.length })}</h2>
            <p className="text-hint text-subtle">{t("tools.hint")}</p>
          </div>
          <ul className="space-y-1.5">
            {own.map((tool) => (
              <ToolRow key={tool.name} tool={tool} />
            ))}
          </ul>
        </section>

        <section className="space-y-2">
          <div className="space-y-0.5 px-1">
            <h2 className="flex items-center gap-1.5 text-title font-medium text-fg">
              <Plug size={14} /> {t("tools.mcpTitle")}
            </h2>
            <p className="text-hint text-subtle">{t("tools.mcpHint")}</p>
          </div>
          {servers.length === 0 ? (
            <p className="rounded-xl border border-dashed border-line px-3 py-3 text-body text-muted">
              {t("tools.mcpEmpty")}
            </p>
          ) : (
            servers.map((server) => (
              <ServerBlock
                key={server.name}
                server={server}
                tools={tools.filter((tool) => tool.source === `mcp.json › ${server.name}`)}
              />
            ))
          )}
        </section>
      </div>
    </div>
  );
}

const STATE_STYLE = {
  running: "bg-success",
  starting: "bg-warning animate-pulse",
  failed: "bg-danger",
  disabled: "bg-subtle",
} as const;

function ServerBlock({ server, tools }: { server: McpServerStatus; tools: ToolInfo[] }) {
  const { t } = useTranslation();
  return (
    <div className="space-y-1.5">
      <div className="flex items-center gap-2 px-1 text-body">
        <span className={clsx("size-2 shrink-0 rounded-full", STATE_STYLE[server.state])} />
        <span className="font-medium text-fg">{server.name}</span>
        <span className="text-hint text-subtle">
          {t(`tools.serverState.${server.state}`)}
          {server.state === "running" && ` · ${t("tools.title", { count: server.tools })}`}
        </span>
      </div>
      {server.error && <p className="px-1 text-hint leading-snug text-danger">{server.error}</p>}
      <ul className="space-y-1.5">
        {tools.map((tool) => (
          <ToolRow key={tool.name} tool={tool} />
        ))}
      </ul>
    </div>
  );
}

function ToolRow({ tool }: { tool: ToolInfo }) {
  const { t } = useTranslation();
  const confirm = tool.permission === "confirm";

  return (
    <li className="rounded-xl border border-line bg-raised px-3 py-2">
      <div className="flex items-center gap-2">
        <span className="min-w-0 flex-1 truncate text-body text-fg">{tool.title}</span>
        <span
          title={confirm ? t("tools.confirm") : t("tools.auto")}
          className={clsx("shrink-0", confirm ? "text-warning" : "text-subtle")}
        >
          <ShieldCheck size={13} />
        </span>
        <span className="shrink-0 font-mono text-caption text-subtle">{tool.name}</span>
      </div>
      <p className="mt-0.5 line-clamp-2 text-hint leading-snug text-muted" title={tool.description}>
        {tool.description}
      </p>
      {tool.triggers.length > 0 && (
        <div className="mt-1.5 flex flex-wrap items-center gap-1">
          <Zap size={13} className="shrink-0 text-accent" />
          {tool.triggers.map((trigger) => (
            <code key={trigger} className="rounded bg-sunken px-1.5 py-0.5 font-mono text-caption text-muted">
              {trigger}
            </code>
          ))}
        </div>
      )}
      {tool.kind !== "mcp" && <p className="mt-1 truncate text-caption text-subtle">{tool.source}</p>}
    </li>
  );
}
