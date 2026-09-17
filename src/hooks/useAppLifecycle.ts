import { useEffect } from "react";
import { api, onConfigChanged } from "../lib/ipc";
import { useChat } from "../store/chat";
import { useUi } from "../store/ui";

/** Loads the configuration and keeps it in sync with hot reloads. */
export function useConfigSync() {
  useEffect(() => {
    const { applyConfig } = useUi.getState();
    void api.configSnapshot().then(applyConfig);
    const unlisten = onConfigChanged(applyConfig);
    return () => void unlisten.then((fn) => fn());
  }, []);
}

/** Window-level keyboard shortcuts. */
export function useKeyboardShortcuts() {
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      const ctrl = e.ctrlKey && !e.altKey && !e.metaKey;

      if (e.key === "Escape") {
        e.preventDefault();
        void api.windowHide();
      } else if (ctrl && e.key.toLowerCase() === "c" && copySelection()) {
        e.preventDefault();
      } else if (ctrl && e.key.toLowerCase() === "n") {
        e.preventDefault();
        void useChat.getState().reset();
      } else if (ctrl && e.key.toLowerCase() === "e") {
        e.preventDefault();
        useUi.getState().toggleMode();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);
}

/**
 * Copies selected chat text. Text fields keep their native behaviour; for everything else we
 * copy explicitly, because the focus often stays in the composer while text is selected elsewhere.
 */
function copySelection(): boolean {
  const active = document.activeElement;
  if (active instanceof HTMLInputElement || active instanceof HTMLTextAreaElement) {
    const hasFieldSelection = active.selectionStart !== active.selectionEnd;
    if (hasFieldSelection) return false;
  }
  const text = window.getSelection()?.toString() ?? "";
  if (!text) return false;
  void navigator.clipboard.writeText(text);
  return true;
}

/** Suppresses the browser context menu so the app feels native (keeps it on text fields). */
export function useNativeContextMenu() {
  useEffect(() => {
    const onContextMenu = (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      const editable = target.closest("textarea, input, .prose-orbit, pre");
      if (!editable && !import.meta.env.DEV) e.preventDefault();
    };
    window.addEventListener("contextmenu", onContextMenu);
    return () => window.removeEventListener("contextmenu", onContextMenu);
  }, []);
}
