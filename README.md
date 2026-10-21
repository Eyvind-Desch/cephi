# Cephi

> Always-available desktop assistant for Windows: ask AI, search the web, and get things done on your PC.
> Runs locally with Ollama.

- **Category:** tools
- **Created:** 2026-09-16
- **Status:** wip <!-- idea | wip | usable | done | abandoned -->
- **Name:** from cephalopod, spoken "Kefi". Called *Orbit* until 2026-09-18; on first start Cephi takes over
  the old settings, tools, chats and keys automatically (`src-tauri/src/migrate.rs`), and old tool files
  (`$env:ORBIT_ARG_…`, `"target": "orbit"`) keep working.

## Why

No need to open a separate browser window just to search for something or ask the AI.
One hotkey, one input field: answer, search, or action ("öffne die Bluetooth-Einstellungen" / "open Bluetooth settings").

## Usage

| Key | Action |
|---|---|
| `Ctrl+Alt+Space` | Show/hide Cephi (configurable under ⋯ → *Settings* → *Shortcuts*) |
| `Ctrl+Alt+T` | Pick up the text selected in any app (translate, explain, ask about it) |
| `Enter` / `Shift+Enter` | Send / new line |
| `Ctrl+E` | Compact bar ↔ large panel |
| `Ctrl+N` | New chat (the previous one is archived) |
| `Ctrl+1` … `Ctrl+4` | Chat, web tab, translator, notes |
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | Next / previous tab |
| `Esc` | Back to the chat, in the chat: hide (also works inside a web page that doesn't use Esc itself) |
| `Ctrl+L` | Web tab: address bar |
| `/` | Commands: `/clear`, `/web …`, `/translate …`, `/model …`, `/history`, `/screenshot`, `/tools`, `/settings`, `/shortcuts`, `/keys`, `/mcp`, `/pin`, `/config`, `/reload` |

Clicking outside hides Cephi, unless it's pinned (⋯ menu). Tray icon: open, configuration, quit.

**Your own shortcuts:** ⋯ → *Settings* → *Shortcuts* lists everything above and lets you add global key
combinations of your own: open Cephi, pick up the selected text, screenshot, dictate, new chat, translator,
new note, web tab, earlier chats, tools, settings — or *send a message*. A message that is also a quick phrase
(`pause`, `? weather`, `tr word`) runs straight away, without the AI, so a shortcut can drive anything
Cephi's tools can do.

The chat is saved after every answer and is still there after a restart; *New chat* archives it (the last 50
chats are kept in `%APPDATA%\io.github.eyvind-desch.cephi\chats\`). ⋯ → *Settings* → *Start with Windows* makes Cephi wait in the tray
after login (installed version only).

**Selected text:** mark text anywhere (browser, PDF, chat), press `Ctrl+Alt+T`, and it appears above the input
with *Translate* and *Explain*; typing your own question sends the text along as context. Cephi copies the
selection for that moment and **restores your clipboard afterwards**. The normal hotkey never touches the clipboard.

**Dictation:** the microphone button next to the input records what you say and writes it into the input field
(offline via a local Whisper container, [setup](docs/VOICE.md)). `Esc` throws the recording away.

**Screenshot:** ⋯ → *Screenshot* (or `/screenshot`) freezes the monitor the mouse is on — Cephi hides itself
for the shot — and lays it over the screen: drag a rectangle to pick a part, `Enter` takes the whole monitor,
`Esc` throws it away. The cut-out lands above the input: *What is on it?* or your own question about it.
You can also paste a picture from the clipboard (Snipping Tool, a copied image) into the input. This all
needs a model that can read pictures — Cephi knows which ones (cloud models, and local vision models
like `llava` or `…vl`), and the attached screenshot offers to switch to one. Pictures stay in the session: a saved chat never keeps a screenshot.

The panel has three tabs (icons top right): **Chat**, **Web** and **Translator**. The web tab shows Google and other pages inside
Cephi (address bar: search terms or an address) and can be enlarged; links from the chat and search results open there too.

### Quick actions (no AI, instant)

| Input | Action |
|---|---|
| `öffne discord` / "open discord", `starte den rechner` / "launch calculator" | Launch app |
| `spiele another love von tom odell auf spotify` / "play another love by tom odell on spotify" | Play song ([setup](docs/SPOTIFY.md)) |
| `pause` / "pause", `nächster song` / "next song", `stumm` / "mute", `ton an` / "unmute" | Playback |
| `lautstärke auf 30` / "volume to 30", `mach 24 lauter` / "turn it up by 24", `mach es leiser` / "turn it down" | Volume (exact) |
| `tr word`, `übersetze …` / "translate …" | Offline translation German ↔ English ([setup](docs/TRANSLATE.md)) |
| `? search term` | Web search in Cephi: result cards + short AI answer with sources ([setup](docs/SEARCH.md)) |
| `g search term` | Google in Cephi's web tab |
| `yt search term` | YouTube in the browser |

Everything else goes to the AI, which can call the same tools. Custom quick actions: `triggers` in the
tool JSON files ([docs/TOOLS.md](docs/TOOLS.md)). Settings (⋯ → *Settings*, or `/settings`): general options, API keys and MCP servers, all without editing JSON.

## Configuration

Everything lives in **`%APPDATA%\Cephi\`** and is reloaded immediately on changes:

```
Cephi/
├── settings.json   language, shortcuts, model profiles, AI providers, window, agent
├── tools/          custom tools as JSON → docs/TOOLS.md
├── mcp.json        MCP servers (tools from other apps) → docs/MCP.md
└── schemas/        for editor autocompletion (generated automatically)
```

`settings.json` also has a `"language": "system" | "en" | "de"` field (default `system`, which follows
the Windows display language). The UI and all messages are localized (English source, German translation).

## Install

```powershell
npm run app:build
.\src-tauri\target\release\bundle\nsis\Cephi_0.4.0_x64-setup.exe
```

The installer needs no admin rights (installs for the current user, German or English). Settings, tools and
keys are shared with the development build. Only one Cephi runs at a time, so close the dev version (`npm run app`)
before starting the installed one. After installing: ⋯ → *Settings* → *Start with Windows*.

## Development

Requirements: Node 22+, Rust (MSVC), VS C++ Build Tools, Ollama.

```powershell
npm install
npm run app          # dev mode with hot reload
npm run check        # TypeScript + Clippy + Rust tests
npm run app:build    # installer (NSIS) → src-tauri/target/release/bundle/
npm run clean        # throws src-tauri/target away (a fresh build takes ~4 minutes)
```

Custom config folder for testing: `$env:CEPHI_CONFIG_DIR = "X:\tmp\cephi-config"`.
Logs: `%LOCALAPPDATA%\io.github.eyvind-desch.cephi\logs\`.

**Build folder.** A debug build with symbols for every dependency reaches tens of gigabytes and
Cargo never prunes it, so dependencies are compiled without debug information (`[profile.dev]` in
`src-tauri/Cargo.toml`); our own code keeps line numbers, so panics stay readable. That keeps
`src-tauri/target` at about 3.5 GB instead of 60.

## Docs

- [PLAN.md](PLAN.md): goals, decisions, stages
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md): structure and data flow
- [docs/TOOLS.md](docs/TOOLS.md): custom tools, quick actions, API keys
- [docs/SPOTIFY.md](docs/SPOTIFY.md): Spotify setup
- [docs/SEARCH.md](docs/SEARCH.md): web search (local SearXNG)
- [docs/TRANSLATE.md](docs/TRANSLATE.md): offline translator (local LibreTranslate)
- [docs/VOICE.md](docs/VOICE.md): dictation (local Whisper)
- [docs/MCP.md](docs/MCP.md): MCP servers (tools from other apps, Claude Desktop format)
- [docs/PROVIDERS.md](docs/PROVIDERS.md): cloud AI providers (Claude, Gemini, OpenAI-compatible)
