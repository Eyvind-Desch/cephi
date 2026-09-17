# Orbit – Plan

Immer verfügbarer Desktop-Assistent für Windows: KI fragen, im Web suchen und Dinge am PC erledigen,
ohne ein extra Fenster zu öffnen. Läuft komplett lokal mit Ollama.

## Entscheidungen

| Thema | Entscheidung | Warum |
|---|---|---|
| App-Framework | **Tauri 2** (Rust + React/TS) | ~10 MB, wenig RAM, WebView2 ist in Windows schon drin, Mica/Tray/Hotkey als Plugins. Eigene UI ohne Grenzen. |
| Nicht: PowerToys-Erweiterung (C#) | – | Die Command Palette gibt Listen-UI vor: kein freies Chat-Panel, kein eingebettetes Google, kein eigenes Design. Als *zusätzlicher* Einstieg später denkbar. |
| UI | React + TypeScript + Tailwind + Framer Motion | Modern, schnelle Iteration, flüssige Animationen |
| KI | Ollama (`localhost:11434`) | Modelle schon vorhanden |
| Websuche | **SearXNG** (Docker), DuckDuckGo als Notlösung | Siehe unten |
| Tools/Aktionen | Eingebaute Tools + **MCP-Client** (Rust, `rmcp`) | Aktionen wie „öffne Spotify“ plus beliebige MCP-Server |

### SearXNG vs. DuckDuckGo

- **DuckDuckGo** hat keine offizielle Such-API. Man liest die HTML-Seite aus: das wird schnell
  gebremst (Rate-Limit/CAPTCHA), liefert nur eine Quelle und kann jederzeit kaputtgehen.
- **SearXNG** läuft lokal in Docker, fragt Google, Bing, Brave, DuckDuckGo, Wikipedia, GitHub usw.
  gleichzeitig ab, hat eine saubere JSON-API, kein Konto, kein Key, keine Kosten, kein Tracking.
  → **Deutlich besser.** Docker wird später auch für andere Projekte nützlich.
- DuckDuckGo bleibt als Notlösung, wenn der SearXNG-Container nicht läuft.

## UX

```
 versteckt ──Hotkey──▶  kompakt (Leiste unten rechts)  ──Tab/Klick──▶  groß (Panel)
  (Tray-Icon)          ┌──────────────────────────┐          ┌─────────────┬──────────────┐
                       │ ◯ Frag mich was…      ⏎  │          │ Chat        │ Web / Google │
                       └──────────────────────────┘          │ + Quellen   │ (Webview)    │
                                                             └─────────────┴──────────────┘
```

- Globaler Hotkey `Ctrl+Alt+Space`, einstellbar (nicht `Ctrl+Space`: kollidiert mit IntelliSense in VS Code)
- `Esc` versteckt; Klick nach außen blendet aus, außer das Fenster ist angepinnt
- Mica/Acryl-Hintergrund, Hell/Dunkel folgt dem System, abgerundete Ecken, weiche Übergänge
- Antworten live gestreamt, Markdown mit Code-Highlighting, Kopieren-Button
- Tray-Icon, Autostart

### Eingabe: ein Feld für alles

| Eingabe | Was passiert |
|---|---|
| `öffne spotify`, `spotify` | **Schnellweg ohne KI:** unscharfe Suche über Startmenü-Apps, startet sofort |
| `g katzen` | Google im Web-Panel |
| `? wie wird das wetter morgen` | KI mit Websuche → Antwort mit Quellen-Karten |
| alles andere | KI-Chat; die KI darf Tools aufrufen |
| `/` | Befehlsliste (Modell wechseln, neuer Chat, Einstellungen …) |

## Architektur

```
┌──────────────── Frontend (React/TS, WebView2) ────────────────┐
│ Compact-Bar · Chat-Panel · Web-Panel · Einstellungen          │
└──────────────── Tauri IPC (commands + events) ────────────────┘
┌──────────────────────── Rust-Core ────────────────────────────┐
│ window     Hotkey, Tray, Zustände, Mica, Always-on-top        │
│ llm        Ollama-Client (Streaming, Tool-Calls)              │
│ agent      Schleife: Modell → Tool-Call → Ergebnis → Modell   │
│ tools      Registry: eingebaute Tools + MCP-Tools             │
│ mcp        MCP-Client (rmcp), startet Server aus mcp.json     │
│ launcher   Index: Startmenü-.lnk + Store-Apps, Fuzzy-Suche    │
│ search     SearXNG-Client, DDG-Notlösung, Seiten → Text       │
│ store      Einstellungen + Chatverlauf (SQLite)               │
└───────────────────────────────────────────────────────────────┘
        │                    │                     │
   Ollama :11434      SearXNG :8888 (Docker)   MCP-Server (stdio/http)
```

## Tools / Aktionen

Tools sind **JSON-Dateien** in `%APPDATA%\Orbit\tools\` mit JSON-Schema (Autovervollständigung im Editor)
und Hot-Reload. Aktionstypen: `open_url`, `launch`, `powershell`, `http`, `builtin`. Details: [docs/TOOLS.md](docs/TOOLS.md).

Mitgeliefert: `google_search`, `youtube_search`, `open_website`, `weather`, `datetime`,
`clipboard_read`, `clipboard_write`, `open_explorer`, `open_settings`, `system_info`,
`lock_pc` (confirm), `run_powershell` (confirm).

Geplant: `open_app` (Launcher-Index), `web_search` + `fetch_page` (SearXNG), `find_files`.

**Sicherheitsstufen:** `auto` / `confirm` (Standard), pro Tool, später auch für MCP-Tools.

**MCP:** `mcp.json` im gleichen Format wie Claude Desktop (`mcpServers`), damit vorhandene
Server-Konfigurationen direkt funktionieren.

### Modelle

| Aufgabe | Modell |
|---|---|
| Schnelle Antworten | `assistant-fast` (qwen2.5 3B) |
| Tools & Websuche | `assistant` (qwen3 4B) – kann besser mit Tools umgehen |
| Gründlich | `assistant-big` (gemma4 26B), per Schalter |

Kleine Modelle werden mit vielen Tools unzuverlässig: pro Anfrage nur die passenden ~8 Tools
mitschicken. Eindeutige Befehle („öffne X“) laufen über den Schnellweg ohne KI.

## Etappen

1. ✅ **MVP** – Tauri-Gerüst, Hotkey, Tray, kompakte Leiste ↔ großes Panel, Ollama-Chat mit Streaming,
   Modell-Profile, Mica, Hell/Dunkel
2. ✅ **Aktionen** – JSON-Tool-System mit Schema, Validierung, Hot-Reload, Tool-Calling-Schleife,
   Bestätigungsdialoge · Schnellweg-Trigger ohne KI · App-Launcher · Spotify (Web-API-Suche + URI) ·
   Medientasten · Schlüssel-Tresor (Windows Credential Manager) · Modell-Preload
2b. **Modell-Anbieter** – `providers` in `settings.json`: Ollama, Anthropic, Gemini, OpenAI-kompatibel;
   Profile verweisen auf einen Anbieter; API-Keys über den Schlüssel-Tresor
3. **Web** – Entschieden: *beides*. Ergebnis-Karten sofort (SearXNG) + KI-Kurzfassung nur aus den
   Auszügen (schnell), dazu ein Google-Tab im Panel
4. **MCP** – `mcp.json`, Tools aus MCP-Servern
5. **Feinschliff** – Einstellungs-UI, Verlauf speichern, Code-Highlighting, Autostart, Installer
6. **Extras** – markierten Text übernehmen, Screenshot fragen (Bild-Modell), Sprache

## Setup

- [x] Git, Node LTS, Rustup, VS C++ Build Tools (winget)
- [x] Projekt-Gerüst: `create-tauri-app` (React + TS)
- [ ] Docker Desktop (ab Etappe 3)
