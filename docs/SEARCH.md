# Web search

Orbit searches the web through **SearXNG**, a metasearch engine running locally in Docker. It queries
Google, Bing, Brave, DuckDuckGo, Wikipedia, … at once and returns a clean JSON list: no account, no key,
no tracking. A search takes about one second.

## Setup (once)

Docker must be available in WSL (the default distro). Then:

```powershell
.\infra\searxng\start.ps1           # copies the config to ~/.orbit/searxng in WSL and starts the container
.\infra\searxng\start.ps1 -Update   # pull the newest SearXNG image first
```

The container `orbit-searxng` listens on `http://127.0.0.1:8888` (only reachable from this PC) and
restarts with Docker. Config: [`infra/searxng/settings.yml`](../infra/searxng/settings.yml); run
`start.ps1` again after changing it.

## Usage

| Input | What happens |
|---|---|
| `? what's new in tauri 2` | Result cards immediately, then a short AI answer with sources `[1]` |
| `suche im web nach …` / "search the web for …" | same |
| Any question that needs current information | The AI can call `web_search` by itself |
| `g …` | Opens Google in the browser (`google_search` tool) |

The answer only uses the excerpts of the results (fast, also with small local models). Click a card or a
citation badge to open the page in the browser.

## Settings (`settings.json`)

```json
"search": {
  "searxngUrl": "http://127.0.0.1:8888",
  "language": "auto",
  "maxResults": 6
}
```

- `language`: `auto` detects it from the query; `all`, `de`, `en-US`, … force one.
- `maxResults`: fewer results = shorter prompt = faster answers from local models.

## Troubleshooting

- *"Web search is not running"*: start the container with `start.ps1`, check `wsl -- docker ps`.
- *"SearXNG does not allow JSON results"*: `json` is missing under `search.formats` in `settings.yml`.
- Logs: `wsl -- docker logs orbit-searxng`.
