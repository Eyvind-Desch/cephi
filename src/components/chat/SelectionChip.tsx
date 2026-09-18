import { Languages, Quote, Sparkles, X } from "lucide-react";
import { motion } from "motion/react";
import { useTranslation } from "react-i18next";
import { useChat } from "../../store/chat";
import { useTranslate } from "../../store/translate";
import { useUi } from "../../store/ui";

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
    <motion.div
      initial={{ opacity: 0, y: 4 }}
      animate={{ opacity: 1, y: 0 }}
      className="mb-2 rounded-xl border border-line bg-raised px-3 py-2"
    >
      <div className="flex items-start gap-2">
        <Quote size={13} className="mt-1 shrink-0 text-subtle" />
        <p className="line-clamp-3 min-w-0 flex-1 text-[12.5px] leading-snug text-muted select-text">{selection}</p>
        <button
          type="button"
          onClick={() => setSelection(null)}
          title={t("selection.dismiss")}
          className="grid size-6 shrink-0 place-items-center rounded-md text-subtle hover:bg-sunken hover:text-fg"
        >
          <X size={13} />
        </button>
      </div>
      <div className="mt-1.5 flex flex-wrap gap-1.5">
        <Action
          icon={Languages}
          label={t("selection.translate")}
          onClick={() =>
            use(() => {
              useTranslate.setState({ text: selection });
              setView("translate");
            })
          }
        />
        <Action
          icon={Sparkles}
          label={t("selection.explain")}
          disabled={running}
          onClick={() => use(() => void send(t("selection.explainPrompt", { text: selection })))}
        />
      </div>
    </motion.div>
  );
}

interface ActionProps {
  icon: typeof Languages;
  label: string;
  disabled?: boolean;
  onClick: () => void;
}

function Action({ icon: Icon, label, disabled, onClick }: ActionProps) {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      className="flex items-center gap-1.5 rounded-lg border border-line px-2.5 py-1 text-[12.5px] text-muted select-none hover:bg-sunken hover:text-fg disabled:opacity-40"
    >
      <Icon size={13} /> {label}
    </button>
  );
}
