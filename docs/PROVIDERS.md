# AI providers

Orbit talks to three kinds of model backends:

| Type | Used for | Protocol |
|---|---|---|
| `ollama` (built in) | local models | Ollama `/api/chat` |
| `anthropic` | Claude | Anthropic Messages API |
| `openai` | OpenAI and compatible APIs: Google Gemini, OpenRouter, Groq, LM Studio, … | Chat Completions |

Profiles pick a provider and a model; the model picker in the panel header lists them under
**Local** and **Cloud**. Cloud profiles show *Key missing* until their API key is saved.

## Adding a key

Panel → ⋯ → **Keys & API keys** → the provider's card → paste the key → *Save* → *Test connection*.
Keys are stored in the Windows Credential Manager as `Orbit/<providerId>_api_key`, never in files.

| Provider | Where to get a key |
|---|---|
| Anthropic | [console.anthropic.com/settings/keys](https://console.anthropic.com/settings/keys) |
| Google Gemini | [aistudio.google.com/apikey](https://aistudio.google.com/apikey) |
| OpenAI | [platform.openai.com/api-keys](https://platform.openai.com/api-keys) |

## Configuration (`settings.json`)

```json
"providers": [
  { "id": "anthropic", "type": "anthropic", "label": "Anthropic" },
  { "id": "gemini", "type": "openai", "label": "Google Gemini",
    "baseUrl": "https://generativelanguage.googleapis.com/v1beta/openai" },
  { "id": "openrouter", "type": "openai", "label": "OpenRouter", "baseUrl": "https://openrouter.ai/api/v1" }
],
"profiles": [
  { "id": "fast", "label": "Fast", "model": "assistant-fast:latest" },
  { "id": "claude", "label": "Claude", "provider": "anthropic", "model": "claude-opus-5", "effort": "low" },
  { "id": "gemini", "label": "Gemini", "provider": "gemini", "model": "gemini-3.8-flash" }
]
```

| Provider field | Meaning |
|---|---|
| `id` | Referenced by profiles; also names the key (`<id>_api_key`). Not `ollama`. |
| `type` | `anthropic` or `openai` |
| `label` | Display name, plain or `{ "en": …, "de": … }` |
| `baseUrl` | Required for `openai` |
| `apiKeySecret` | Custom key name (default `<id>_api_key`) |
| `maxTokens` | Output limit per answer (Anthropic default 64000) |
| `fallbacks` | Anthropic, default `true`: see below |

| Profile field | Meaning |
|---|---|
| `provider` | `ollama` (default) or a provider id |
| `effort` | Anthropic: `low` … `max`. `low` keeps the assistant fast; raise it for hard questions. |
| `think` | Ollama only |
| `temperature` | Ollama and `openai` only (current Claude models reject it) |

## Provider details

**Anthropic**
- Streaming Messages API with tool use; tool inputs stream eagerly and are validated before any tool runs.
- Thinking blocks are passed back unchanged when the conversation continues on the same model.
- For Claude Opus 5 / Fable 5, requests use server-side refusal fallbacks (`fallbacks: "default"`):
  a declined request is re-run on Anthropic's recommended fallback model inside the same call.
  Disable per provider with `"fallbacks": false`.
- If the output limit cuts off a tool call, the tool is not run.

**OpenAI-compatible**
- Tool calls are stored exactly as the API returned them and sent back unchanged, which keeps
  provider extras such as Gemini thought signatures intact.
- Tool parameter schemas are reduced to `type`, `description`, `enum` and `required`, which every
  compatible API accepts.

## Speed

Local fast-path triggers never reach a model. For questions, the local `fast` profile answers
without network latency; cloud profiles are usually smarter and, on a weak GPU, often faster too.
