import { ArrowLeft } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useUi } from "../../store/ui";

/** The way back to the chat, on every view that is not the chat (Esc does the same). */
export function BackButton() {
  const { t } = useTranslation();
  const setView = useUi((s) => s.setView);

  return (
    <button
      type="button"
      onClick={() => setView("chat")}
      title={t("common.backTitle")}
      className="flex items-center gap-1.5 rounded-md px-2 py-1 text-body text-muted transition-colors hover:bg-sunken hover:text-fg"
    >
      <ArrowLeft size={14} /> {t("common.back")}
    </button>
  );
}
