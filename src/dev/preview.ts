/**
 * Design preview: runs the UI in a normal browser with sample data (dev server only).
 *
 * Open http://localhost:1420/?mode=expanded&view=keys&lang=de
 *   mode: compact | expanded | large   view: chat | keys | web | translate   chat: demo | results   lang: en | de   keys: some
 *   draft: text in the composer (e.g. "/" shows the commands)   web: page (web tab with a loaded page)   service: stopped (translator container down)   selection: 1 (text picked up from another app)
 *
 * Used to review UI changes (and take headless screenshots) without touching the running app.
 */
import { mockIPC } from "@tauri-apps/api/mocks";
import type { ConfigSnapshot, SecretGroup } from "../lib/ipc";
import { useChat } from "../store/chat";
import type { Message } from "../store/chatModel";
import { useTranslate } from "../store/translate";
import { useUi, type PanelView } from "../store/ui";

const params = new URLSearchParams(location.search);
const lang = params.get("lang") === "en" ? "en" : "de";
const someKeys = params.get("keys") === "some";

const config: ConfigSnapshot = {
  language: lang,
  configDir: "C:\\Users\\you\\AppData\\Roaming\\Orbit",
  hotkey: "Ctrl+Alt+Space",
  defaultProfile: "fast",
  profiles: [
    profile("fast", "Schnell", "assistant-fast:latest", "ollama", "Ollama", true, true),
    profile("smart", "Smart", "assistant:latest", "ollama", "Ollama", true, true),
    profile("deep", "Gründlich", "assistant-big:latest", "ollama", "Ollama", true, true),
    profile("claude", "Claude", "claude-opus-5", "anthropic", "Anthropic", false, someKeys),
    profile("gemini", "Gemini", "gemini-3.8-flash", "gemini", "Google Gemini", false, false),
  ],
  tools: [
    { name: "spotify_play", title: "Spotify", description: "Plays music on Spotify: a song, an album, an artist or a playlist.", permission: "auto", kind: "builtin", source: "tools/media/spotify_play.json", triggers: ["(spiele|play) {query} (auf|on) spotify"] },
    { name: "web_search", title: "Websuche", description: "Searches the web and returns the top results with short excerpts.", permission: "auto", kind: "builtin", source: "tools/web/web_search.json", triggers: ["?{query}", "search the web for {query}"] },
    { name: "run_powershell", title: "PowerShell", description: "Runs a PowerShell command.", permission: "confirm", kind: "powershell", source: "tools/system/run_powershell.json", triggers: [] },
  ],
  diagnostics: [],
};

const aiDescription =
  lang === "de"
    ? "KI-Modelle in der Cloud. Profile mit diesem Anbieter sind nutzbar, sobald der Key gespeichert ist."
    : "Cloud AI models. Profiles using this provider work once the key is saved.";

const secretGroups: SecretGroup[] = [
  aiGroup("anthropic", "Anthropic", "anthropic", ["Claude"], someKeys),
  aiGroup("gemini", "Google Gemini", "gemini", ["Gemini"], false),
  aiGroup("openai", "OpenAI", "openai", [], false),
  {
    id: "spotify",
    title: "Spotify",
    description: lang === "de" ? "Songs per Befehl abspielen." : "Play songs by command.",
    icon: "music",
    category: "integration",
    docsUrl: "https://developer.spotify.com/dashboard",
    testable: true,
    secrets: [
      secret("spotify_client_id", "Client ID", false, true, ["Spotify"]),
      secret("spotify_client_secret", "Client Secret", true, true, ["Spotify"]),
    ],
  },
];

const demoChat: Message[] = [
  { id: "1", role: "user", text: "Wie ist das Wetter in Hamburg?" },
  {
    id: "2",
    role: "assistant",
    status: "done",
    parts: [
      {
        kind: "tool",
        callId: "c1",
        name: "weather",
        title: "Wetter",
        args: { location: "Hamburg" },
        status: "ok",
        fast: false,
        output: "Hamburg: ☁️ +11°C",
        durationMs: 412,
      },
      { kind: "text", text: "In Hamburg sind es gerade **11 °C** und bewölkt." },
    ],
  },
  { id: "3", role: "user", text: "spiele another love von tom odell auf spotify" },
  {
    id: "4",
    role: "assistant",
    status: "done",
    parts: [
      {
        kind: "tool",
        callId: "c2",
        name: "spotify_play",
        title: "Spotify",
        args: { query: "another love tom odell" },
        status: "ok",
        fast: true,
        output: "Spielt in Spotify: Another Love – Tom Odell",
        durationMs: 238,
      },
    ],
  },
  { id: "5", role: "user", text: "? tauri 2 neueste version" },
  {
    id: "6",
    role: "assistant",
    status: "done",
    parts: [
      {
        kind: "tool",
        callId: "c3",
        name: "web_search",
        title: "Websuche",
        args: { query: "tauri 2 neueste version" },
        status: "ok",
        fast: true,
        output: "…",
        durationMs: 912,
        display: {
          kind: "webResults",
          query: "tauri 2 neueste version",
          results: [
            webResult("Tauri 2.0 Release", "https://v2.tauri.app/blog/tauri-20/", "Tauri 2.0 ist stabil: mobile Unterstützung für iOS und Android, ein neues Berechtigungssystem und Plugins."),
            webResult("Releases · tauri-apps/tauri", "https://github.com/tauri-apps/tauri/releases", "Build smaller, faster, and more secure desktop and mobile applications with a web frontend."),
            webResult("Tauri – Wikipedia", "https://de.wikipedia.org/wiki/Tauri_(Framework)", "Tauri ist ein Framework zur Entwicklung plattformübergreifender Anwendungen mit Webtechnologien."),
            webResult("What's new in Tauri", "https://v2.tauri.app/release/", "Changelog of all Tauri packages."),
          ],
        },
      },
      {
        kind: "text",
        text: "Die aktuelle Hauptversion ist **Tauri 2** [1]. Einzelne Pakete bekommen laufend Patch-Releases, die Liste steht auf GitHub [2][4].",
      },
    ],
  },
  { id: "7", role: "user", text: "tr nevertheless" },
  {
    id: "8",
    role: "assistant",
    status: "done",
    parts: [
      {
        kind: "tool",
        callId: "c4",
        name: "translate",
        title: "Übersetzen",
        args: { text: "nevertheless" },
        status: "ok",
        fast: true,
        output: "dennoch (trotzdem, gleichwohl)",
        durationMs: 41,
        display: { kind: "translation", source: "nevertheless", text: "dennoch", alternatives: ["trotzdem", "gleichwohl"], from: "en", to: "de" },
      },
    ],
  },
  { id: "9", role: "user", text: "tr awkard" },
  {
    id: "10",
    role: "assistant",
    status: "done",
    parts: [
      {
        kind: "tool",
        callId: "c5",
        name: "translate",
        title: "Übersetzen",
        args: { text: "awkard" },
        status: "ok",
        fast: true,
        output: "awkard",
        durationMs: 114,
        // A typo: LibreTranslate returns unknown words unchanged.
        display: { kind: "translation", source: "awkard", text: "awkard", alternatives: [], from: "en", to: "de" },
      },
    ],
  },
];

