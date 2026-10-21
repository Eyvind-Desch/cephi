import type { NoteColor } from "../../lib/ipc";

/** Order of the swatches in the editor; mirrors `COLORS` in `src-tauri/src/notes.rs`. */
export const NOTE_COLORS: NoteColor[] = ["default", "red", "orange", "yellow", "green", "teal", "blue", "purple"];

/** Tints that stay readable on the translucent window in light and dark mode. */
export const CARD_CLASS: Record<NoteColor, string> = {
  default: "border-line bg-raised",
  red: "border-red-500/25 bg-red-500/12",
  orange: "border-orange-500/25 bg-orange-500/12",
  yellow: "border-yellow-500/30 bg-yellow-400/16",
  green: "border-green-500/25 bg-green-500/12",
  teal: "border-teal-500/25 bg-teal-500/12",
  blue: "border-blue-500/25 bg-blue-500/12",
  purple: "border-purple-500/25 bg-purple-500/12",
};

/** The editor sits on top of the list, so its tint must not let the cards show through. */
export const EDITOR_CLASS: Record<NoteColor, string> = {
  default: "bg-popover",
  red: "bg-[color-mix(in_oklab,var(--popover),var(--color-red-500)_14%)]",
  orange: "bg-[color-mix(in_oklab,var(--popover),var(--color-orange-500)_14%)]",
  yellow: "bg-[color-mix(in_oklab,var(--popover),var(--color-yellow-400)_18%)]",
  green: "bg-[color-mix(in_oklab,var(--popover),var(--color-green-500)_14%)]",
  teal: "bg-[color-mix(in_oklab,var(--popover),var(--color-teal-500)_14%)]",
  blue: "bg-[color-mix(in_oklab,var(--popover),var(--color-blue-500)_14%)]",
  purple: "bg-[color-mix(in_oklab,var(--popover),var(--color-purple-500)_14%)]",
};

export const SWATCH_CLASS: Record<NoteColor, string> = {
  default: "bg-raised",
  red: "bg-red-500/70",
  orange: "bg-orange-500/70",
  yellow: "bg-yellow-400/80",
  green: "bg-green-500/70",
  teal: "bg-teal-500/70",
  blue: "bg-blue-500/70",
  purple: "bg-purple-500/70",
};
