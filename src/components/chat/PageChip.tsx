import { FileText, Globe } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useChat } from "../../store/chat";
import { useUi } from "../../store/ui";
import { Chip, ChipAction } from "./Chip";

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
    <Chip align="center" onDismiss={() => setPageContext(null)}>
      <Globe size={13} className="shrink-0 text-subtle" />
      <span className="min-w-0 flex-1 truncate text-body text-muted" title={page.url}>
        {page.title || host}
        {page.title && <span className="text-subtle"> · {host}</span>}
      </span>
      <ChipAction
        icon={FileText}
        label={t("pageChip.summarize")}
        disabled={running}
        onClick={() => {
          setPageContext(null);
          void send(t("pageChip.summarizePrompt", { url: page.url, title: page.title || host }));
        }}
      />
    </Chip>
  );
}

function hostOf(url: string): string {
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return url;
  }
}
