import { create } from "zustand";
import { api } from "../lib/ipc";
import { applyAgentEvent, needsPanel, type AssistantMessage, type Message } from "./chatModel";
import { useUi } from "./ui";

/** How long a successful fast-path confirmation stays visible before Orbit hides itself. */
const AUTO_HIDE_MS = 1400;

interface ChatState {
  messages: Message[];
  running: boolean;

  send: (text: string) => Promise<void>;
  cancel: () => void;
  confirm: (callId: string, approved: boolean) => void;
  reset: () => Promise<void>;
}

export const useChat = create<ChatState>()((set, get) => {
  const updateAssistant = (id: string, update: (message: AssistantMessage) => AssistantMessage) =>
    set((state) => ({
      messages: state.messages.map((m) => (m.id === id && m.role === "assistant" ? update(m) : m)),
    }));

  const findAssistant = (id: string) =>
    get().messages.find((m): m is AssistantMessage => m.id === id && m.role === "assistant");

  return {
    messages: [],
    running: false,

    send: async (raw) => {
      const text = raw.trim();
      if (!text || get().running) return;

      const ui = useUi.getState();
      ui.showFlash(null);
      const assistantId = crypto.randomUUID();
      set((state) => ({
        running: true,
        messages: [
          ...state.messages,
          { id: crypto.randomUUID(), role: "user", text },
          { id: assistantId, role: "assistant", parts: [], status: "streaming" },
        ],
      }));

      try {
        await api.agentSend(text, ui.profileId, (event) => {
          updateAssistant(assistantId, (message) => applyAgentEvent(message, event));
          if (needsPanel(event)) useUi.getState().setMode("expanded");
        });
      } catch (error) {
        updateAssistant(assistantId, (message) => applyAgentEvent(message, { type: "error", message: String(error) }));
        useUi.getState().setMode("expanded");
      } finally {
        set({ running: false });
      }

      // Still compact means everything ran via the fast path: confirm briefly, then get out of the way.
      // Failures open the panel instead, where the full message (and e.g. the keys view) fits.
      const { mode, pinned, showFlash, setMode } = useUi.getState();
      const tool = findAssistant(assistantId)?.parts.find((p) => p.kind === "tool");
      if (mode === "compact" && tool?.kind === "tool") {
        if (tool.status !== "ok") {
          setMode("expanded");
          return;
        }
        const text = tool.output ?? tool.title;
        showFlash({ ok: true, text });
        if (!pinned) {
          setTimeout(() => {
            if (useUi.getState().flash?.text === text) void api.windowHide();
          }, AUTO_HIDE_MS);
        }
      }
    },

    cancel: () => void api.agentCancel(),

    confirm: (callId, approved) => {
      set((state) => ({
        messages: state.messages.map((m) =>
          m.role === "assistant"
            ? {
                ...m,
                parts: m.parts.map((p) => (p.kind === "tool" && p.callId === callId ? { ...p, status: "running" } : p)),
              }
            : m,
        ),
      }));
      void api.agentConfirm(callId, approved);
    },

    reset: async () => {
      await api.agentReset();
      set({ messages: [] });
    },
  };
});
