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
  tabs: {
    chat: "Chat",
    web: "Web",
  },
  web: {
    back: "Back",
    forward: "Forward",
    reload: "Reload",
    stop: "Stop loading",
    openInBrowser: "Open in browser",
    placeholder: "Search Google or enter an address",
    emptyTitle: "Search the web without leaving Orbit",
    emptyHint: "Type above, or use “g search term” anywhere in Orbit.",
  },
  webResults: {
    more_one: "Show {{count}} more result",
    more_other: "Show {{count}} more results",
    less: "Show fewer",
    summarize: "Summarize with AI",
    open: "Open {{url}}",
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
    local: "Local",
    cloud: "Cloud",
    keyMissing: "Key missing",
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
    title: "Keys",
    storageNote: "Stored securely in the Windows Credential Manager, never shown again.",
    empty: "Nothing needs a key right now.",
    sections: {
      ai: "AI providers",
      integration: "Integrations",
      custom: "Custom tools",
    },
    status: {
      active: "Active",
      notSet: "Not set up",
      partial_one: "{{count}} missing",
      partial_other: "{{count}} missing",
    },
    group: {
      testConnection: "Test connection",
      getKey: "Get a key",
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
