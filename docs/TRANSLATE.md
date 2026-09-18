# Translator

Orbit translates offline with **LibreTranslate** (Argos Translate models), running locally in Docker next to
SearXNG. No AI model and no graphics card are needed: a translation takes about 0.05–0.5 s on the CPU.
Only German and English are loaded (about 1.6 GB RAM in the container, no CPU load while idle).

## Setup (once)

```powershell
.\infra\start.ps1           # starts SearXNG and LibreTranslate (first start downloads the models)
.\infra\start.ps1 -Update   # pull newer images first
```

The container `orbit-libretranslate` listens on `http://127.0.0.1:8890` (only reachable from this PC).

## Usage

| Input | What happens |
|---|---|
| `tr nevertheless` | Translation right in the compact bar, with a copy button |
| `übersetze Kannst du mir helfen?` / "translate …" | same |
| `übersetze guten Morgen ins Englische` / "translate … into German" | fixed target language |
| Select text anywhere + `Ctrl+Alt+T` | The text appears above the input; *Translate* opens it in the translator |
| Translator tab (文A icon) | type or paste, the translation follows while typing; alternatives as chips (click = copy) |

**Direction:** German text is translated into English, everything else into German. In the tab you can
also force "→ English" or "→ German". For single words LibreTranslate also returns alternatives
("dennoch · trotzdem · gleichwohl").

**Unknown words:** LibreTranslate returns words it does not know (typos, rare terms) unchanged. Orbit detects
that, says so, and offers **Translate with AI**, which also fixes typos ("awkard" → "awkward" → "unbeholfen").

A click on the translation card in the chat opens it in the translator tab.

The AI can call the `translate` tool too. For things a dictionary can't do ("say this more politely",
"what's the difference between awkward and embarrassing") just ask in the chat.

## Settings (`settings.json`)

```json
"translate": {
  "libretranslateUrl": "http://127.0.0.1:8890",
  "nativeLanguage": "de",
  "foreignLanguage": "en"
}
```

More languages: add them to `LT_LOAD_ONLY` in [`infra/compose.yaml`](../infra/compose.yaml) (e.g. `en,de,es,fr`)
and run `start.ps1` again. `übersetze … ins Spanische` then works too.

## Troubleshooting

- *"The translator is not running"*: `.\infra\start.ps1`, check `wsl -- docker ps`.
- Logs: `wsl -- docker logs orbit-libretranslate`.
