import { create } from "zustand";
import { api } from "../lib/ipc";
import { applyAgentEvent, type AssistantMessage, type Message } from "./chatModel";
import { useUi } from "./ui";

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

  return {
    messages: [],
    running: false,

    send: async (raw) => {
      const text = raw.trim();
      if (!text || get().running) return;

      const assistantId = crypto.randomUUID();
      set((state) => ({
        running: true,
        messages: [
          ...state.messages,
          { id: crypto.randomUUID(), role: "user", text },
          { id: assistantId, role: "assistant", parts: [], status: "streaming" },
        ],
      }));
      useUi.getState().setMode("expanded");

      try {
        await api.agentSend(text, useUi.getState().profileId, (event) =>
          updateAssistant(assistantId, (message) => applyAgentEvent(message, event)),
        );
      } catch (error) {
        updateAssistant(assistantId, (message) => applyAgentEvent(message, { type: "error", message: String(error) }));
      } finally {
        set({ running: false });
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
