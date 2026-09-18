import { FileText, Globe, X } from "lucide-react";
import { motion } from "motion/react";
import { useTranslation } from "react-i18next";
import { useChat } from "../../store/chat";
import { useUi } from "../../store/ui";

/** The web tab's page the next question is about: shown above the composer with a quick summary action. */
export function PageChip() {
  const { t } = useTranslation();
  const page = useUi((s) => s.pageContext);
  const setPageContext = useUi((s) => s.setPageContext);
  const send = useChat((s) => s.send);
  const running = useChat((s) => s.running);
  if (!page) return null;

  const host = hostOf(page.url);

  return (
    <motion.div
      initial={{ opacity: 0, y: 4 }}
      animate={{ opacity: 1, y: 0 }}
      className="mb-2 flex items-center gap-2 rounded-xl border border-line bg-raised py-1.5 pr-1.5 pl-3"
    >
      <Globe size={13} className="shrink-0 text-subtle" />
      <span className="min-w-0 flex-1 truncate text-[12.5px] text-muted" title={page.url}>
        {page.title || host}
        {page.title && <span className="text-subtle"> · {host}</span>}
      </span>
      <button
        type="button"
        disabled={running}
        onClick={() => {
          setPageContext(null);
          void send(t("pageChip.summarizePrompt", { url: page.url, title: page.title || host }));
        }}
        className="flex shrink-0 items-center gap-1.5 rounded-lg border border-line px-2.5 py-1 text-[12.5px] text-muted select-none hover:bg-sunken hover:text-fg disabled:opacity-40"
      >
        <FileText size={13} /> {t("pageChip.summarize")}
      </button>
      <button
        type="button"
        onClick={() => setPageContext(null)}
        title={t("selection.dismiss")}
        className="grid size-6 shrink-0 place-items-center rounded-md text-subtle hover:bg-sunken hover:text-fg"
      >
        <X size={13} />
      </button>
    </motion.div>
  );
}

function hostOf(url: string): string {
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return url;
  }
}
