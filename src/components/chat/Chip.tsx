import clsx from "clsx";
import { X, type LucideIcon } from "lucide-react";
import { motion } from "motion/react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

interface ChipProps {
  onDismiss: () => void;
  /** `center` for a single line of content, `start` when it is taller than one line. */
  align?: "start" | "center";
  /** Quick actions or a hint below the content. */
  footer?: ReactNode;
  children: ReactNode;
}

/**
 * The card above the composer that says what the next question is about: text picked up from
 * another app, a screenshot, the page in the web tab — or why dictation did not work. They all look
 * the same and can be put away again.
 */
export function Chip({ onDismiss, align = "start", footer, children }: ChipProps) {
  const { t } = useTranslation();

  return (
    <motion.div
      initial={{ opacity: 0, y: 4 }}
      animate={{ opacity: 1, y: 0 }}
      className="mb-2 rounded-xl border border-line bg-raised py-2 pr-1.5 pl-3"
    >
      <div className={clsx("flex gap-2", align === "center" ? "items-center" : "items-start")}>
        {children}
        <button
          type="button"
          onClick={onDismiss}
          title={t("common.dismiss")}
          className="grid size-6 shrink-0 place-items-center rounded-md text-subtle hover:bg-sunken hover:text-fg"
        >
          <X size={13} />
        </button>
      </div>
      {footer && <div className="mt-1.5 flex flex-wrap gap-1.5">{footer}</div>}
    </motion.div>
  );
}

interface ChipActionProps {
  icon: LucideIcon;
  label: string;
  disabled?: boolean;
  onClick: () => void;
}

/** One of the things a chip offers to do right away ("Translate", "Summarize", …). */
export function ChipAction({ icon: Icon, label, disabled, onClick }: ChipActionProps) {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      className="flex shrink-0 items-center gap-1.5 rounded-lg border border-line px-2.5 py-1 text-body text-muted select-none hover:bg-sunken hover:text-fg disabled:opacity-40"
    >
      <Icon size={13} /> {label}
    </button>
  );
}
