/**
 * Typed bridge to the Rust backend. Mirrors `src-tauri/src/commands.rs`
 * and the serialized types in the Rust modules.
 */
import { Channel, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/** `large` is the enlarged panel (e.g. the web tab after "enlarge"). */
export type WindowMode = "compact" | "expanded" | "large";

/** `src-tauri/src/desktop/web.rs` */
export interface WebState {
  url: string | null;
  title: string;
  loading: boolean;
}

/** Page area of the web tab in CSS pixels, relative to the window. */
export interface WebBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

export type WebCommand = "back" | "forward" | "reload" | "stop" | "openExternal";

export type Permission = "auto" | "confirm";

export interface ProfileInfo {
  id: string;
  label: string;
  model: string;
  tools: boolean;
  /** Provider id; `ollama` for local models. */
  provider: string;
  providerLabel: string;
  /** Logo name for `BrandIcon`. */
  providerIcon: string;
  local: boolean;
  /** False when the provider's API key is missing. */
  available: boolean;
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
  /** UI language, already resolved by the backend (settings may say "system"). */
  language: "en" | "de";
  configDir: string;
  hotkey: string;
  defaultProfile: string;
  profiles: ProfileInfo[];
  tools: ToolInfo[];
  diagnostics: Diagnostic[];
}

/** `src-tauri/src/secrets/catalog.rs`. Secret values never leave the backend. */
export interface SecretInfo {
  name: string;
  label: string;
  sensitive: boolean;
  placeholder: string;
  isSet: boolean;
  usedBy: string[];
}

export interface SecretGroup {
  id: string;
  title: string;
  description: string;
  /** Logo name: anthropic | gemini | openai | openrouter | ai | music | wrench */
  icon: string;
  category: "ai" | "integration" | "custom";
  docsUrl: string | null;
  testable: boolean;
  secrets: SecretInfo[];
}

/** `src-tauri/src/search/mod.rs` */
export interface WebResult {
  title: string;
  url: string;
  snippet: string;
  engines: string[];
}

/** `src-tauri/src/tools/executor.rs`: structured tool output the UI renders richer than text. */
export type ToolDisplay =
  | { kind: "webResults"; query: string; results: WebResult[] }
  /** A page opened in the web tab. */
  | { kind: "webPage"; url: string };

/** `src-tauri/src/agent/events.rs` */
export type AgentEvent =
  | { type: "token"; text: string }
  | { type: "toolCall"; callId: string; name: string; title: string; args: unknown; fast: boolean }
  | { type: "toolAwaitingConfirmation"; callId: string; preview: string }
  | {
      type: "toolResult";
      callId: string;
      ok: boolean;
      output: string;
      durationMs: number;
      missingSecrets: string[];
      display: ToolDisplay | null;
    }
  | { type: "notice"; message: string }
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
  /** Lets the model answer from the latest tool results (e.g. summarize search results). */
  agentContinue: (profileId: string | null, onEvent: (event: AgentEvent) => void) => {
    const channel = new Channel<AgentEvent>();
    channel.onmessage = onEvent;
    return invoke<void>("agent_continue", { profileId, onEvent: channel });
  },
  agentPreload: (profileId: string) => invoke<void>("agent_preload", { profileId }),
  agentCancel: () => invoke<void>("agent_cancel"),
  agentConfirm: (callId: string, approved: boolean) => invoke<boolean>("agent_confirm", { callId, approved }),
  agentReset: () => invoke<void>("agent_reset"),

  windowSetMode: (mode: WindowMode) => invoke<void>("window_set_mode", { mode }),
  windowHide: () => invoke<void>("window_hide"),
  windowSetPinned: (pinned: boolean) => invoke<void>("window_set_pinned", { pinned }),

  openExternal: (url: string) => invoke<void>("open_external", { url }),

  webState: () => invoke<WebState>("web_state"),
  /** Address bar input: URL, domain or search terms. */
  webNavigate: (input: string) => invoke<void>("web_navigate", { input }),
  /** Opens a link in the web tab; the backend then emits `orbit://web-open`. */
  webOpen: (url: string) => invoke<void>("web_open", { url }),
  /** `seq` must increase with every call: IPC calls can arrive out of order. */
  webSetLayout: (bounds: WebBounds, visible: boolean, seq: number) =>
    invoke<void>("web_set_layout", { bounds, visible, seq }),
  webCommand: (command: WebCommand) => invoke<void>("web_command", { command }),

  secretsList: () => invoke<SecretGroup[]>("secrets_list"),
  /** Resolves with a success message, rejects with the reason. */
  secretsTest: (groupId: string) => invoke<string>("secrets_test", { groupId }),
  secretsSet: (name: string, value: string) => invoke<void>("secrets_set", { name, value }),
  secretsDelete: (name: string) => invoke<void>("secrets_delete", { name }),
};

export const onWindowShown = (handler: () => void): Promise<UnlistenFn> => listen("orbit://shown", handler);

export const onWebState = (handler: (state: WebState) => void): Promise<UnlistenFn> =>
  listen<WebState>("orbit://web-state", (event) => handler(event.payload));

/** A tool opened a page in the web tab. */
export const onWebOpen = (handler: () => void): Promise<UnlistenFn> => listen("orbit://web-open", handler);

export const onConfigChanged = (handler: (snapshot: ConfigSnapshot) => void): Promise<UnlistenFn> =>
  listen<ConfigSnapshot>("orbit://config-changed", (event) => handler(event.payload));
