import { useEffect } from "react";
import { api, onConfigChanged, onWebOpen, onWebState } from "../lib/ipc";
import { useChat } from "../store/chat";
import { useUi } from "../store/ui";
import { useWeb } from "../store/web";

/** Loads the configuration and keeps it in sync with hot reloads. */
export function useConfigSync() {
  useEffect(() => {
    const { applyConfig } = useUi.getState();
    void api.configSnapshot().then(applyConfig);
    const unlisten = onConfigChanged(applyConfig);
    return () => void unlisten.then((fn) => fn());
  }, []);
}

/** Mirrors the web tab's page state and switches to the tab when a tool opens a page. */
export function useWebSync() {
  useEffect(() => {
    void api.webState().then((state) => state && useWeb.setState(state));
    const unlistenState = onWebState((state) => useWeb.setState(state));
    const unlistenOpen = onWebOpen(() => useUi.getState().setView("web"));
    return () => {
      void unlistenState.then((fn) => fn());
      void unlistenOpen.then((fn) => fn());
    };
  }, []);
}

/** Window-level keyboard shortcuts. */
export function useKeyboardShortcuts() {
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      const ctrl = e.ctrlKey && !e.altKey && !e.metaKey;
      const key = e.key.toLowerCase();

      if (e.key === "Escape") {
        e.preventDefault();
        // Sub-views go back first; only the chat hides the window.
        const ui = useUi.getState();
        if (ui.mode !== "compact" && ui.view !== "chat") ui.setView("chat");
        else void api.windowHide();
      } else if (ctrl && key === "c") {
        copySelectionOutsideFocus();
      } else if (ctrl && key === "n") {
        e.preventDefault();
        useUi.getState().setView("chat");
        void useChat.getState().reset();
      } else if (ctrl && key === "e") {
        e.preventDefault();
        useUi.getState().toggleMode();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);
}

/**
 * The native copy handles the normal case. It fails when text is selected in the chat while
 * the keyboard focus is still in an input (e.g. the composer): then Ctrl+C would copy the
 * input's empty selection. In that case we copy the page selection ourselves.
 */
function copySelectionOutsideFocus() {
  const active = document.activeElement;
  const inField = active instanceof HTMLInputElement || active instanceof HTMLTextAreaElement;
  if (!inField || active.selectionStart !== active.selectionEnd) return;

  const text = window.getSelection()?.toString() ?? "";
  if (text) void navigator.clipboard.writeText(text);
}

/** Suppresses the browser context menu so the app feels native (keeps it on selectable content). */
export function useNativeContextMenu() {
  useEffect(() => {
    const onContextMenu = (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      const allowed = target.closest("textarea, input, .selectable, pre");
      if (!allowed && !import.meta.env.DEV) e.preventDefault();
    };
    window.addEventListener("contextmenu", onContextMenu);
    return () => window.removeEventListener("contextmenu", onContextMenu);
  }, []);
}
