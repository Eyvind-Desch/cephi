# Eigene Tools schreiben

Tools sind JSON-Dateien in **`%APPDATA%\Orbit\tools\`**, eine Datei pro Tool. Unterordner zum Sortieren
sind erlaubt (`web/`, `system/`, `apps/` …). Orbit lädt Änderungen **sofort**, ein Neustart ist nicht nötig.

> Ordner öffnen: Tray-Icon → *Konfiguration öffnen*, oder das Ordner-Symbol im Panel.

Fehler (ungültiges JSON, unbekannte Felder, Tippfehler in Platzhaltern) erscheinen als gelber Hinweis
im Panel, mit Dateiname und Grund. Ein kaputtes Tool legt die anderen nicht lahm.

## Grundgerüst

```json
{
  "$schema": "../../schemas/tool.schema.json",
  "name": "github_search",
  "title": "GitHub-Suche",
  "description": "Sucht Repositories auf GitHub. Nutze dies, wenn der Nutzer Code oder Projekte auf GitHub finden möchte.",
  "permission": "auto",
  "parameters": {
    "type": "object",
    "properties": {
      "query": { "type": "string", "description": "Suchbegriff" }
    },
    "required": ["query"]
  },
  "triggers": ["gh {query}", "(suche|such) {query} (auf|bei) github"],
  "action": {
    "type": "open_url",
    "url": "https://github.com/search?type=repositories&q={{query}}"
  }
}
```

**`$schema`** sorgt in VS Code für Autovervollständigung und Fehleranzeige. Der Pfad ist relativ zur Datei:
`tools/x.json` → `../schemas/tool.schema.json`, `tools/web/x.json` → `../../schemas/tool.schema.json`.

| Feld | Pflicht | Bedeutung |
|---|---|---|
| `name` | ✔ | Technischer Name für das Modell: `a-z`, `0-9`, `_`, eindeutig |
| `title` |  | Anzeigename in der Oberfläche |
| `description` | ✔ | **Wann** das Tool benutzt wird. Das Modell entscheidet nur anhand dieses Textes: konkret formulieren! |
| `enabled` |  | `false` = Tool wird nicht geladen (Standard `true`) |
| `permission` |  | `auto` = sofort, `confirm` = vorher nachfragen (**Standard**) |
| `parameters` |  | Was das Modell ausfüllen soll (siehe unten) |
| `triggers` |  | Sätze, die das Tool **sofort ohne KI** ausführen (siehe Schnellwege) |
| `action` | ✔ | Was passiert (siehe Aktionstypen) |

## Parameter

```json
"parameters": {
  "type": "object",
  "properties": {
    "city":  { "type": "string", "description": "Stadtname" },
    "days":  { "type": "integer", "description": "Anzahl Tage", "default": 1 },
    "units": { "type": "string", "enum": ["metric", "imperial"] }
  },
  "required": ["city"]
}
```

- Typen: `string`, `number`, `integer`, `boolean`
- `description`: hilft dem Modell, den Wert richtig zu füllen
- `enum`: nur diese Werte sind erlaubt
- `default`: wird genutzt, wenn das Modell den Parameter weglässt

Orbit prüft die Argumente, bevor etwas ausgeführt wird. Bei Fehlern bekommt das Modell eine Meldung und
kann es erneut versuchen.

## Schnellwege (Trigger)

Trigger sind Sätze, bei denen Orbit **nicht** das Modell fragt, sondern das Tool direkt ausführt. Das
dauert Millisekunden statt Sekunden und funktioniert unabhängig davon, wie schlau das Modell ist.

```json
"triggers": [
  "(spiele|play) {query} (auf|in) spotify",
  "(öffne|starte) [(den|die|das)] [app] {name}",
  { "pattern": "spiele [das] album {query} auf spotify", "args": { "kind": "album" } }
]
```

| Syntax | Bedeutung | Beispiel |
|---|---|---|
| `{name}` | fängt Text ein → Parameter `name` | `öffne {name}` |
| `(a\|b)` | eine der Alternativen | `(spiele\|play)` |
| `[x]` | optional | `[die] [app]` |
| Leerzeichen | ein oder mehr Leerzeichen | |

- Groß-/Kleinschreibung ist egal, `.`, `!`, `?` am Ende werden ignoriert, Anführungszeichen um
  eingefangene Werte entfernt.
- Der **ganze** Satz muss passen. „Spiele X auf Spotify“ passt, „Kannst du X auf Spotify spielen?“ nicht:
  das geht dann ganz normal an die KI (die das Tool trotzdem aufrufen kann).
- Mit `args` setzt ein Trigger feste Werte, z. B. `"kind": "album"`.
- Passen mehrere Trigger, gewinnt der **spezifischere** (mehr feste Wörter).
- Zu allgemeine Trigger wie `{query} spotify` besser vermeiden: sie fangen auch Sätze ab, die etwas
  anderes meinen.
- Im Chat erkennst du Schnellwege am ⚡ auf der Tool-Karte.

## API-Keys ({{secret:…}})

API-Keys gehören **nicht** in die JSON-Datei. Orbit speichert sie in der Windows-Anmeldeinformationsverwaltung
und setzt sie über `{{secret:name}}` ein:

```json
"action": {
  "type": "http",
  "url": "https://api.example.com/v1/translate",
  "headers": { "Authorization": "Bearer {{secret:example_api_key}}" },
  "body": "{ \"text\": {{text}} }"
}
```

Sobald ein Tool einen Schlüssel verwendet, erscheint er im Panel unter **🔑 Schlüssel**. Dort gibst du den
Wert einmal ein. Orbit zeigt gespeicherte Werte nie wieder an und maskiert sie in Bestätigungsdialogen.
Schlüsselnamen: `a-z`, `0-9`, `_`.

## Platzhalter

In Aktionen werden Parameter mit `{{name}}` eingesetzt. Wie sie maskiert werden, hängt vom Feld ab:

| Feld | Standard | Beispiel |
|---|---|---|
| `open_url.url`, `http.url` | URL-kodiert | `Köln Dom` → `K%C3%B6ln+Dom` |
| `launch.program/args/cwd` | roh | |
| `http.headers` | roh (ohne Zeilenumbrüche) | |
| `http.body` | als JSON-Wert | `Hallo` → `"Hallo"` |

Mit einem Filter lässt sich das ändern: `{{name|raw}}`, `{{name|url}}`, `{{name|json}}`.

## Aktionstypen

### `open_url`: Webseite oder App-Link öffnen

```json
"action": { "type": "open_url", "url": "https://github.com/search?q={{query}}" }
```

Funktioniert auch mit App-Protokollen: `ms-settings:`, `spotify:`, `steam://`, `mailto:` …

