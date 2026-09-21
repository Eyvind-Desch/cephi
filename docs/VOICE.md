# Dictation

The microphone button next to the input turns speech into text. Recognition runs **offline** with
**Whisper** (`faster-whisper`, model `small`) in a local Docker container next to SearXNG and
LibreTranslate — nothing is sent to the internet, and no graphics card is needed.

Recording itself happens in Cephi (Windows audio, WASAPI), not in the WebView: no browser permission
prompt, and the audio never leaves the app.

## Setup

Cephi starts the container itself: if speech recognition is not running, the message comes with a
**Start service** button (the first start pulls the image and the model, a few minutes). Manually:

```powershell
.\infra\start.ps1           # starts all services, including Whisper
.\infra\start.ps1 -Update   # pull newer images first
```

The container `cephi-whisper` listens on `http://127.0.0.1:8891` (only reachable from this PC). It needs
about 1 GB of RAM while transcribing and nothing noticeable while idle; a dictated sentence takes roughly
1–3 s on the CPU.

## Usage

| Step | What happens |
|---|---|
| Click the microphone | Recording starts (the button turns into a red stop button) |
| Speak, then click stop | The recording is transcribed and the text lands in the input field |
| `Esc` while recording | The recording is thrown away |

The text is appended to whatever is already typed, so you can dictate a question and finish it by hand.
⋯ → *Settings* → *Send dictation right away* sends it instead of putting it in the input field.

Recording stops by itself after two minutes.

## Settings (`settings.json`)

```json
"voice": {
  "whisperUrl": "http://127.0.0.1:8891",
  "language": "auto",
  "sendDirectly": false
}
```

- **whisperUrl** — any host that speaks the [whisper-asr-webservice](https://github.com/ahmetoner/whisper-asr-webservice)
  API works, e.g. a stronger PC in your network.
- **language** — `auto` lets Whisper detect the language; a code like `de` or `en` is more reliable
  (and slightly faster) when you always dictate in the same language.
- **sendDirectly** — also in the settings page.

A bigger model (better with names and technical terms, slower) is a one-line change in
`infra/compose.yaml`: `ASR_MODEL: medium`. After that, `.\infra\start.ps1 -Update`.

## Troubleshooting

| Problem | Cause |
|---|---|
| "No microphone available" | Windows has no input device, or Cephi has no microphone access (Settings → Privacy & security → Microphone) |
| "Speech recognition is not running" | The container is not up: use *Start service* or `.\infra\start.ps1` |
| "Nothing recorded" | The recording was shorter than a quarter second, or the wrong input device is the default one |
| The first dictation takes long | The model is downloaded and loaded once (a few hundred MB) |
