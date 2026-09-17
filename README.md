# Orbit

> Immer verfügbarer Desktop-Assistent für Windows: KI fragen, im Web suchen und Dinge am PC erledigen.
> Läuft lokal mit Ollama.

- **Category:** tools
- **Created:** 2026-09-16
- **Status:** wip <!-- idea | wip | usable | done | abandoned -->

## Why

Nicht mehr extra ein Browserfenster öffnen, um etwas zu suchen oder die KI zu fragen.
Ein Hotkey, ein Eingabefeld: Antwort, Suche oder Aktion („öffne die Bluetooth-Einstellungen“).

## Bedienung

| Taste | Aktion |
|---|---|
| `Strg+Alt+Leertaste` | Orbit ein-/ausblenden (einstellbar in `settings.json`) |
| `Enter` / `Shift+Enter` | Senden / neue Zeile |
| `Strg+E` | Kompakte Leiste ↔ großes Panel |
| `Strg+N` | Neuer Chat |
| `Esc` | Ausblenden |

Klick außerhalb blendet Orbit aus, außer es ist angepinnt. Tray-Icon: öffnen, Konfiguration, beenden.

## Konfiguration

Alles liegt in **`%APPDATA%\Orbit\`** und wird bei Änderungen sofort neu geladen:

```
Orbit/
├── settings.json   Hotkey, Modell-Profile, Fenster, Agent
├── tools/          eigene Tools als JSON → docs/TOOLS.md
└── schemas/        für Autovervollständigung im Editor (wird automatisch erzeugt)
```

## Entwicklung

Voraussetzungen: Node 22+, Rust (MSVC), VS C++ Build Tools, Ollama.

```powershell
npm install
npm run app          # Entwicklungsmodus mit Hot-Reload
npm run check        # TypeScript + Clippy + Rust-Tests
npm run app:build    # Installer (NSIS) → src-tauri/target/release/bundle/
```

Eigener Config-Ordner zum Testen: `$env:ORBIT_CONFIG_DIR = "X:\tmp\orbit-config"`.
Logs: `%LOCALAPPDATA%\io.github.eyvind-desch.orbit\logs\`.

## Docs

- [PLAN.md](PLAN.md): Ziele, Entscheidungen, Etappen
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md): Aufbau und Datenfluss
- [docs/TOOLS.md](docs/TOOLS.md): eigene Tools schreiben
