/**
 * English UI strings — the source language. Every other locale must mirror this structure
 * (enforced by the `Locale` type in `de.ts`).
 *
 * Texts produced by the backend (tool titles, errors, diagnostics, key labels …) are localized
 * there and are not part of these files.
 */
export const en = {
  common: {
    copy: "Copy",
    copied: "Copied",
    save: "Save",
    cancel: "Cancel",
  },
  composer: {
    placeholder: "Ask me anything…",
    placeholderRunning: "Orbit is working…",
    stop: "Stop",
    send: "Send (Enter)",
  },
  emptyState: {
    title: "How can I help?",
    toolsReady_one: "{{count}} tool ready",
    toolsReady_other: "{{count}} tools ready",
    hotkeyHint: "shows and hides me",
    suggestions: {
      spotify: "Play Another Love by Tom Odell on Spotify",
      calculator: "Open the calculator",
      weather: "What's the weather in Berlin?",
      memory: "How much memory is free?",
    },
  },
  chat: {
    cancelled: "Cancelled",
    thinking: "Orbit is thinking",
  },
  toolCard: {
    status: {
      running: "Running",
      awaiting: "Waiting for approval",
      ok: "Done",
      error: "Failed",
    },
    fastPath: "Fast path: run directly, without AI",
    confirmPrompt: "Orbit wants to run the following:",
    deny: "Deny",
    run: "Run",
    enterKeys: "Enter keys",
    details: {
      tool: "Tool",
      arguments: "Arguments",
      result: "Result",
    },
    durationMs: "{{value}} ms",
    durationS: "{{value}} s",
  },
  header: {
    menu: "Menu",
    pinned: "Pinned: stays open",
    collapse: "Collapse (Ctrl+E)",
  },
  menu: {
    newChat: "New chat",
    newChatShortcut: "Ctrl+N",
    pin: "Pin",
    keys: "Keys & API keys",
    configFolder: "Config folder",
    reloadConfig: "Reload configuration",
  },
  profile: {
    label: "Model",
  },
  compact: {
    expand: "Expand (Ctrl+E)",
    problems_one: "{{count}} configuration problem – details in the panel",
    problems_other: "{{count}} configuration problems – details in the panel",
  },
  diagnostics: {
    count_one: "{{count}} configuration problem",
    count_other: "{{count}} configuration problems",
    openFolder: "Open config folder",
  },
  keys: {
    back: "Chat",
    backTitle: "Back to chat (Esc)",
    allSet: "All set",
    missing_one: "{{count}} key missing",
    missing_other: "{{count}} keys missing",
    title: "Keys",
    storageNote: "Stored in the Windows Credential Manager. Orbit never shows saved values again.",
    empty: "None of your tools needs a key right now.",
    group: {
      complete: "Set up",
      missing_one: "{{count}} missing",
      missing_other: "{{count}} missing",
      testConnection: "Test connection",
      dashboard: "Dashboard",
    },
    field: {
      change: "Change",
      remove: "Remove",
      confirmRemove: "Really remove?",
    },
  },
  keysGuide: {
    spotify: {
      title: "How to get the keys (2 minutes, Premium required)",
      step1: "Sign in to the <b>Spotify Developer Dashboard</b> (button below) with your Premium account.",
      step2: "<b>Create app</b>: name “Orbit”, redirect URI <uri/>, tick <b>Web API</b> under APIs, save.",
      step3: "In the app under <b>Settings</b>, copy the Client ID and Client Secret and paste them here.",
    },
  },
} as const;
