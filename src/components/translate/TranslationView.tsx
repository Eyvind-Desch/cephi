import clsx from "clsx";
import { ArrowRight, Check, Copy, Sparkles } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { Translation } from "../../lib/ipc";
import { useChat } from "../../store/chat";
import { isUntranslated } from "../../store/translate";
import { useUi } from "../../store/ui";
import { languageName } from "./languages";

/** Result of one translation: languages, text, alternatives — shared by the tab and the chat card. */
export function TranslationView({ translation, dimmed }: { translation: Translation; dimmed?: boolean }) {
  const { t } = useTranslation();
  const failed = isUntranslated(translation);

  return (
    <div className={clsx("transition-opacity", dimmed && "opacity-60")}>
      <div className="mb-1 flex items-center gap-1.5 text-[11.5px] text-subtle select-none">
        {languageName(translation.from)} <ArrowRight size={11} /> {languageName(translation.to)}
      </div>

      {failed ? (
        <div className="space-y-2">
          <p className="text-[13px] text-muted">{t("translate.notFound", { text: translation.source })}</p>
          <AskAiButton translation={translation} />
        </div>
      ) : (
        <>
          <div className="flex items-start gap-2">
            <p className="min-w-0 flex-1 text-[15px] leading-relaxed whitespace-pre-wrap text-fg">{translation.text}</p>
            <CopyButton text={translation.text} />
          </div>
          {translation.alternatives.length > 0 && <Alternatives items={translation.alternatives} />}
        </>
      )}
    </div>
  );
}

/** The dictionary has no entry: hand the text to the model, which also copes with typos. */
function AskAiButton({ translation }: { translation: Translation }) {
  const { t } = useTranslation();
  const send = useChat((s) => s.send);
  const running = useChat((s) => s.running);
  const setView = useUi((s) => s.setView);

  return (
    <button
      type="button"
      disabled={running}
      onClick={() => {
        setView("chat");
        void send(t("translate.aiPrompt", { text: translation.source, language: languageName(translation.to) }));
      }}
      className="flex items-center gap-1.5 rounded-lg border border-line bg-raised px-2.5 py-1.5 text-[12.5px] text-muted select-none hover:bg-sunken hover:text-fg disabled:opacity-40"
    >
      <Sparkles size={13} /> {t("translate.askAi")}
    </button>
  );
}

function Alternatives({ items }: { items: string[] }) {
  const { t } = useTranslation();
  return (
    <div className="mt-2.5 flex flex-wrap items-center gap-1.5">
      <span className="text-[11.5px] text-subtle select-none">{t("translate.alternatives")}</span>
      {items.map((item) => (
        <CopyChip key={item} text={item} />
      ))}
    </div>
  );
}

function CopyChip({ text }: { text: string }) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  return (
    <button
      type="button"
      title={t("common.copy")}
      onClick={async () => {
        await navigator.clipboard.writeText(text);
        setCopied(true);
        setTimeout(() => setCopied(false), 1200);
      }}
      className="inline-flex items-center gap-1 rounded-md border border-line bg-raised px-2 py-0.5 text-[12.5px] text-muted hover:text-fg"
    >
      {copied && <Check size={12} className="text-success" />}
      {text}
    </button>
  );
}

export function CopyButton({ text }: { text: string }) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  return (
    <button
      type="button"
      title={copied ? t("common.copied") : t("common.copy")}
      onClick={async () => {
        await navigator.clipboard.writeText(text);
        setCopied(true);
        setTimeout(() => setCopied(false), 1500);
      }}
      className="grid size-7 shrink-0 place-items-center rounded-md text-subtle hover:bg-raised hover:text-fg"
    >
      {copied ? <Check size={14} className="text-success" /> : <Copy size={14} />}
    </button>
  );
}
