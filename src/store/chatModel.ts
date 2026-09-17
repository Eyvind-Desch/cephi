import i18n from "../i18n";
import type { AgentEvent, ToolDisplay, WebResult } from "../lib/ipc";

export type ToolStatus = "running" | "awaiting" | "ok" | "error";

export interface TextPart {
  kind: "text";
  text: string;
}

export interface ToolPart {
  kind: "tool";
  callId: string;
  name: string;
  title: string;
  args: unknown;
  status: ToolStatus;
  /** Run directly via a trigger phrase, without the model. */
  fast: boolean;
  preview?: string;
  output?: string;
  durationMs?: number;
  missingSecrets?: string[];
  display?: ToolDisplay;
}

export type Part = TextPart | ToolPart;

export interface UserMessage {
  id: string;
  role: "user";
  text: string;
}

export interface AssistantMessage {
  id: string;
  role: "assistant";
  parts: Part[];
  status: "streaming" | "done" | "cancelled" | "error";
  error?: string;
}

export type Message = UserMessage | AssistantMessage;

/** Pure reducer: folds one backend event into the assistant message being streamed. */
export function applyAgentEvent(message: AssistantMessage, event: AgentEvent): AssistantMessage {
  switch (event.type) {
    case "token": {
      const parts = [...message.parts];
      const last = parts.at(-1);
      if (last?.kind === "text") {
        parts[parts.length - 1] = { ...last, text: last.text + event.text };
      } else {
        parts.push({ kind: "text", text: event.text });
      }
      return { ...message, parts };
    }
    case "toolCall":
      return {
        ...message,
        parts: [
          ...message.parts,
          {
            kind: "tool",
            callId: event.callId,
            name: event.name,
            title: event.title,
            args: event.args,
            fast: event.fast,
            status: "running",
          },
        ],
      };
    case "toolAwaitingConfirmation":
      return updateTool(message, event.callId, { status: "awaiting", preview: event.preview });
    case "toolResult":
      return updateTool(message, event.callId, {
        status: event.ok ? "ok" : "error",
        output: event.output,
        durationMs: event.durationMs,
        missingSecrets: event.missingSecrets,
        display: event.display ?? undefined,
      });
    case "done":
      return { ...message, status: "done" };
    case "cancelled":
      return { ...message, status: "cancelled", parts: settleTools(message.parts) };
    case "error":
      return { ...message, status: "error", error: event.message, parts: settleTools(message.parts) };
  }
}

/**
 * Whether an event needs the full panel. Fast-path tool runs stay in the compact bar;
 * streamed answers, confirmations and errors need room.
 */
export function needsPanel(event: AgentEvent): boolean {
  switch (event.type) {
    case "token":
    case "toolAwaitingConfirmation":
    case "error":
      return true;
    case "toolCall":
      return !event.fast;
    case "toolResult":
      return event.display != null;
    default:
      return false;
  }
}

/**
 * Sources that `[n]` citations in a text part refer to: the most recent web results before it.
 * Returns one entry per part (undefined for tool parts and texts without preceding results).
 */
export function citationSources(parts: Part[]): (WebResult[] | undefined)[] {
  let current: WebResult[] | undefined;
  return parts.map((part) => {
    if (part.kind === "tool") {
      if (part.display?.kind === "webResults") current = part.display.results;
      return undefined;
    }
    return current;
  });
}

function updateTool(message: AssistantMessage, callId: string, patch: Partial<ToolPart>): AssistantMessage {
  return {
    ...message,
    parts: message.parts.map((part) => (part.kind === "tool" && part.callId === callId ? { ...part, ...patch } : part)),
  };
}

/** Tools still running/awaiting when a run ends will never finish. */
function settleTools(parts: Part[]): Part[] {
  return parts.map((part) =>
    part.kind === "tool" && (part.status === "running" || part.status === "awaiting")
      ? { ...part, status: "error", output: part.output ?? i18n.t("chat.cancelled") }
      : part,
  );
}
