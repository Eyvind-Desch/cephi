# Architecture

## Overview

```
┌──────────────────── Frontend: React + TypeScript (WebView2) ────────────────────┐
│ components/shell   CompactBar · Panel · Header · ProfileSwitch · Diagnostics    │
│ components/chat    Composer · MessageList · MessageView · ToolCard · Markdown   │
│ store/             zustand: ui (mode, pin, config) · chat (messages, run)       │
│ lib/ipc.ts         typed bridge to all Rust commands and events                 │
└─────────────── invoke() · Channel<AgentEvent> · Events ─────────────────────────┘
┌──────────────────────────── Backend: Rust (Tauri 2) ────────────────────────────┐
│ commands.rs   thin IPC layer, no logic                                          │
│ app.rs        startup, config reload, hotkey registration                       │
│ state.rs      AppState: config snapshot (Arc, swapped atomically), HTTP, agent  │
│ config/       loads settings.json + tools, default files, schemas, watcher      │
│ tools/        manifest format, validation, templates, execution                 │
│ agent/        loop: model → tools → model, confirmations, cancellation          │
│ llm/          providers: Ollama, Anthropic, OpenAI-compatible (streaming, tools)│
│ secrets/      Windows Credential Manager + key catalog for the keys view        │
│ launcher/ integrations/  app index, Spotify, media/volume, radios                │
│ search/       SearXNG client, excerpts for the model                            │
│ desktop/      window placement, web tab (child webview), tray, global hotkey     │
└─────────────────────────────────────────────────────────────────────────────────┘
          │                       │                          │
    Ollama :11434      SearXNG :8888 (Docker/WSL)   %APPDATA%\Orbit\ (settings.json, tools/)
```

## Principles

- **Configuration instead of code.** Tools and profiles are JSON files with a JSON schema. New capabilities
  normally don't require any Rust code.
- **Never hard-crash because of configuration.** Errors are collected as `Diagnostic` and shown in the UI.
  A broken `settings.json` keeps the last valid settings.
- **Immutable snapshots.** `Arc<Config>` is swapped atomically on reload; a running
  agent run keeps working with its own snapshot, without holding locks across `await`.
- **Secure by default.** Tools ask for confirmation by default (`confirm`). PowerShell receives parameters
  only as environment variables, `launch` doesn't use a shell, `open_url` restricts schemes. The WebView has
  only `core:default`; every system access goes through Orbit's own commands.
- **The backend owns history.** The frontend only renders events; the chat history for the model
  lives in `AgentController`.

## Request flow

```
Composer ─ agent_send(text, profileId, channel) ─▶ agent::run
                                                    │ history + system prompt (with date)
                                                    ▼
                                          OllamaClient::chat (stream)
                                     token ──▶ Channel ──▶ UI (live)
                                                    │
                                     tool_calls?  ──┴── no ──▶ Done
                                          │ yes
                            args::normalize (types, required, enum, default)
                                          │
                          permission = confirm? ──▶ ToolAwaitingConfirmation ──▶ UI card
                                          │                   agent_confirm(callId, ok)
                                  executor::execute
                                          │
                               ToolResult ──▶ UI · result added to history ──▶ next round
```

Cancellation (`agent_cancel`) acts via a `CancellationToken` on both the stream and
pending confirmations.

**Fast path:** before the model is asked, the input is matched against tool `triggers`. A match runs the
tool directly (`ToolCall { fast: true }`) and is recorded in the history like a model tool call. With
`afterTrigger: "answer"` the loop above then continues so the model answers from the result.

**Display data:** besides the text for the model, a tool may return `ToolDisplay` (e.g. web results).
It travels only in the `ToolResult` event, so the UI can render cards without parsing text.

## Web tab

The web tab is a second, native webview (`web`) inside the Orbit window (`tauri` feature `unstable`,
`Window::add_child`). The window is therefore handled as `Window`, not `WebviewWindow`.

- **Layout:** the React `WebView` reserves the page area and reports position and visibility together
  (`web_set_layout`) on every resize. Each report carries an increasing `seq`: IPC calls can arrive out of
  order, and a stale "hidden" must not win. Native views are drawn above the HTML, so the page hides while a
  menu is open (`ui.overlays`).
- **Window size:** the web tab uses the normal panel size; *enlarge* switches the window to mode `large`.
  Mode changes are animated in the backend (`SetWindowPos` per frame, position and size in one call).
- **State:** URL, title and loading state come from page load and title events (`orbit://web-state`).
  Tools with `"target": "orbit"` navigate and emit `orbit://web-open`, which switches to the tab.
- **Isolation:** the capability lists webviews (`"webviews": ["main"]`), not windows, so the web tab gets
  no permissions; Tauri also rejects IPC from remote origins without an explicit `remote` capability.
  Only `http`/`https` navigation is allowed, popups open in the same tab.

## IPC contracts

Rust types and their TypeScript mirrors must stay in sync:

| Rust | TypeScript |
|---|---|
| `commands.rs` | `api` in `src/lib/ipc.ts` |
| `agent/events.rs` → `AgentEvent` | `AgentEvent` |
| `config/snapshot.rs` → `ConfigSnapshot` | `ConfigSnapshot` |

| `desktop/web.rs` → `WebState` | `WebState` |

Events: `orbit://shown` (window shown), `orbit://config-changed` (hot reload), `orbit://web-state`,
`orbit://web-open` (web tab).

## New action types / builtins

1. `tools/manifest.rs`: variant in `Action` (or `Builtin` in `builtin.rs`) + validation
2. `tools/executor.rs`: execution + `preview`
3. `assets/schemas/tool.schema.json`: extend the schema
4. `docs/TOOLS.md`: document it
5. Test in `manifest.rs`
