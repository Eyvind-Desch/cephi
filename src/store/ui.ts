import { create } from "zustand";
import { setLanguage } from "../i18n";
import { api, type ConfigSnapshot, type WindowMode } from "../lib/ipc";

export type PanelView = "chat" | "settings" | "tools" | "web" | "translate";
export type SettingsTab = "general" | "keys" | "mcp";

/** Short confirmation shown in the compact bar after a successful fast-path action. */
export interface Flash {
  ok: boolean;
  text: string;
  /** Stays until the next interaction and can be copied (e.g. a translation). */
  sticky?: boolean;
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
  /** The web tab is enlarged (window mode `large`). */
  webLarge: boolean;
  /** Text picked up from another app via the selection hotkey. */
  selection: string | null;
  /** Section shown on the settings page. */
  settingsTab: SettingsTab;

  /** Shows the panel for the current view (no-op if a panel is already shown). */
  expand: () => void;
  collapse: () => void;
  toggleMode: () => void;
  setView: (view: PanelView) => void;
  openSettings: (tab: SettingsTab) => void;
  togglePinned: () => void;
  setProfile: (id: string) => void;
  applyConfig: (config: ConfigSnapshot) => void;
  showFlash: (flash: Flash | null) => void;
  setOverlay: (open: boolean) => void;
  setSelection: (text: string | null) => void;
  toggleWebLarge: () => void;
}

/** Panels share the normal size; only the web tab can be enlarged. */
const panelMode = (view: PanelView, webLarge: boolean): WindowMode =>
  view === "web" && webLarge ? "large" : "expanded";

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
    webLarge: false,
    selection: null,
    settingsTab: "general",

    expand: () => {
      if (get().mode === "compact") setMode(panelMode(get().view, get().webLarge));
    },

    collapse: () => setMode("compact"),

    toggleMode: () => (get().mode === "compact" ? get().expand() : get().collapse()),

    setView: (view) => {
      set({ view });
      setMode(panelMode(view, get().webLarge));
    },

    openSettings: (settingsTab) => {
      set({ settingsTab });
      get().setView("settings");
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

    setSelection: (selection) => set({ selection }),

    setOverlay: (open) => set((s) => ({ overlays: Math.max(0, s.overlays + (open ? 1 : -1)) })),

    toggleWebLarge: () => {
      const webLarge = !get().webLarge;
      set({ webLarge });
      if (get().mode !== "compact") setMode(panelMode(get().view, webLarge));
    },
  };
});
