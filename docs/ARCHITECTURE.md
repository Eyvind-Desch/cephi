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
│ desktop/      window placement, tray, global hotkey                             │
└─────────────────────────────────────────────────────────────────────────────────┘
          │                                    │
    Ollama :11434                   %APPDATA%\Orbit\ (settings.json, tools/)
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

## IPC contracts

Rust types and their TypeScript mirrors must stay in sync:

| Rust | TypeScript |
|---|---|
| `commands.rs` | `api` in `src/lib/ipc.ts` |
| `agent/events.rs` → `AgentEvent` | `AgentEvent` |
| `config/snapshot.rs` → `ConfigSnapshot` | `ConfigSnapshot` |

Events: `orbit://shown` (window shown), `orbit://config-changed` (hot reload).

## New action types / builtins

1. `tools/manifest.rs`: variant in `Action` (or `Builtin` in `builtin.rs`) + validation
2. `tools/executor.rs`: execution + `preview`
3. `assets/schemas/tool.schema.json`: extend the schema
4. `docs/TOOLS.md`: document it
5. Test in `manifest.rs`
