import { create } from "zustand";
import { errorText } from "../lib/errors";
import { api } from "../lib/ipc";
import { useChat } from "./chat";
import { useUi } from "./ui";

type Phase = "idle" | "recording" | "transcribing";

interface VoiceState {
  phase: Phase;
  /** Last problem (no microphone, service not running …). */
  error: string | null;
  /** True when the speech service is not running: the error offers to start it. */
  serviceDown: boolean;

  /** Records, or stops and dictates what was said into the composer. */
  toggle: () => Promise<void>;
  /** Throws a running recording away (Escape). */
  cancel: () => void;
  dismissError: () => void;
}

/** Dictation: the recording runs in the backend, this only holds what the UI shows. */
export const useVoice = create<VoiceState>()((set, get) => ({
  phase: "idle",
  error: null,
  serviceDown: false,

  toggle: async () => {
    const phase = get().phase;
    if (phase === "transcribing") return;

    if (phase === "idle") {
      set({ error: null, serviceDown: false });
      try {
        await api.voiceStart();
        set({ phase: "recording" });
      } catch (error) {
        set({ error: errorText(error) });
      }
      return;
    }

    set({ phase: "transcribing" });
    try {
      const text = (await api.voiceStop()).trim();
      set({ phase: "idle" });
      if (!text) return;
      const chat = useChat.getState();
      if (useUi.getState().config?.general.voiceSendDirectly) {
        void chat.send(text);
      } else {
        // Continues what is already typed.
        const draft = chat.draft.trim();
        chat.setDraft(draft ? `${draft} ${text}` : text);
      }
    } catch (error) {
      set({ phase: "idle", error: errorText(error) });
      // Only the button can tell whether the container is simply not running.
      api
        .servicesStatus()
        .then((services) => set({ serviceDown: services.some((s) => s.id === "voice" && !s.running) }))
        .catch(() => undefined);
    }
  },

  cancel: () => {
    if (get().phase !== "recording") return;
    void api.voiceCancel();
    set({ phase: "idle" });
  },

  dismissError: () => set({ error: null, serviceDown: false }),
}));
