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

/** `src-tauri/src/history.rs` */
export interface ArchivedChat {
  id: string;
  /** RFC 3339 */
  savedAt: string;
  /** The first question, shortened. */
  title: string;
  messages: number;
}

/** `src-tauri/src/desktop/autostart.rs` */
export interface AutostartState {
  enabled: boolean;
  /** False in development builds, which need the dev server. */
  available: boolean;
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
  /** Whether a screenshot can be sent to this model. */
  vision: boolean;
}

/** `src-tauri/src/desktop/capture.rs` */
export interface Screenshot {
  /** Base64 PNG, without the `data:` prefix. */
  data: string;
  width: number;
  height: number;
}

export interface ToolInfo {
  name: string;
  title: string;
  description: string;
  permission: Permission;
  kind: string;
  source: string;
  /** Fast-path phrases as written in the manifest. */
  triggers: string[];
}

/** `src-tauri/src/mcp/config.rs`: one entry of `mcp.json`. */
export interface McpServerConfig {
  command: string | null;
  args: string[];
  env: Record<string, string>;
  cwd: string | null;
  url: string | null;
  headers: Record<string, string>;
  disabled: boolean;
  permission: Permission;
  autoApprove: string[];
}

/** `src-tauri/src/config/snapshot.rs` */
export interface GeneralSettings {
  language: "system" | "en" | "de";
  selectionHotkey: string | null;
  hideOnBlur: boolean;
  summarizeSearch: boolean;
  /** Dictation is sent right away instead of landing in the input field. */
  voiceSendDirectly: boolean;
}

/** `src-tauri/src/mcp/mod.rs` */
export interface McpServerStatus {
  name: string;
  state: "starting" | "running" | "failed" | "disabled";
  error: string | null;
  tools: number;
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
  /** Servers from `mcp.json`. */
  mcpServers: McpServerStatus[];
  /** `mcp.json` as configured (for the settings page). */
  mcpConfig: Record<string, McpServerConfig>;
  general: GeneralSettings;
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

/** `src-tauri/src/services/mod.rs`: Cephi's local containers (Docker in WSL). */
export type ServiceId = "search" | "translate" | "voice";

export interface ServiceStatus {
  id: ServiceId;
  title: string;
  running: boolean;
}

/** `src-tauri/src/translate/mod.rs` */
export interface Translation {
  /** The text that was translated. */
  source: string;
  text: string;
  alternatives: string[];
  /** Language codes, e.g. `en` → `de`. */
  from: string;
  to: string;
}

/** `src-tauri/src/tools/executor.rs`: structured tool output the UI renders richer than text. */
export type ToolDisplay =
  | { kind: "webResults"; query: string; results: WebResult[] }
  /** A page opened in the web tab. */
  | { kind: "webPage"; url: string }
  | ({ kind: "translation" } & Translation)
  /** The system volume after a volume change. */
  | { kind: "volume"; percent: number; muted: boolean };

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
      /** Local services the tool needs that are not running. */
      stoppedServices: ServiceId[];
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
  /** Changes keys in settings.json (objects merge, `null` removes). Rejects with the reason if invalid. */
  settingsUpdate: (patch: Record<string, unknown>) => invoke<ConfigSnapshot>("settings_update", { patch }),
  mcpSaveServer: (name: string, previousName: string | null, server: Partial<McpServerConfig>) =>
    invoke<ConfigSnapshot>("mcp_save_server", { name, previousName, server }),
  mcpRemoveServer: (name: string) => invoke<ConfigSnapshot>("mcp_remove_server", { name }),

