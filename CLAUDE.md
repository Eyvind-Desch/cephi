# Orbit – Notes for Claude

Tauri 2 (Rust) + React/TypeScript. Architecture: `docs/ARCHITECTURE.md`, tool format: `docs/TOOLS.md`.

## Rules

- `commands.rs` stays thin; logic belongs in the modules (`agent`, `tools`, `config`, `desktop`).
- Keep IPC types in sync: Rust (`commands.rs`, `agent/events.rs`, `config/snapshot.rs`) ↔ `src/lib/ipc.ts`.
- Never turn configuration errors into a panic/crash: report them as a `Diagnostic`.
- New fields in `settings.json`/tool manifests: update the Rust struct + JSON schema in `src-tauri/assets/schemas/` + docs together.
- New default tools: add the file in `src-tauri/assets/defaults/tools/` **and** register it in `config/defaults.rs` (checked by the `all_default_tools_are_valid` test).
- Security: PowerShell parameters only via `$env:ORBIT_ARG_*`, no shell for `launch`, keep WebView capabilities minimal.
- All code, comments, identifiers and docs in English. User-facing strings only via translation keys: Rust `t!()` with `src-tauri/locales/{en,de}.json`, frontend i18next with `src/i18n/locales/{en,de}.ts`. Every new key must exist in both languages (enforced by a Rust test and TypeScript types).
- Zustand selectors must not return new objects/arrays on every call (infinite loop in v5).

## Checks

`npm run check` (tsc + clippy `-D warnings` + cargo test).
