import { create } from "zustand";
import { setLanguage } from "../i18n";
import { api, type ConfigSnapshot, type WindowMode } from "../lib/ipc";

export type PanelView = "chat" | "keys" | "web";

/** Short confirmation shown in the compact bar after a successful fast-path action. */
export interface Flash {
  ok: boolean;
  text: string;
}

interface UiState {
  mode: WindowMode;
  view: PanelView;
  pinned: boolean;
  config: ConfigSnapshot | null;
  profileId: string | null;
  flash: Flash | null;
  /**
   * Open menus and popovers. The web page is a native view drawn above the UI,
   * so it hides while something would overlap it.
   */
  overlays: number;

  /** Shows the panel for the current view (no-op if a panel is already shown). */
  expand: () => void;
  collapse: () => void;
  toggleMode: () => void;
  setView: (view: PanelView) => void;
  togglePinned: () => void;
  setProfile: (id: string) => void;
  applyConfig: (config: ConfigSnapshot) => void;
  showFlash: (flash: Flash | null) => void;
  setOverlay: (open: boolean) => void;
}

/** The web tab gets a wider window. */
const panelMode = (view: PanelView): WindowMode => (view === "web" ? "web" : "expanded");

export const useUi = create<UiState>()((set, get) => {
  const setMode = (mode: WindowMode) => {
    if (get().mode === mode) return;
    set({ mode, flash: null });
    void api.windowSetMode(mode);
  };

  return {
    mode: "compact",
    view: "chat",
    pinned: false,
    config: null,
    profileId: null,
    flash: null,
    overlays: 0,

    expand: () => {
      if (get().mode === "compact") setMode(panelMode(get().view));
    },

    collapse: () => setMode("compact"),

    toggleMode: () => (get().mode === "compact" ? get().expand() : get().collapse()),

    setView: (view) => {
      set({ view });
      setMode(panelMode(view));
    },

    togglePinned: () => {
      const pinned = !get().pinned;
      set({ pinned });
      void api.windowSetPinned(pinned);
    },

    setProfile: (profileId) => {
      set({ profileId });
      // Load the model now, so the first question doesn't wait for it.
      void api.agentPreload(profileId);
    },

    applyConfig: (config) => {
      setLanguage(config.language);
      const current = get().profileId;
      // Fall back to the default if the profile was removed or its API key was deleted.
      const stillUsable = config.profiles.some((p) => p.id === current && p.available);
      set({ config, profileId: stillUsable ? current : config.defaultProfile });
    },

    showFlash: (flash) => set({ flash }),

    setOverlay: (open) => set((s) => ({ overlays: Math.max(0, s.overlays + (open ? 1 : -1)) })),
  };
});
