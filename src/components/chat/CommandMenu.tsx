import clsx from "clsx";
import { useTranslation } from "react-i18next";

export interface CommandItem {
  key: string;
  title: string;
  /** Muted text after the title, e.g. the argument or the model name. */
  hint?: string;
  description?: string;
  /** Enter. */
  apply: () => void;
  /** Tab: replaces the input (e.g. `/web ` to start typing the argument). */
  complete?: string;
}

interface CommandMenuProps {
  items: CommandItem[];
  active: number;
  onHover: (index: number) => void;
}

/** Suggestions above the composer while a slash command is typed. Keyboard handling lives in the composer. */
export function CommandMenu({ items, active, onHover }: CommandMenuProps) {
  const { t } = useTranslation();

  return (
    <div
      role="listbox"
      className="absolute inset-x-0 bottom-full z-40 mb-2 flex flex-col rounded-xl border border-line bg-popover p-1 shadow-xl shadow-black/20"
    >
      <div className="max-h-64 overflow-y-auto">
        {items.length === 0 ? (
          <p className="px-2.5 py-2 text-[12.5px] text-subtle">{t("commands.unknown")}</p>
        ) : (
          items.map((item, index) => (
            <button
              key={item.key}
              type="button"
              role="option"
              aria-selected={index === active}
              tabIndex={-1}
              // Keep the focus in the composer.
              onMouseDown={(e) => e.preventDefault()}
              onPointerEnter={() => onHover(index)}
              onClick={item.apply}
              className={clsx(
                "flex w-full items-baseline gap-2 rounded-lg px-2.5 py-1.5 text-left",
                index === active ? "bg-sunken" : "",
              )}
            >
              <span className="shrink-0 font-mono text-[12.5px] text-fg">{item.title}</span>
              {item.hint && <span className="shrink-0 text-[12px] text-subtle">{item.hint}</span>}
              {item.description && (
                <span className="min-w-0 flex-1 truncate text-right text-[12px] text-muted">{item.description}</span>
              )}
            </button>
          ))
      )}
      </div>
      <p className="border-t border-line px-2.5 pt-1.5 pb-1 text-[11px] text-subtle">{t("commands.keysHint")}</p>
    </div>
  );
}