**Sicherheit:** Steht das Schema fest im Template (`ms-settings:{{page|raw}}`), ist nur dieses Schema
erlaubt. Kommt die ganze URL vom Modell (`{{url|raw}}`), sind nur `http`/`https` erlaubt, also kein `file:`.

### `launch`: Programm starten

```json
"action": {
  "type": "launch",
  "program": "%LOCALAPPDATA%\\Programs\\Microsoft VS Code\\Code.exe",
  "args": ["{{path}}"]
}
```

- Startet direkt **ohne Shell**: keine Befehlsverkettung möglich
- `%UMGEBUNGSVARIABLEN%` werden ersetzt
- Argumente, die nach dem Einsetzen leer sind, fallen weg (praktisch für optionale Parameter)
- `cwd` (optional): Arbeitsverzeichnis

### `powershell`: Skript ausführen, Ausgabe ans Modell

```json
"action": {
  "type": "powershell",
  "timeoutSecs": 20,
  "script": "Get-Process -Name $env:ORBIT_ARG_NAME | Select-Object Name, CPU, WS | Format-Table | Out-String"
}
```

- Parameter kommen als **Umgebungsvariablen** an: `$env:ORBIT_ARG_<NAME IN GROSSBUCHSTABEN>`
- `{{Platzhalter}}` sind hier **absichtlich verboten**. So kann das Modell keinen eigenen Code in dein
  Skript schmuggeln (Code-Injection).
- Die Ausgabe (stdout) geht an das Modell, Fehler (stderr, Exit-Code ≠ 0) als Fehlermeldung
- Nutzt PowerShell 7 (`pwsh`), falls installiert, sonst Windows PowerShell
- Standard-Zeitlimit: 30 s

### `http`: Web-API abfragen, Antwort ans Modell

```json
"action": {
  "type": "http",
  "method": "POST",
  "url": "https://api.example.com/v1/translate",
  "headers": { "Authorization": "Bearer DEIN_KEY" },
  "body": "{ \"text\": {{text}}, \"target\": \"de\" }",
  "timeoutSecs": 15
}
```

- `method`: `GET` (Standard) oder `POST`
- `body` wird als `application/json` gesendet; Platzhalter werden automatisch zu JSON-Werten

### `builtin`: eingebaute Funktion

```json
"action": { "type": "builtin", "id": "clipboard_read" }
```

| id | Parameter | Funktion |
|---|---|---|
| `datetime` | – | Datum, Wochentag, Uhrzeit |
| `clipboard_read` | – | Text aus der Zwischenablage |
| `clipboard_write` | `text` | Text in die Zwischenablage |
| `open_app` | `name` | Installierte App starten (unscharfe Suche über Startmenü- und Store-Apps) |
| `spotify_play` | `query`, optional `kind` (`track`/`album`/`artist`/`playlist`) | Musik in Spotify abspielen, siehe [SPOTIFY.md](SPOTIFY.md) |
| `media_control` | `action` (`play_pause`/`next`/`previous`/`volume_up`/`volume_down`/`mute`) | Medientasten, funktioniert mit jedem Player |

`open_app` nimmt den Schnellweg nur, wenn eine App eindeutig passt. Sonst darf die KI übernehmen.

## Tipps für gute Tools

- **Beschreibung ist alles.** Kleine Modelle wählen Tools nur anhand von `description`. Nenne typische
  Formulierungen: *„Nutze dies, wenn der Nutzer … möchte“*.
- **Wenige, klare Parameter.** Lieber zwei spezifische Tools als eins mit fünf optionalen Parametern.
- **`enum` statt freiem Text**, wo es geht: weniger Fehler.
- **`confirm` für alles, was etwas verändert** (löschen, senden, herunterfahren, Befehle).
- Zu viele Tools verwirren kleine Modelle. Deaktiviere, was du nicht brauchst (`"enabled": false`).

## Tool deaktivieren oder zurücksetzen

- Deaktivieren: `"enabled": false`
- Löschen: Datei entfernen. Gelöschte Standard-Tools kommen **nicht** zurück.
- Alle Standard-Tools wiederherstellen: Ordner `tools` umbenennen und Orbit neu starten.
