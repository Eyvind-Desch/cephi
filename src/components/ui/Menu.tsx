import clsx from "clsx";
import { Check, type LucideIcon } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { useEffect, useId, useRef, useState, type KeyboardEvent, type ReactNode } from "react";

export type MenuEntry =
  | {
      kind?: "item";
      label: string;
      icon: LucideIcon;
      onSelect: () => void;
      shortcut?: string;
      checked?: boolean;
      disabled?: boolean;
    }
  | { kind: "separator" };

interface MenuProps {
  /** Renders the trigger button; receives the props it must spread. */
  trigger: (props: { open: boolean; toggle: () => void; id: string }) => ReactNode;
  entries: MenuEntry[];
}

/** Lightweight dropdown menu: keyboard navigable, closes on outside click and Escape. */
export function Menu({ trigger, entries }: MenuProps) {
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(0);
  const rootRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const id = useId();

  const items = entries.flatMap((entry, index) => (entry.kind === "separator" || entry.disabled ? [] : [index]));

  useEffect(() => {
    if (!open) return;
    setActive(items[0] ?? 0);
    listRef.current?.focus();
    const onPointerDown = (e: PointerEvent) => {
      if (!rootRef.current?.contains(e.target as Node)) setOpen(false);
    };
    window.addEventListener("pointerdown", onPointerDown);
    return () => window.removeEventListener("pointerdown", onPointerDown);
    // Only on open: `items` is derived from `entries`, re-running on every render would reset the selection.
  }, [open]);

  const select = (index: number) => {
    const entry = entries[index];
    if (!entry || entry.kind === "separator" || entry.disabled) return;
    setOpen(false);
    entry.onSelect();
  };

  const onKeyDown = (e: KeyboardEvent) => {
    const position = items.indexOf(active);
    const move = (delta: number) => setActive(items[(position + delta + items.length) % items.length]);
    switch (e.key) {
      case "ArrowDown":
        move(1);
        break;
      case "ArrowUp":
        move(-1);
        break;
      case "Home":
        setActive(items[0]);
        break;
      case "End":
        setActive(items[items.length - 1]);
        break;
      case "Enter":
      case " ":
        select(active);
        break;
      case "Escape":
      case "Tab":
        setOpen(false);
        break;
      default:
        return;
    }
    // Keep Escape from also hiding the whole window.
    e.preventDefault();
    e.stopPropagation();
  };

  return (
    <div ref={rootRef} className="relative">
      {trigger({ open, toggle: () => setOpen((v) => !v), id })}
      <AnimatePresence>
        {open && (
          <motion.div
            ref={listRef}
            id={id}
            role="menu"
            tabIndex={-1}
            onKeyDown={onKeyDown}
            initial={{ opacity: 0, scale: 0.96, y: -4 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.96, y: -4 }}
            transition={{ duration: 0.12, ease: "easeOut" }}
            style={{ transformOrigin: "top right" }}
            className="absolute top-full right-0 z-50 mt-1.5 w-60 rounded-xl border border-line bg-popover p-1 shadow-xl shadow-black/20 outline-none"
          >
            {entries.map((entry, index) =>
              entry.kind === "separator" ? (
                <div key={`sep-${index}`} role="separator" className="mx-2 my-1 h-px bg-line" />
              ) : (
                <button
                  key={entry.label}
                  type="button"
                  role={entry.checked === undefined ? "menuitem" : "menuitemcheckbox"}
                  aria-checked={entry.checked}
                  disabled={entry.disabled}
                  tabIndex={-1}
                  onPointerEnter={() => !entry.disabled && setActive(index)}
                  onClick={() => select(index)}
                  className={clsx(
                    "flex w-full items-center gap-2.5 rounded-lg px-2.5 py-1.5 text-left text-[13px] disabled:opacity-40",
                    active === index && !entry.disabled ? "bg-sunken text-fg" : "text-muted",
                  )}
                >
                  <entry.icon size={15} className="shrink-0" />
                  <span className="flex-1">{entry.label}</span>
                  {entry.checked && <Check size={14} className="text-accent" />}
                  {entry.shortcut && <span className="text-[11px] text-subtle">{entry.shortcut}</span>}
                </button>
              ),
            )}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
