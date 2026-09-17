# Architektur

## Überblick

```
┌──────────────────── Frontend: React + TypeScript (WebView2) ────────────────────┐
│ components/shell   CompactBar · Panel · Header · ProfileSwitch · Diagnostics    │
│ components/chat    Composer · MessageList · MessageView · ToolCard · Markdown   │
│ store/             zustand: ui (Modus, Pin, Config) · chat (Nachrichten, Lauf)  │
│ lib/ipc.ts         typisierte Brücke zu allen Rust-Commands und Events          │
└─────────────── invoke() · Channel<AgentEvent> · Events ─────────────────────────┘
┌──────────────────────────── Backend: Rust (Tauri 2) ────────────────────────────┐
│ commands.rs   dünne IPC-Schicht, keine Logik                                    │
│ app.rs        Start, Config-Reload, Hotkey-Anwendung                            │
│ state.rs      AppState: Config-Snapshot (Arc, atomar getauscht), HTTP, Agent    │
│ config/       settings.json + Tools laden, Standarddateien, Schemas, Watcher    │
│ tools/        Manifest-Format, Validierung, Templates, Ausführung               │
│ agent/        Schleife Modell → Tools → Modell, Bestätigungen, Abbruch          │
│ llm/          Ollama-Client (NDJSON-Streaming, Tool-Calls)                      │
│ desktop/      Fenster-Platzierung, Tray, globaler Hotkey                        │
└─────────────────────────────────────────────────────────────────────────────────┘
          │                                    │
    Ollama :11434                   %APPDATA%\Orbit\ (settings.json, tools/)
```

## Grundsätze

- **Konfiguration statt Code.** Tools und Profile sind JSON-Dateien mit JSON-Schema. Neue Fähigkeiten
  brauchen im Normalfall keinen Rust-Code.
- **Nie hart abstürzen wegen Konfiguration.** Fehler werden als `Diagnostic` gesammelt und im UI
  angezeigt. Eine kaputte `settings.json` behält die letzten gültigen Einstellungen.
- **Unveränderliche Snapshots.** `Arc<Config>` wird beim Reload atomar ersetzt; ein laufender
  Agent-Lauf arbeitet mit seinem Snapshot weiter, ohne Locks über `await` zu halten.
- **Sicher per Default.** Tools fragen standardmäßig nach (`confirm`). PowerShell bekommt Parameter nur
  als Umgebungsvariablen, `launch` nutzt keine Shell, `open_url` begrenzt Schemata. Die WebView hat
  nur `core:default`; jeder Systemzugriff geht über Orbits eigene Commands.
- **Backend besitzt den Verlauf.** Das Frontend rendert nur Events; der Chatverlauf für das Modell
  liegt in `AgentController`.

## Ablauf einer Anfrage

```
Composer ─ agent_send(text, profileId, channel) ─▶ agent::run
                                                    │ Verlauf + System-Prompt (mit Datum)
                                                    ▼
                                          OllamaClient::chat (Stream)
                                     token ──▶ Channel ──▶ UI (live)
                                                    │
                                     tool_calls?  ──┴── nein ──▶ Done
                                          │ ja
                            args::normalize (Typen, Pflicht, enum, default)
                                          │
                          permission = confirm? ──▶ ToolAwaitingConfirmation ──▶ UI-Karte
                                          │                   agent_confirm(callId, ok)
                                  executor::execute
                                          │
                               ToolResult ──▶ UI · Ergebnis in den Verlauf ──▶ nächste Runde
```

Abbruch (`agent_cancel`) wirkt über einen `CancellationToken` sowohl auf den Stream als auch auf
wartende Bestätigungen.

## IPC-Verträge

Rust-Typen und ihre TypeScript-Spiegel müssen synchron bleiben:

| Rust | TypeScript |
|---|---|
| `commands.rs` | `api` in `src/lib/ipc.ts` |
| `agent/events.rs` → `AgentEvent` | `AgentEvent` |
| `config/snapshot.rs` → `ConfigSnapshot` | `ConfigSnapshot` |

Events: `orbit://shown` (Fenster eingeblendet), `orbit://config-changed` (Hot-Reload).

## Neue Aktionstypen / Builtins

1. `tools/manifest.rs`: Variante in `Action` (bzw. `Builtin` in `builtin.rs`) + Validierung
2. `tools/executor.rs`: Ausführung + `preview`
3. `assets/schemas/tool.schema.json`: Schema ergänzen
4. `docs/TOOLS.md`: dokumentieren
5. Test in `manifest.rs`
