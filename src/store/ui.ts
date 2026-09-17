import { create } from "zustand";
import { setLanguage } from "../i18n";
import { api, type ConfigSnapshot, type WindowMode } from "../lib/ipc";

export type PanelView = "chat" | "keys";

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

  setMode: (mode: WindowMode) => void;
  toggleMode: () => void;
  setView: (view: PanelView) => void;
  togglePinned: () => void;
  setProfile: (id: string) => void;
  applyConfig: (config: ConfigSnapshot) => void;
  showFlash: (flash: Flash | null) => void;
}

export const useUi = create<UiState>()((set, get) => ({
  mode: "compact",
  view: "chat",
  pinned: false,
  config: null,
  profileId: null,
  flash: null,

  setMode: (mode) => {
    if (get().mode === mode) return;
    set({ mode, flash: null });
    void api.windowSetMode(mode);
  },

  toggleMode: () => get().setMode(get().mode === "compact" ? "expanded" : "compact"),

  setView: (view) => {
    set({ view });
    get().setMode("expanded");
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
}));
