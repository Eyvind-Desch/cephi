import { create } from "zustand";
import { setLanguage } from "../i18n";
import { errorText } from "../lib/errors";
import { api, type ConfigSnapshot, type Screenshot, type WindowMode } from "../lib/ipc";

export type PanelView = "chat" | "history" | "settings" | "tools" | "web" | "translate";
export type SettingsTab = "general" | "keys" | "mcp";

export interface PageContext {
  url: string;
  title: string;
}

/** Short confirmation shown in the compact bar after a successful fast-path action. */
export interface Flash {
  ok: boolean;
  text: string;
  /** Stays until the next interaction and can be copied (e.g. a translation). */
  sticky?: boolean;
  /** After a volume change: shown as a level bar. */
  volume?: { percent: number; muted: boolean };
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
  /** The web tab's page the next question is about ("ask Cephi about this page"). */
  pageContext: PageContext | null;
  /** Screenshot waiting above the composer for a question about it. */
  screenshot: Screenshot | null;
  /** A screenshot is being taken (Cephi hides itself meanwhile). */
  capturing: boolean;
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
  setPageContext: (page: PageContext | null) => void;
  setScreenshot: (shot: Screenshot | null) => void;
  /** Freezes the screen and opens the overlay to pick a region out of it. */
  captureScreenshot: () => Promise<void>;
  /** The picked region: lands above the composer, in the chat. */
  useScreenshot: (shot: Screenshot) => void;
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
    pageContext: null,
    screenshot: null,
    capturing: false,
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

    setPageContext: (pageContext) => set({ pageContext }),

    setScreenshot: (screenshot) => set({ screenshot }),

    captureScreenshot: async () => {
      if (get().capturing) return;
      set({ capturing: true, flash: null });
      try {
        // The overlay takes over from here; the region comes back as an event (`useScreenshotSync`).
        await api.screenshotRegion();
      } catch (error) {
        set({ flash: { ok: false, text: errorText(error) } });
      } finally {
        set({ capturing: false });
      }
    },

    useScreenshot: (screenshot) => {
      set({ screenshot });
      get().setView("chat");
      get().expand();
    },

    setOverlay: (open) => set((s) => ({ overlays: Math.max(0, s.overlays + (open ? 1 : -1)) })),

    toggleWebLarge: () => {
      const webLarge = !get().webLarge;
      set({ webLarge });
      if (get().mode !== "compact") setMode(panelMode(get().view, webLarge));
    },
  };
});