  /** Resolves when the run has finished, been cancelled or failed. */
  agentSend: (
    text: string,
    profileId: string | null,
    onEvent: (event: AgentEvent) => void,
    /** Screenshots (base64 PNG) the question is about. */
    images?: string[],
  ) => {
    const channel = new Channel<AgentEvent>();
    channel.onmessage = onEvent;
    return invoke<void>("agent_send", { text, images, profileId, onEvent: channel });
  },
  /** Screenshot of the monitor the mouse is on; Cephi hides itself for the shot. */
  screenshotCapture: () => invoke<Screenshot>("screenshot_capture"),
  /** Starts recording the microphone for dictation. */
  voiceStart: () => invoke<void>("voice_start"),
  /** Stops the recording and returns the recognized text. */
  voiceStop: () => invoke<string>("voice_stop"),
  voiceCancel: () => invoke<void>("voice_cancel"),
  /** Lets the model answer from the latest tool results (e.g. summarize search results). */
  agentContinue: (profileId: string | null, onEvent: (event: AgentEvent) => void) => {
    const channel = new Channel<AgentEvent>();
    channel.onmessage = onEvent;
    return invoke<void>("agent_continue", { profileId, onEvent: channel });
  },
  agentPreload: (profileId: string) => invoke<void>("agent_preload", { profileId }),
  agentCancel: () => invoke<void>("agent_cancel"),
  agentConfirm: (callId: string, approved: boolean) => invoke<boolean>("agent_confirm", { callId, approved }),
  /** New chat: the backend archives the current one. */
  agentReset: () => invoke<void>("agent_reset"),
  /** The saved chat (UI messages) from the last session, or `null`. */
  chatLoad: () => invoke<unknown>("chat_load"),
  chatSave: (ui: unknown) => invoke<void>("chat_save", { ui }),
  chatArchiveList: () => invoke<ArchivedChat[]>("chat_archive_list"),
  /** Opens an archived chat (the current one is archived) and returns its UI messages. */
  chatArchiveOpen: (id: string) => invoke<unknown>("chat_archive_open", { id }),
  chatArchiveDelete: (id: string) => invoke<void>("chat_archive_delete", { id }),

  windowSetMode: (mode: WindowMode) => invoke<void>("window_set_mode", { mode }),
  windowHide: () => invoke<void>("window_hide"),
  windowSetPinned: (pinned: boolean) => invoke<void>("window_set_pinned", { pinned }),

  autostartState: () => invoke<AutostartState>("autostart_state"),
  autostartSet: (enabled: boolean) => invoke<AutostartState>("autostart_set", { enabled }),

  openExternal: (url: string) => invoke<void>("open_external", { url }),

  webState: () => invoke<WebState>("web_state"),
  /** Address bar input: URL, domain or search terms. */
  webNavigate: (input: string) => invoke<void>("web_navigate", { input }),
  /** Opens a link in the web tab; the backend then emits `cephi://web-open`. */
  webOpen: (url: string) => invoke<void>("web_open", { url }),
  /** `seq` must increase with every call: IPC calls can arrive out of order. */
  webSetLayout: (bounds: WebBounds, visible: boolean, seq: number) =>
    invoke<void>("web_set_layout", { bounds, visible, seq }),
  webCommand: (command: WebCommand) => invoke<void>("web_command", { command }),

  /** `to`: language code; `null` = automatic direction. */
  translate: (text: string, to: string | null) => invoke<Translation>("translate", { text, to }),

  servicesStatus: () => invoke<ServiceStatus[]>("services_status"),
  /** Starts a local service; resolves once it answers (may take a while on first start). */
  serviceStart: (service: ServiceId) => invoke<void>("service_start", { service }),

  secretsList: () => invoke<SecretGroup[]>("secrets_list"),
  /** Resolves with a success message, rejects with the reason. */
  secretsTest: (groupId: string) => invoke<string>("secrets_test", { groupId }),
  secretsSet: (name: string, value: string) => invoke<void>("secrets_set", { name, value }),
  secretsDelete: (name: string) => invoke<void>("secrets_delete", { name }),
};

export const onWindowShown = (handler: () => void): Promise<UnlistenFn> => listen("cephi://shown", handler);

export const onWebState = (handler: (state: WebState) => void): Promise<UnlistenFn> =>
  listen<WebState>("cephi://web-state", (event) => handler(event.payload));

/** Text picked up by the selection hotkey; `null` = nothing was selected. */
export const onSelection = (handler: (text: string | null) => void): Promise<UnlistenFn> =>
  listen<string | null>("cephi://selection", (event) => handler(event.payload));

/** `src-tauri/src/desktop/web.rs` (`SHORTCUT_ACTIONS`): a Cephi shortcut pressed inside the web page. */
export type WebShortcut = "escape" | "new-chat" | "toggle-mode" | "address";

export const onWebShortcut = (handler: (action: WebShortcut) => void): Promise<UnlistenFn> =>
  listen<WebShortcut>("cephi://web-shortcut", (event) => handler(event.payload));

/** A tool opened a page in the web tab. */
export const onWebOpen = (handler: () => void): Promise<UnlistenFn> => listen("cephi://web-open", handler);

export const onConfigChanged = (handler: (snapshot: ConfigSnapshot) => void): Promise<UnlistenFn> =>
  listen<ConfigSnapshot>("cephi://config-changed", (event) => handler(event.payload));
