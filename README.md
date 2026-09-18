# Orbit

> Always-available desktop assistant for Windows: ask AI, search the web, and get things done on your PC.
> Runs locally with Ollama.

- **Category:** tools
- **Created:** 2026-09-16
- **Status:** wip <!-- idea | wip | usable | done | abandoned -->

## Why

No need to open a separate browser window just to search for something or ask the AI.
One hotkey, one input field: answer, search, or action ("öffne die Bluetooth-Einstellungen" / "open Bluetooth settings").

## Usage

| Key | Action |
|---|---|
| `Ctrl+Alt+Space` | Show/hide Orbit (configurable in `settings.json`) |
| `Ctrl+Alt+T` | Pick up the text selected in any app (translate, explain, ask about it) |
| `Enter` / `Shift+Enter` | Send / new line |
| `Ctrl+E` | Compact bar ↔ large panel |
| `Ctrl+N` | New chat (the previous one is archived) |
| `Esc` | Hide |

Clicking outside hides Orbit, unless it's pinned (⋯ menu). Tray icon: open, configuration, quit.

The chat is saved after every answer and is still there after a restart; *New chat* archives it (the last 50
chats are kept in `%APPDATA%\io.github.eyvind-desch.orbit\chats\`). ⋯ → *Start with Windows* makes Orbit wait in the tray
after login (installed version only).

**Selected text:** mark text anywhere (browser, PDF, chat), press `Ctrl+Alt+T`, and it appears above the input
with *Translate* and *Explain*; typing your own question sends the text along as context. Orbit copies the
selection for that moment and **restores your clipboard afterwards**. The normal hotkey never touches the clipboard.

The panel has three tabs (icons top right): **Chat**, **Web** and **Translator**. The web tab shows Google and other pages inside
Orbit (address bar: search terms or an address) and can be enlarged; links from the chat and search results open there too.

### Quick actions (no AI, instant)

| Input | Action |
|---|---|
| `öffne discord` / "open discord", `starte den rechner` / "launch calculator" | Launch app |
| `spiele another love von tom odell auf spotify` / "play another love by tom odell on spotify" | Play song ([setup](docs/SPOTIFY.md)) |
| `pause` / "pause", `nächster song` / "next song", `stumm` / "mute", `ton an` / "unmute" | Playback |
| `lautstärke auf 30` / "volume to 30", `mach 24 lauter` / "turn it up by 24", `mach es leiser` / "turn it down" | Volume (exact) |
| `tr word`, `übersetze …` / "translate …" | Offline translation German ↔ English ([setup](docs/TRANSLATE.md)) |
| `? search term` | Web search in Orbit: result cards + short AI answer with sources ([setup](docs/SEARCH.md)) |
| `g search term` | Google in Orbit's web tab |
| `yt search term` | YouTube in the browser |

Everything else goes to the AI, which can call the same tools. Custom quick actions: `triggers` in the
tool JSON files ([docs/TOOLS.md](docs/TOOLS.md)). API keys: ⋯ → *Keys & API Keys*.

## Configuration

Everything lives in **`%APPDATA%\Orbit\`** and is reloaded immediately on changes:

```
Orbit/
├── settings.json   language, hotkey, model profiles, AI providers, window, agent
├── tools/          custom tools as JSON → docs/TOOLS.md
└── schemas/        for editor autocompletion (generated automatically)
```

`settings.json` also has a `"language": "system" | "en" | "de"` field (default `system`, which follows
the Windows display language). The UI and all messages are localized (English source, German translation).

## Development

Requirements: Node 22+, Rust (MSVC), VS C++ Build Tools, Ollama.

```powershell
npm install
npm run app          # dev mode with hot reload
npm run check        # TypeScript + Clippy + Rust tests
npm run app:build    # installer (NSIS) → src-tauri/target/release/bundle/
```

Custom config folder for testing: `$env:ORBIT_CONFIG_DIR = "X:\tmp\orbit-config"`.
Logs: `%LOCALAPPDATA%\io.github.eyvind-desch.orbit\logs\`.

## Docs

- [PLAN.md](PLAN.md): goals, decisions, stages
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md): structure and data flow
- [docs/TOOLS.md](docs/TOOLS.md): custom tools, quick actions, API keys
- [docs/SPOTIFY.md](docs/SPOTIFY.md): Spotify setup
- [docs/SEARCH.md](docs/SEARCH.md): web search (local SearXNG)
- [docs/TRANSLATE.md](docs/TRANSLATE.md): offline translator (local LibreTranslate)
- [docs/PROVIDERS.md](docs/PROVIDERS.md): cloud AI providers (Claude, Gemini, OpenAI-compatible)
