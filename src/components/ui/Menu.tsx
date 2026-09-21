import clsx from "clsx";
import { Check } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { useEffect, useId, useRef, useState, type ComponentType, type KeyboardEvent, type ReactNode } from "react";
import { useUi } from "../../store/ui";

/** Lucide icons and `BrandIcon` wrappers both fit. */
export type MenuIcon = ComponentType<{ size?: number; className?: string }>;

export type MenuEntry =
  | {
      kind?: "item";
      /** Stable key; defaults to the label. */
      id?: string;
      label: string;
      /** Second line under the label. */
      description?: string;
      icon?: MenuIcon;
      onSelect: () => void;
      shortcut?: string;
      /** Small emphasised note on the right, e.g. "Key missing". */
      hint?: string;
      checked?: boolean;
      disabled?: boolean;
    }
  | { kind: "separator" }
  | { kind: "label"; text: string };

interface MenuProps {
  /** Renders the trigger button; receives the props it must spread. */
  trigger: (props: { open: boolean; toggle: () => void; id: string }) => ReactNode;
  entries: MenuEntry[];
  /** Tailwind width class of the popup. */
  width?: string;
}

const isSelectable = (entry: MenuEntry) => entry.kind !== "separator" && entry.kind !== "label" && !entry.disabled;

/** Lightweight dropdown menu: keyboard navigable, closes on outside click and Escape. */
export function Menu({ trigger, entries, width = "w-60" }: MenuProps) {
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(0);
  const rootRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const id = useId();

  const items = entries.flatMap((entry, index) => (isSelectable(entry) ? [index] : []));

  // The web page is drawn above the UI and would cover the popup.
  useEffect(() => {
    if (!open) return;
    const { setOverlay } = useUi.getState();
    setOverlay(true);
    return () => setOverlay(false);
  }, [open]);

  useEffect(() => {
    if (!open) return;
    // Start on the checked entry if there is one.
    const checked = entries.findIndex((e) => isSelectable(e) && e.kind !== "separator" && e.kind !== "label" && e.checked);
    setActive(checked >= 0 ? checked : (items[0] ?? 0));
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
    if (!entry || !isSelectable(entry) || entry.kind === "separator" || entry.kind === "label") return;
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
            className={clsx(
              "absolute top-full right-0 z-50 mt-1.5 rounded-xl border border-line bg-popover p-1 shadow-xl shadow-black/20 outline-none",
              width,
            )}
          >
            {entries.map((entry, index) => {
              if (entry.kind === "separator") {
                return <div key={`sep-${index}`} role="separator" className="mx-2 my-1 h-px bg-line" />;
              }
              if (entry.kind === "label") {
                return (
                  <div
                    key={`label-${index}`}
                    className="px-2.5 pt-2 pb-1 text-caption font-medium tracking-wider text-subtle uppercase"
                  >
                    {entry.text}
                  </div>
                );
              }
              const Icon = entry.icon;
              return (
                <button
                  key={entry.id ?? entry.label}
                  type="button"
                  role={entry.checked === undefined ? "menuitem" : "menuitemcheckbox"}
                  aria-checked={entry.checked}
                  disabled={entry.disabled}
                  tabIndex={-1}
                  onPointerEnter={() => !entry.disabled && setActive(index)}
                  onClick={() => select(index)}
                  className={clsx(
                    "flex w-full items-center gap-2.5 rounded-lg px-2.5 py-1.5 text-left text-body disabled:opacity-40",
                    active === index && !entry.disabled ? "bg-sunken text-fg" : "text-muted",
                  )}
                >
                  {Icon && <Icon size={14} className="shrink-0" />}
                  <span className="min-w-0 flex-1">
                    <span className="block truncate">{entry.label}</span>
                    {entry.description && (
                      <span className="block truncate text-caption text-subtle">{entry.description}</span>
                    )}
                  </span>
                  {entry.hint && <span className="shrink-0 text-caption text-subtle">{entry.hint}</span>}
                  {entry.checked && <Check size={14} className="shrink-0 text-accent" />}
                  {entry.shortcut && <span className="shrink-0 text-caption text-subtle">{entry.shortcut}</span>}
                </button>
              );
            })}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
