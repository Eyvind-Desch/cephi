/**
 * Typed bridge to the Rust backend. Mirrors `src-tauri/src/commands.rs`
 * and the serialized types in the Rust modules.
 */
import { Channel, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type WindowMode = "compact" | "expanded";
export type Permission = "auto" | "confirm";

export interface ProfileInfo {
  id: string;
  label: string;
  model: string;
  tools: boolean;
}

export interface ToolInfo {
  name: string;
  title: string;
  description: string;
  permission: Permission;
  kind: string;
  source: string;
}

export interface Diagnostic {
  source: string;
  message: string;
}

export interface ConfigSnapshot {
  configDir: string;
  hotkey: string;
  defaultProfile: string;
  profiles: ProfileInfo[];
  tools: ToolInfo[];
  diagnostics: Diagnostic[];
}

/** `src-tauri/src/agent/events.rs` */
export type AgentEvent =
  | { type: "token"; text: string }
  | { type: "toolCall"; callId: string; name: string; title: string; args: unknown }
  | { type: "toolAwaitingConfirmation"; callId: string; preview: string }
  | { type: "toolResult"; callId: string; ok: boolean; output: string }
  | { type: "done" }
  | { type: "cancelled" }
  | { type: "error"; message: string };

export const api = {
  configSnapshot: () => invoke<ConfigSnapshot>("config_snapshot"),
  configReload: () => invoke<ConfigSnapshot>("config_reload"),
  configOpenFolder: () => invoke<void>("config_open_folder"),

  /** Resolves when the run has finished, been cancelled or failed. */
  agentSend: (text: string, profileId: string | null, onEvent: (event: AgentEvent) => void) => {
    const channel = new Channel<AgentEvent>();
    channel.onmessage = onEvent;
    return invoke<void>("agent_send", { text, profileId, onEvent: channel });
  },
  agentCancel: () => invoke<void>("agent_cancel"),
  agentConfirm: (callId: string, approved: boolean) => invoke<boolean>("agent_confirm", { callId, approved }),
  agentReset: () => invoke<void>("agent_reset"),

  windowSetMode: (mode: WindowMode) => invoke<void>("window_set_mode", { mode }),
  windowHide: () => invoke<void>("window_hide"),
  windowSetPinned: (pinned: boolean) => invoke<void>("window_set_pinned", { pinned }),

  openExternal: (url: string) => invoke<void>("open_external", { url }),
};

export const onWindowShown = (handler: () => void): Promise<UnlistenFn> => listen("orbit://shown", handler);

export const onConfigChanged = (handler: (snapshot: ConfigSnapshot) => void): Promise<UnlistenFn> =>
  listen<ConfigSnapshot>("orbit://config-changed", (event) => handler(event.payload));
