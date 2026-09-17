import { create } from "zustand";
import { api, type ConfigSnapshot, type WindowMode } from "../lib/ipc";

interface UiState {
  mode: WindowMode;
  pinned: boolean;
  config: ConfigSnapshot | null;
  profileId: string | null;

  setMode: (mode: WindowMode) => void;
  toggleMode: () => void;
  togglePinned: () => void;
  setProfile: (id: string) => void;
  applyConfig: (config: ConfigSnapshot) => void;
}

export const useUi = create<UiState>()((set, get) => ({
  mode: "compact",
  pinned: false,
  config: null,
  profileId: null,

  setMode: (mode) => {
    if (get().mode === mode) return;
    set({ mode });
    void api.windowSetMode(mode);
  },

  toggleMode: () => get().setMode(get().mode === "compact" ? "expanded" : "compact"),

  togglePinned: () => {
    const pinned = !get().pinned;
    set({ pinned });
    void api.windowSetPinned(pinned);
  },

  setProfile: (profileId) => set({ profileId }),

  applyConfig: (config) => {
    const current = get().profileId;
    const stillExists = config.profiles.some((p) => p.id === current);
    set({ config, profileId: stillExists ? current : config.defaultProfile });
  },
}));
