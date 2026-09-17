# Orbit – Hinweise für Claude

Tauri 2 (Rust) + React/TypeScript. Architektur: `docs/ARCHITECTURE.md`, Tool-Format: `docs/TOOLS.md`.

## Regeln

- `commands.rs` bleibt dünn; Logik gehört in die Module (`agent`, `tools`, `config`, `desktop`).
- IPC-Typen synchron halten: Rust (`commands.rs`, `agent/events.rs`, `config/snapshot.rs`) ↔ `src/lib/ipc.ts`.
- Konfigurationsfehler nie als Panic/Absturz: als `Diagnostic` melden.
- Neue Felder in `settings.json`/Tool-Manifesten: Rust-Struct + JSON-Schema in `src-tauri/assets/schemas/` + Doku gemeinsam ändern.
- Neue Standard-Tools: Datei in `src-tauri/assets/defaults/tools/` **und** in `config/defaults.rs` registrieren (der Test `all_default_tools_are_valid` prüft sie).
- Sicherheit: PowerShell-Parameter nur über `$env:ORBIT_ARG_*`, keine Shell bei `launch`, WebView-Capabilities minimal lassen.
- UI-Texte auf Deutsch, Code/Kommentare auf Englisch.
- Zustand-Selektoren dürfen keine neuen Objekte/Arrays pro Aufruf zurückgeben (Endlosschleife in v5).

## Prüfen

`npm run check` (tsc + clippy `-D warnings` + cargo test).