function webResult(title: string, url: string, snippet: string) {
  return { title, url, snippet, engines: ["google", "brave"] };
}

export function installPreview() {
  mockIPC(
    (cmd) => {
      switch (cmd) {
        case "config_snapshot":
        case "config_reload":
          return config;
        case "secrets_list":
          return secretGroups;
        case "secrets_test":
          return lang === "de" ? "Verbindung funktioniert." : "Connection works.";
        case "translate":
          if (params.get("service") === "stopped") throw new Error("Der Übersetzer läuft nicht (LibreTranslate unter http://127.0.0.1:8890).");
          return { source: "nevertheless", text: "dennoch", alternatives: ["trotzdem", "gleichwohl"], from: "en", to: "de" };
        case "services_status":
          return [
            { id: "search", title: "Websuche", running: true },
            { id: "translate", title: "Übersetzer", running: params.get("service") !== "stopped" },
          ];
        case "autostart_state":
          return { enabled: false, available: true };
        case "chat_load":
          return null;
        case "web_state":
          return params.get("web") === "page"
            ? { url: "https://www.google.com/search?q=tauri+child+webview", title: "tauri child webview - Google Suche", loading: true }
            : { url: null, title: "", loading: false };
        default:
          return null;
      }
    },
    { shouldMockEvents: true },
  );

  document.documentElement.dataset.preview = "true";
  useUi.setState({
    mode: params.get("mode") === "compact" ? "compact" : params.get("mode") === "large" ? "large" : "expanded",
    webLarge: params.get("mode") === "large",
    view: (params.get("view") as PanelView | null) ?? "chat",
    selection:
      params.get("selection") === "1"
        ? "Nevertheless, the committee agreed that the proposal was somewhat awkward to implement."
        : null,
  });
  if (params.get("chat") === "demo") useChat.setState({ messages: demoChat });
  if (params.get("draft")) useChat.setState({ draft: params.get("draft") ?? "" });
  if (params.get("view") === "translate") useTranslate.setState({ text: params.get("text") ?? "renovated" });
  // Search results without an AI answer (model unavailable).
  if (params.get("chat") === "results") {
    const [question, answer] = demoChat.slice(-2);
    if (answer.role === "assistant") {
      const notice =
        lang === "de"
          ? "Keine KI-Antwort: Ollama ist nicht erreichbar (http://localhost:11434)."
          : "No AI answer: Ollama is not reachable (http://localhost:11434).";
      useChat.setState({ messages: [question, { ...answer, parts: answer.parts.slice(0, 1), notice }] });
    }
  }

  // ?open=<text> clicks the first expandable row / button containing that text after rendering.
  const open = params.get("open");
  if (open) {
    setTimeout(() => {
      const target = [...document.querySelectorAll<HTMLElement>("button")].find((b) => b.textContent?.includes(open));
      target?.click();
    }, 1500);
  }
}

function profile(
  id: string,
  label: string,
  model: string,
  provider: string,
  providerLabel: string,
  local: boolean,
  available: boolean,
) {
  const providerIcon = local ? "ollama" : provider;
  return { id, label, model, tools: true, provider, providerLabel, providerIcon, local, available };
}

function aiGroup(id: string, title: string, icon: string, usedBy: string[], isSet: boolean): SecretGroup {
  return {
    id,
    title,
    description: aiDescription,
    icon,
    category: "ai",
    docsUrl: "https://example.com",
    testable: true,
    secrets: [secret(`${id}_api_key`, "API Key", true, isSet, usedBy)],
  };
}

function secret(name: string, label: string, sensitive: boolean, isSet: boolean, usedBy: string[]) {
  return { name, label, sensitive, placeholder: lang === "de" ? "Wert einfügen" : "Paste value", isSet, usedBy };
}
