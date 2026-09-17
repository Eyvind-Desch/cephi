# Writing custom tools

Tools are JSON files in **`%APPDATA%\Orbit\tools\`**, one file per tool. Subfolders for organizing
are allowed (`web/`, `system/`, `apps/`, …). Orbit loads changes **immediately**; a restart isn't necessary.

> Open the folder: tray icon → *Open configuration*, or the folder icon in the panel.

Errors (invalid JSON, unknown fields, typos in placeholders) appear as a yellow notice
in the panel, with the file name and reason. A broken tool doesn't take down the others.

## Basic structure

```json
{
  "$schema": "../../schemas/tool.schema.json",
  "name": "github_search",
  "title": { "en": "GitHub Search", "de": "GitHub-Suche" },
  "description": "Searches repositories on GitHub. Use this when the user wants to find code or projects on GitHub.",
  "permission": "auto",
  "parameters": {
    "type": "object",
    "properties": {
      "query": { "type": "string", "description": "Search term" }
    },
    "required": ["query"]
  },
  "triggers": ["gh {query}", "(suche|such|search) {query} (auf|bei|on) github"],
  "action": {
    "type": "open_url",
    "url": "https://github.com/search?type=repositories&q={{query}}"
  }
}
```

**`$schema`** gives you autocompletion and error highlighting in VS Code. The path is relative to the file:
`tools/x.json` → `../schemas/tool.schema.json`, `tools/web/x.json` → `../../schemas/tool.schema.json`.

| Field | Required | Meaning |
|---|---|---|
| `name` | ✔ | Technical name for the model: `a-z`, `0-9`, `_`, unique |
| `title` |  | Display name in the UI. Plain string or localized object: `{ "en": "Weather", "de": "Wetter" }` |
| `description` | ✔ | **When** the tool is used. The model decides based on this text alone: be specific! Write it in English — small models pick tools more reliably that way. |
| `enabled` |  | `false` = tool isn't loaded (default `true`) |
| `permission` |  | `auto` = runs immediately, `confirm` = asks first (**default**) |
| `parameters` |  | What the model should fill in (see below) |
| `triggers` |  | Phrases that run the tool **immediately, without AI** (see Quick actions). Can include phrases in multiple languages. |
| `action` | ✔ | What happens (see Action types) |

## Parameters

```json
"parameters": {
  "type": "object",
  "properties": {
    "city":  { "type": "string", "description": "City name" },
    "days":  { "type": "integer", "description": "Number of days", "default": 1 },
    "units": { "type": "string", "enum": ["metric", "imperial"] }
  },
  "required": ["city"]
}
```

- Types: `string`, `number`, `integer`, `boolean`
- `description`: helps the model fill in the value correctly
- `enum`: only these values are allowed
- `default`: used when the model omits the parameter

Orbit validates the arguments before executing anything. On errors, the model gets a message and
can try again.

## Quick actions (triggers)

Triggers are phrases where Orbit **doesn't** ask the model but runs the tool directly. This
takes milliseconds instead of seconds and works regardless of how smart the model is.

```json
"triggers": [
  "(spiele|play) {query} (auf|in) spotify",
  "(öffne|starte) [(den|die|das)] [app] {name}",
  { "pattern": "spiele [das] album {query} auf spotify", "args": { "kind": "album" } }
]
```

| Syntax | Meaning | Example |
|---|---|---|
| `{name}` | captures text → parameter `name` | `öffne {name}` |
| `(a\|b)` | one of the alternatives | `(spiele\|play)` |
| `[x]` | optional | `[die] [app]` |
| Space | one or more spaces | |

- Case doesn't matter, trailing `.`, `!`, `?` are ignored, quotation marks around
  captured values are stripped.
- The **entire** sentence must match. "Spiele X auf Spotify" ("Play X on Spotify") matches, but "Kannst du X auf Spotify spielen?" ("Can you play X on Spotify?") doesn't:
  that then goes to the AI as normal (which can still call the tool).
- With `args`, a trigger sets fixed values, e.g. `"kind": "album"`.
- If several triggers match, the **more specific** one wins (more fixed words).
- Avoid overly generic triggers like `{query} spotify`: they also catch sentences that mean
  something else.
- In the chat, you can spot quick actions by the ⚡ on the tool card.
- `"afterTrigger": "answer"` lets the AI answer from the result after a quick action (default `"show"`:
  only the result). `web_search` uses it: the result cards appear immediately, the summary follows.

## API keys (`{{secret:…}}`)

API keys don't belong **in** the JSON file. Orbit stores them in the Windows Credential Manager
and inserts them via `{{secret:name}}`:

```json
"action": {
  "type": "http",
  "url": "https://api.example.com/v1/translate",
  "headers": { "Authorization": "Bearer {{secret:example_api_key}}" },
  "body": "{ \"text\": {{text}} }"
}
```

As soon as a tool uses a key, it appears in the panel under **🔑 Keys**. There you enter the
value once. Orbit never shows stored values again and masks them in confirmation dialogs.
Key names: `a-z`, `0-9`, `_`.

## Placeholders

In actions, parameters are inserted with `{{name}}`. How they're escaped depends on the field:

| Field | Default | Example |
|---|---|---|
| `open_url.url`, `http.url` | URL-encoded | `Köln Dom` → `K%C3%B6ln+Dom` |
| `launch.program/args/cwd` | raw | |
| `http.headers` | raw (no line breaks) | |
| `http.body` | as JSON value | `Hello` → `"Hello"` |

This can be changed with a filter: `{{name|raw}}`, `{{name|url}}`, `{{name|json}}`.

## Action types

### `open_url`: open a website or app link

```json
"action": { "type": "open_url", "url": "https://github.com/search?q={{query}}" }
```

Also works with app protocols: `ms-settings:`, `spotify:`, `steam://`, `mailto:`, …

