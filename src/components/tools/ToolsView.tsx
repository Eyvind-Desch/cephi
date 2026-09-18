import clsx from "clsx";
import { ArrowLeft, FolderCog, ShieldCheck, Zap } from "lucide-react";
import { useTranslation } from "react-i18next";
import { api, type ToolInfo } from "../../lib/ipc";
import { useUi } from "../../store/ui";

/** All loaded tools with their quick phrases (`/tools`). */
export function ToolsView() {
  const { t } = useTranslation();
  const tools = useUi((s) => s.config?.tools);
  const setView = useUi((s) => s.setView);

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex items-center justify-between px-2 pt-2">
        <button
          type="button"
          onClick={() => setView("chat")}
          title={t("keys.backTitle")}
          className="flex items-center gap-1.5 rounded-md px-2 py-1 text-[12.5px] text-muted transition-colors hover:bg-sunken hover:text-fg"
        >
          <ArrowLeft size={14} /> {t("keys.back")}
        </button>
        <button
          type="button"
          onClick={() => void api.configOpenFolder()}
          className="flex items-center gap-1.5 rounded-md px-2 py-1 text-[12.5px] text-muted transition-colors hover:bg-sunken hover:text-fg"
        >
          <FolderCog size={14} /> {t("tools.openFolder")}
        </button>
      </div>

      <div className="selectable flex-1 space-y-3 overflow-y-auto px-4 pt-3 pb-5">
        <div className="space-y-0.5 px-1">
          <h2 className="text-[15px] font-medium text-fg">{t("tools.title", { count: tools?.length ?? 0 })}</h2>
          <p className="text-[12px] text-subtle">{t("tools.hint")}</p>
        </div>
        <ul className="space-y-1.5">
          {tools?.map((tool) => <ToolRow key={tool.name} tool={tool} />)}
        </ul>
      </div>
    </div>
  );
}

function ToolRow({ tool }: { tool: ToolInfo }) {
  const { t } = useTranslation();
  const confirm = tool.permission === "confirm";

  return (
    <li className="rounded-xl border border-line bg-raised px-3 py-2">
      <div className="flex items-center gap-2">
        <span className="min-w-0 flex-1 truncate text-[13px] text-fg">{tool.title}</span>
        <span
          title={confirm ? t("tools.confirm") : t("tools.auto")}
          className={clsx("shrink-0", confirm ? "text-warning" : "text-subtle")}
        >
          <ShieldCheck size={13} />
        </span>
        <span className="shrink-0 font-mono text-[11px] text-subtle">{tool.name}</span>
      </div>
      <p className="mt-0.5 line-clamp-2 text-[12px] leading-snug text-muted" title={tool.description}>
        {tool.description}
      </p>
      {tool.triggers.length > 0 && (
        <div className="mt-1.5 flex flex-wrap items-center gap-1">
          <Zap size={11} className="shrink-0 text-accent" />
          {tool.triggers.map((trigger) => (
            <code key={trigger} className="rounded bg-sunken px-1.5 py-0.5 font-mono text-[11px] text-muted">
              {trigger}
            </code>
          ))}
        </div>
      )}
      <p className="mt-1 truncate text-[11px] text-subtle">{tool.source}</p>
    </li>
  );
}
