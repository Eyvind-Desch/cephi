import type { Translation } from "../../lib/ipc";
import { TranslationView } from "./TranslationView";

/** A translation in the chat (from `tr …` / `übersetze …`). */
export function TranslationCard({ translation }: { translation: Translation }) {
  return (
    <div className="rounded-xl border border-line bg-sunken px-3.5 py-2.5">
      <TranslationView translation={translation} />
    </div>
  );
}
