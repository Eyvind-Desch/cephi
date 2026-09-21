import { create } from "zustand";
import { errorText } from "../lib/errors";
import { api, type AgentEvent } from "../lib/ipc";
import { applyAgentEvent, needsPanel, restoreMessages, type AssistantMessage, type Message } from "./chatModel";
import { isUntranslated } from "./translate";
import { useUi } from "./ui";

/** How long a successful fast-path confirmation stays visible before Cephi hides itself. */
const AUTO_HIDE_MS = 1400;

interface ChatState {
  messages: Message[];
  running: boolean;
  /** Text in the composer; kept in the store so it survives switching between bar and panel. */
  draft: string;

  setDraft: (draft: string) => void;

  send: (text: string) => Promise<void>;
  /** Asks the model to answer from the search results at the end of the last message. */
  summarize: (messageId: string) => Promise<void>;
  cancel: () => void;
  confirm: (callId: string, approved: boolean) => void;
  reset: () => Promise<void>;
  /** Shows the chat saved in the last session (at startup). */
  restore: () => Promise<void>;
  /** Continues an archived chat; the current one goes to the archive. */
  openArchived: (id: string) => Promise<void>;
}

export const useChat = create<ChatState>()((set, get) => {
  const updateAssistant = (id: string, update: (message: AssistantMessage) => AssistantMessage) =>
    set((state) => ({
      messages: state.messages.map((m) => (m.id === id && m.role === "assistant" ? update(m) : m)),
    }));

  const findAssistant = (id: string) =>
    get().messages.find((m): m is AssistantMessage => m.id === id && m.role === "assistant");

  /** Runs a backend request and folds its events into the given assistant message. */
  const stream = async (assistantId: string, start: (onEvent: (event: AgentEvent) => void) => Promise<void>) => {
    try {
      await start((event) => {
        updateAssistant(assistantId, (message) => applyAgentEvent(message, event));
        if (needsPanel(event)) useUi.getState().expand();
      });
    } catch (error) {
      updateAssistant(assistantId, (message) => applyAgentEvent(message, { type: "error", message: errorText(error) }));
      useUi.getState().expand();
    } finally {
      set({ running: false });
      // Every finished request is saved, so the chat survives a restart.
      void api.chatSave(get().messages).catch((e) => console.warn("chat not saved", e));
    }
  };

  return {
    messages: [],
    running: false,
    draft: "",

    setDraft: (draft) => set({ draft }),

    send: async (raw) => {
      const text = raw.trim();
      if (!text || get().running) return;

      const ui = useUi.getState();
      ui.showFlash(null);
      // Answers appear in the chat, even if the panel last showed another view.
      if (ui.view !== "chat") useUi.setState({ view: "chat" });
      // A screenshot waiting above the composer belongs to this question.
      const screenshot = ui.screenshot;
      if (screenshot) ui.setScreenshot(null);
      const assistantId = crypto.randomUUID();
      set((state) => ({
        running: true,
        messages: [
          ...state.messages,
          { id: crypto.randomUUID(), role: "user", text, image: screenshot?.data },
          { id: assistantId, role: "assistant", parts: [], status: "streaming" },
        ],
      }));

      await stream(assistantId, (onEvent) =>
        api.agentSend(text, ui.profileId, onEvent, screenshot ? [screenshot.data] : undefined),
      );

      // Still compact means everything ran via the fast path: confirm briefly, then get out of the way.
      // Failures open the panel instead, where the full message (and e.g. the keys view) fits.
      const { mode, pinned, showFlash, expand } = useUi.getState();
      const tool = findAssistant(assistantId)?.parts.find((p) => p.kind === "tool");
      if (mode === "compact" && tool?.kind === "tool") {
        if (tool.status !== "ok") {
          expand();
          return;
        }
        // Something to read and copy, not just a confirmation: keep it until the next interaction.
        // Without a translation the panel opens instead, where the AI can take over.
        if (tool.display?.kind === "translation") {
          if (isUntranslated(tool.display)) expand();
          else showFlash({ ok: true, text: tool.display.text, sticky: true });
          return;
        }
        const text = tool.output ?? tool.title;
        const volume = tool.display?.kind === "volume" ? tool.display : undefined;
        showFlash({ ok: true, text, volume });
        if (!pinned) {
          setTimeout(() => {
            if (useUi.getState().flash?.text === text) void api.windowHide();
          }, AUTO_HIDE_MS);
        }
      }
    },

    summarize: async (messageId) => {
      const message = findAssistant(messageId);
      const last = get().messages.at(-1);
      if (get().running || !message || last?.id !== messageId) return;

      set({ running: true });
      updateAssistant(messageId, (m) => ({ ...m, status: "streaming", notice: undefined }));
      await stream(messageId, (onEvent) => api.agentContinue(useUi.getState().profileId, onEvent));
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

    openArchived: async (id) => {
      if (get().running) return;
      const saved = await api.chatArchiveOpen(id);
      set({ messages: restoreMessages(saved) });
      useUi.getState().setView("chat");
    },

    restore: async () => {
      const saved = await api.chatLoad();
      // Only if nothing was typed in the meantime.
      if (get().messages.length === 0) set({ messages: restoreMessages(saved) });
    },
  };
});
