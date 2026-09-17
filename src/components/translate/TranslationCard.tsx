import { ArrowRight } from "lucide-react";
import type { Translation } from "../../lib/ipc";
import { languageName } from "./languages";
import { Alternatives, CopyButton } from "./TranslateView";

/** A translation in the chat (from `tr …` / `übersetze …`). */
export function TranslationCard({ translation }: { translation: Translation }) {
  return (
    <div className="rounded-xl border border-line bg-sunken px-3.5 py-2.5">
      <div className="mb-1 flex items-center gap-1.5 text-[11.5px] text-subtle select-none">
        {languageName(translation.from)} <ArrowRight size={11} /> {languageName(translation.to)}
      </div>
      <div className="flex items-start gap-2">
        <p className="min-w-0 flex-1 text-[15px] leading-relaxed whitespace-pre-wrap text-fg">{translation.text}</p>
        <CopyButton text={translation.text} />
      </div>
      {translation.alternatives.length > 0 && <Alternatives items={translation.alternatives} />}
    </div>
  );
}
