# Cephi – Plan

Always-available desktop assistant for Windows: ask AI, search the web, and get things done on your PC,
without opening an extra window. Runs locally with Ollama; cloud models (Claude, Gemini, OpenAI-compatible) are optional.

## Decisions

| Topic | Decision | Why |
|---|---|---|
| App framework | **Tauri 2** (Rust + React/TS) | ~10 MB, low RAM, WebView2 is already built into Windows, Mica/tray/hotkey available as plugins. Custom UI without limits. |
| Not: PowerToys extension (C#) | – | The Command Palette dictates a list UI: no free-form chat panel, no embedded Google, no custom design. Conceivable later as an *additional* entry point. |
| UI | React + TypeScript + Tailwind + Framer Motion | Modern, fast iteration, smooth animations |
| AI | Ollama (`localhost:11434`) | Models already available |
| Web search | **SearXNG** (Docker in WSL), started from Cephi on demand | See below |
| Tools/actions | Built-in tools + **MCP client** (Rust, `rmcp`) | Actions like "open Spotify" plus arbitrary MCP servers |

### SearXNG vs. DuckDuckGo

- **DuckDuckGo** has no official search API. You scrape the HTML page, which quickly gets
  throttled (rate limiting/CAPTCHA), only provides one source, and can break at any time.
- **SearXNG** runs locally in Docker, queries Google, Bing, Brave, DuckDuckGo, Wikipedia, GitHub, etc.
  simultaneously, has a clean JSON API, no account, no key, no cost, no tracking.
  → **Clearly better.** Docker will also be useful for other projects later.
- A DuckDuckGo fallback was tried (2026-09-18) and dropped: after a few requests it only returns a bot challenge.
  When SearXNG is down, Cephi offers a start button instead.

## UX

```
 hidden ──hotkey──▶  compact (bar, bottom right)  ──Tab/click──▶  large (panel)
 (tray icon)         ┌──────────────────────────┐          ┌─────────────┬──────────────┐
                      │ ◯ Ask me something…    ⏎ │          │ Chat        │ Web / Google │
                      └──────────────────────────┘          │ + sources   │ (Webview)    │
                                                              └─────────────┴──────────────┘
```

- Global hotkey `Ctrl+Alt+Space`, configurable (not `Ctrl+Space`: conflicts with IntelliSense in VS Code)
- `Esc` hides it; clicking outside hides it too, unless the window is pinned
- Mica/acrylic background, light/dark follows the system, rounded corners, smooth transitions
- Responses streamed live, Markdown with code highlighting, copy button
- Tray icon, autostart

### Input: one field for everything

| Input | What happens |
|---|---|
| `öffne spotify` / "open spotify", `spotify` | **Quick action, no AI:** fuzzy search over Start menu apps, launches instantly |
| `g katzen` / "g cats" | Google in the web panel |
| `? wie wird das wetter morgen` / "? what will the weather be like tomorrow" | AI with web search → answer with source cards |
| everything else | AI chat; the AI may call tools |
| `/` | Command list (switch model, new chat, settings, …) |

## Architecture

```
┌──────────────── Frontend (React/TS, WebView2) ────────────────┐
│ Compact bar · Chat panel · Web panel · Settings                │
└──────────────── Tauri IPC (commands + events) ────────────────┘
┌──────────────────────── Rust core ─────────────────────────────┐
│ window     hotkey, tray, state, Mica, always-on-top             │
│ llm        Ollama client (streaming, tool calls)                │
│ agent      loop: model → tool call → result → model             │
│ tools      registry: built-in tools + MCP tools                 │
│ mcp        MCP client (rmcp), starts servers from mcp.json      │
│ launcher   index: Start menu .lnk + Store apps, fuzzy search     │
│ search     SearXNG client, DDG fallback, pages → text            │
│ store      settings + chat history (SQLite)                     │
└───────────────────────────────────────────────────────────────┘
        │                    │                     │
   Ollama :11434      SearXNG :8888 (Docker)   MCP server (stdio/http)
```

## Tools / Actions

Tools are **JSON files** in `%APPDATA%\Cephi\tools\` with a JSON schema (editor autocompletion)
and hot reload. Action types: `open_url`, `launch`, `powershell`, `http`, `builtin`. Details: [docs/TOOLS.md](docs/TOOLS.md).

Shipped: `google_search`, `youtube_search`, `open_website`, `weather`, `datetime`,
`clipboard_read`, `clipboard_write`, `open_explorer`, `open_settings`, `system_info`,
`lock_pc` (confirm), `run_powershell` (confirm).

Built since: `open_app` (launcher index), `web_search` (SearXNG), `fetch_page`, `translate`. Planned: `find_files`.

**Permission levels:** `auto` / `confirm` (default), per tool, later also for MCP tools.

**MCP:** `mcp.json` in the same format as Claude Desktop (`mcpServers`), so existing
server configurations work directly.

### Models

| Task | Model |
|---|---|
| Fast answers | `assistant-fast` (qwen2.5 3B) |
| Tools & web search | `assistant` (qwen3 4B) – handles tools better |
| Thorough | `assistant-big` (gemma4 26B), via toggle |

Small models become unreliable with many tools: send only the ~8 matching tools per
request. Unambiguous commands ("open X") run through the quick action path without AI.

## Stages

1. ✅ **MVP** – Tauri scaffold, hotkey, tray, compact bar ↔ large panel, Ollama chat with streaming,
   model profiles, Mica, light/dark
2. ✅ **Actions** – JSON tool system with schema, validation, hot reload, tool-calling loop,
   confirmation dialogs · quick-action triggers without AI · app launcher · Spotify (Web API search + URI) ·
   media keys · key vault (Windows Credential Manager) · model preload · i18n: English source, German translation
2b. ✅ **Model providers** – `providers` in `settings.json`: Ollama (local), Anthropic (native Messages API),
   OpenAI-compatible (OpenAI, Gemini, OpenRouter, Groq, …); profiles reference a provider; API keys via
   the key vault; model picker grouped Local/Cloud → [docs/PROVIDERS.md](docs/PROVIDERS.md)
3. **Web** – Decided: *both*. Result cards immediately (SearXNG) + AI summary from the
   excerpts only (fast), plus a Google tab in the panel
   - ✅ 3a: SearXNG in Docker/WSL (`infra/searxng`), `web_search` builtin, `? query` fast path with
     `afterTrigger: "answer"`, source grid + citation badges → [docs/SEARCH.md](docs/SEARCH.md)
   - ✅ 3b: Web tab: native child webview in the panel (normal size, animated enlarge), icon tabs, address bar,
     `g query` and chat links open there, `open_url` `target: "cephi"`
3c. ✅ **Translator** – offline via LibreTranslate (Docker, CPU): `tr …` fast path, translator tab, chat card
   → [docs/TRANSLATE.md](docs/TRANSLATE.md). Local LLMs were tested first: ~2.5 s per request, 2–3 GB VRAM,
   unreliable for single words.
4. ✅ **MCP** – `mcp.json` in Claude Desktop format (local + HTTP servers), tools offered as `<server>__<tool>`,
   confirmation unless approved, keys via `{{secret:…}}`, status in `/tools` → [docs/MCP.md](docs/MCP.md)
5. **Polish** – ✅ save chat history (restored on start, archived on "new chat") · ✅ start with Windows · ✅ installer
   (NSIS, per-user, own icon) · ✅ slash commands + tools overview · ✅ settings page (general, keys, MCP servers by form) · ✅ code highlighting
6. **Extras** – ✅ pick up selected text (`Ctrl+Alt+T`, clipboard is restored) · ✅ ask about a screenshot (camera button/`/screenshot`, needs a vision model) · ✅ dictation (microphone button, local Whisper in Docker)

## Approach

Every feature is first made **fit for daily use** (real-world use finds the actual problems) before the
next stage begins. Whatever isn't a nuisance goes into the backlog instead of the next commit.

### Backlog (polish)

- [x] When launched from the terminal, the window stays hidden (Windows foreground lock): `AttachThreadInput` + `SetForegroundWindow` when `set_focus` loses (needs a real-world test)
- [x] Code highlighting in responses (highlight.js core, ~20 languages, language label + copy)
- [x] Browse archived chats: `/history`, ⋯ → *Earlier chats*, the two latest on the empty chat
- [x] Volume indicator after "louder/quieter": level bar in the compact bar
- [ ] Check whether Spotify auto-plays albums/playlists
- [ ] ~~Search: DuckDuckGo fallback~~ tested 2026-09-18: DuckDuckGo answers with a bot challenge after ~4 requests, not reliable. Alternative if needed: Brave Search API (own key)
- [x] Search: `fetch_page` tool (reads a public page as text; local/private addresses refused, also via redirect)
- [x] Web tab: Esc (if the page does not use it), Ctrl+N, Ctrl+E and Ctrl+L also work while the page has focus
- [x] Web tab: ask the AI about the open page (✨ in the toolbar → page chip above the input, "Summarize" or own question, read via `fetch_page`)
- [x] MCP: servers that need a browser login (OAuth): login in the settings, tokens in the Credential Manager
- [ ] MCP: resources and prompts (only tools are used)
- [x] MCP: send only the tools that fit the request above 8 (local) / 64 (cloud) MCP tools, word match + tools used in the chat
- [x] Slash commands in the input (like Claude Code): `/clear`, `/web`, `/translate`, `/model`, `/tools`, `/settings`, `/mcp`, …

## Setup

- [x] Git, Node LTS, Rustup, VS C++ Build Tools (winget)
- [x] Project scaffold: `create-tauri-app` (React + TS)
- [x] Docker: already available in WSL (Ubuntu), no Docker Desktop needed