**Security:** If the scheme is fixed in the template (`ms-settings:{{page|raw}}`), only that scheme
is allowed. If the whole URL comes from the model (`{{url|raw}}`), only `http`/`https` are allowed — no `file:`.

### `launch`: start a program

```json
"action": {
  "type": "launch",
  "program": "%LOCALAPPDATA%\\Programs\\Microsoft VS Code\\Code.exe",
  "args": ["{{path}}"]
}
```

- Starts directly **without a shell**: no command chaining possible
- `%ENVIRONMENT_VARIABLES%` are expanded
- Arguments that are empty after substitution are dropped (handy for optional parameters)
- `cwd` (optional): working directory

### `powershell`: run a script, output goes to the model

```json
"action": {
  "type": "powershell",
  "timeoutSecs": 20,
  "script": "Get-Process -Name $env:ORBIT_ARG_NAME | Select-Object Name, CPU, WS | Format-Table | Out-String"
}
```

- Parameters arrive as **environment variables**: `$env:ORBIT_ARG_<NAME IN UPPERCASE>`
- `{{placeholders}}` are **deliberately forbidden** here. This way the model can't smuggle its own code into your
  script (code injection).
- Output (stdout) goes to the model; errors (stderr, exit code ≠ 0) as an error message
- Uses PowerShell 7 (`pwsh`) if installed, otherwise Windows PowerShell
- Default timeout: 30 s

### `http`: query a web API, response goes to the model

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

- `method`: `GET` (default) or `POST`
- `body` is sent as `application/json`; placeholders are automatically converted to JSON values

### `builtin`: built-in function

```json
"action": { "type": "builtin", "id": "clipboard_read" }
```

| id | Parameters | Function |
|---|---|---|
| `datetime` | – | Date, weekday, time |
| `clipboard_read` | – | Text from the clipboard |
| `clipboard_write` | `text` | Text to the clipboard |
| `open_app` | `name` | Launch an installed app (fuzzy search over Start menu and Store apps) |
| `spotify_play` | `query`, optional `kind` (`track`/`album`/`artist`/`playlist`) | Play music on Spotify, see [SPOTIFY.md](SPOTIFY.md) |
| `media_control` | `action` (`play_pause`/`next`/`previous`/`set_volume`/`volume_up`/`volume_down`/`mute`/`unmute`), optional `level` (0–100), `amount` | Playback via media keys (any player), exact volume via Windows Core Audio |
| `radio` | `device` (`bluetooth`/`wlan`/`wifi`), `state` (`on`/`off`/`toggle`/`status`) | Turn Bluetooth/Wi-Fi on or off, toggle it, or check its status |
| `web_search` | `query` | Web search via the local SearXNG (`settings.search`), result cards in the chat, excerpts for the AI, see [SEARCH.md](SEARCH.md) |

Numeric parameters also understand `30%`, `30 %`, `30 Prozent` ("30 percent"), and `2,5` (comma as decimal separator).

`open_app` only takes the quick-action path when an app matches unambiguously. Otherwise the AI takes over.

## Tips for good tools

- **Description is everything.** Small models choose tools based on `description` alone. Mention typical
  phrasings: *"Use this when the user wants to …"*.
- **Few, clear parameters.** Prefer two specific tools over one with five optional parameters.
- **`enum` instead of free text** wherever possible: fewer errors.
- **`confirm` for anything that changes something** (delete, send, shut down, commands).
- Too many tools confuse small models. Disable what you don't need (`"enabled": false`).

## Disabling or resetting a tool

- Disable: `"enabled": false`
- Delete: remove the file. Deleted default tools do **not** come back.
- Restore all default tools: rename the `tools` folder and restart Orbit.
