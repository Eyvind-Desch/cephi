import { create } from "zustand";
import type { Translation } from "../lib/ipc";

/** `auto` picks the direction from the text; a language code forces the target. */
export type Direction = "auto" | "en" | "de";

interface TranslateState {
  text: string;
  direction: Direction;
  result: Translation | null;
  error: string | null;
  loading: boolean;
  /** Bumped to translate again, e.g. after the service was started. */
  retryToken: number;

  setText: (text: string) => void;
  setDirection: (direction: Direction) => void;
  /** Shows a translation from elsewhere (e.g. a chat card) in the translator tab. */
  show: (translation: Translation) => void;
  retry: () => void;
}

export const useTranslate = create<TranslateState>()((set) => ({
  text: "",
  direction: "auto",
  result: null,
  error: null,
  loading: false,
  retryToken: 0,

  setText: (text) => set({ text }),
  setDirection: (direction) => set({ direction }),
  retry: () => set((s) => ({ result: null, error: null, retryToken: s.retryToken + 1 })),
  show: (translation) => set({ text: translation.source, result: translation, error: null, loading: false }),
}));

/**
 * LibreTranslate returns unknown words (typos, rare terms) unchanged instead of reporting failure.
 * Then the UI offers the AI, which also handles misspellings.
 */
export function isUntranslated(translation: Translation): boolean {
  const normalize = (text: string) => text.trim().toLowerCase();
  return normalize(translation.text) === normalize(translation.source);
}
