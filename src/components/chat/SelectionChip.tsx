import { Languages, Quote, Sparkles } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useChat } from "../../store/chat";
import { useTranslate } from "../../store/translate";
import { useUi } from "../../store/ui";
import { Chip, ChipAction } from "./Chip";

/** Text picked up from another app (selection hotkey): shown above the composer with quick actions. */
export function SelectionChip() {
  const { t } = useTranslation();
  const selection = useUi((s) => s.selection);
  const setSelection = useUi((s) => s.setSelection);
  const setView = useUi((s) => s.setView);
  const send = useChat((s) => s.send);
  const running = useChat((s) => s.running);
  if (!selection) return null;

  const use = (action: () => void) => {
    setSelection(null);
    action();
  };

  return (
    <Chip
      onDismiss={() => setSelection(null)}
      footer={
        <>
          <ChipAction
            icon={Languages}
            label={t("selection.translate")}
            onClick={() =>
              use(() => {
                useTranslate.setState({ text: selection });
                setView("translate");
              })
            }
          />
          <ChipAction
            icon={Sparkles}
            label={t("selection.explain")}
            disabled={running}
            onClick={() => use(() => void send(t("selection.explainPrompt", { text: selection })))}
          />
        </>
      }
    >
      <Quote size={13} className="mt-1 shrink-0 text-subtle" />
      <p className="line-clamp-3 min-w-0 flex-1 text-body leading-snug text-muted select-text">{selection}</p>
    </Chip>
  );
}
