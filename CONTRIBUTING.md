# Contributing to Cephi

Thanks for taking a look! Bug reports, ideas and pull requests are all welcome.

## Setup

You need Windows 10/11, [Node.js](https://nodejs.org) 20+, [Rust](https://rustup.rs) (stable) and, to actually
chat, [Ollama](https://ollama.com) with the models from the README's *Getting started* section.

```powershell
npm install
npm run app        # the app with hot reload
npm run dev        # only the UI in a browser, with mocked backend data (src/dev/preview.ts)
```

## Before a pull request

```powershell
npm run check      # TypeScript, clippy with -D warnings, cargo test
```

Please keep pull requests focused on one thing, and describe what changed and how you tried it.

## Rules of the codebase

The short version lives in [CLAUDE.md](CLAUDE.md); the most important points:

- `src-tauri/src/commands.rs` stays thin. Logic goes into the modules (`agent`, `tools`, `config`, `desktop`, …).
- IPC types in Rust and `src/lib/ipc.ts` change together.
- A broken config file is reported as a diagnostic, never a crash.
- New settings or tool manifest fields: Rust struct + JSON schema in `src-tauri/assets/schemas/` + docs.
- Code, comments and docs are in English. Every user-facing string is a translation key in **both**
  `en` and `de` (Rust: `src-tauri/locales/`, UI: `src/i18n/locales/`). A third language is very welcome.

## Easy first contributions

- **A new tool:** most tools are a single JSON file, no Rust needed. See [docs/TOOLS.md](docs/TOOLS.md).
- **A translation:** translate `src/i18n/locales/en.ts` and `src-tauri/locales/en.json`, then add the language
  next to `en`/`de` in `src/i18n/index.ts`, `src-tauri/src/i18n.rs` and the language setting.
- **Docs:** anything that confused you while setting up is worth fixing.
- **Make existing features better:** use Cephi for a day. Whatever felt slow, clumsy or surprising
  (a tool that misunderstood you, an answer that took too long, a missing option) is worth an issue or a fix.
- **Improve the UI:** spacing, readability, animations, keyboard handling, accessibility, light/dark details.
  For visual changes, please add a before/after screenshot to the pull request. `npm run dev` shows the UI
  in a browser with demo data, so you can work on it without the backend.

Look for issues labelled `good first issue`, or open one to ask before starting something big.
