import type { en } from "./en";

/** Same keys as the English source, every leaf a string: missing or extra keys fail to compile. */
export type Locale<T = typeof en> = { [K in keyof T]: T[K] extends string ? string : Locale<T[K]> };

export const de: Locale = {
  common: {
    copy: "Kopieren",
    copied: "Kopiert",
    save: "Speichern",
    cancel: "Abbrechen",
  },
  composer: {
    placeholder: "Frag mich was…",
    placeholderRunning: "Orbit arbeitet…",
    stop: "Stoppen",
    send: "Senden (Enter)",
  },
  emptyState: {
    title: "Wie kann ich helfen?",
    toolsReady_one: "{{count}} Tool bereit",
    toolsReady_other: "{{count}} Tools bereit",
    hotkeyHint: "blendet mich ein und aus",
    suggestions: {
      spotify: "Spiele Another Love von Tom Odell auf Spotify",
      calculator: "Öffne den Rechner",
      weather: "Wie ist das Wetter in Berlin?",
      memory: "Wie viel Arbeitsspeicher ist frei?",
    },
  },
  chat: {
    cancelled: "Abgebrochen",
    thinking: "Orbit denkt nach",
  },
  toolCard: {
    status: {
      running: "Läuft",
      awaiting: "Wartet auf Freigabe",
      ok: "Erledigt",
      error: "Fehlgeschlagen",
    },
    fastPath: "Schnellweg: direkt ausgeführt, ohne KI",
    confirmPrompt: "Orbit möchte Folgendes ausführen:",
    deny: "Ablehnen",
    run: "Ausführen",
    enterKeys: "Schlüssel eintragen",
    details: {
      tool: "Tool",
      arguments: "Argumente",
      result: "Ergebnis",
    },
    durationMs: "{{value}} ms",
    durationS: "{{value}} s",
  },
  webResults: {
    more_one: "{{count}} weiteres Ergebnis",
    more_other: "{{count}} weitere Ergebnisse",
    less: "Weniger anzeigen",
    open: "{{url}} öffnen",
  },
  header: {
    menu: "Menü",
    pinned: "Angepinnt: bleibt offen",
    collapse: "Verkleinern (Strg+E)",
  },
  menu: {
    newChat: "Neuer Chat",
    newChatShortcut: "Strg+N",
    pin: "Anpinnen",
    keys: "Schlüssel & API-Keys",
    configFolder: "Konfigurationsordner",
    reloadConfig: "Konfiguration neu laden",
  },
  profile: {
    label: "Modell",
    local: "Lokal",
    cloud: "Cloud",
    keyMissing: "Schlüssel fehlt",
  },
  compact: {
    expand: "Erweitern (Strg+E)",
    problems_one: "{{count}} Konfigurationsproblem – Details im Panel",
    problems_other: "{{count}} Konfigurationsprobleme – Details im Panel",
  },
  diagnostics: {
    count_one: "{{count}} Konfigurationsproblem",
    count_other: "{{count}} Konfigurationsprobleme",
    openFolder: "Konfigurationsordner öffnen",
  },
  keys: {
    back: "Chat",
    backTitle: "Zurück zum Chat (Esc)",
    title: "Schlüssel",
    storageNote: "Sicher in der Windows-Anmeldeinformationsverwaltung, wird nie wieder angezeigt.",
    empty: "Gerade braucht nichts einen Schlüssel.",
    sections: {
      ai: "KI-Anbieter",
      integration: "Integrationen",
      custom: "Eigene Tools",
    },
    status: {
      active: "Aktiv",
      notSet: "Nicht eingerichtet",
      partial_one: "{{count}} fehlt",
      partial_other: "{{count}} fehlen",
    },
    group: {
      testConnection: "Verbindung testen",
      getKey: "Key erstellen",
    },
    field: {
      change: "Ändern",
      remove: "Entfernen",
      confirmRemove: "Wirklich entfernen?",
    },
  },
  keysGuide: {
    spotify: {
      title: "So bekommst du die Schlüssel (2 Minuten, Premium nötig)",
      step1: "Im <b>Spotify Developer Dashboard</b> (Button unten) mit deinem Premium-Konto anmelden.",
      step2: "<b>Create app</b>: Name „Orbit“, Redirect URI <uri/>, bei APIs <b>Web API</b> anhaken, speichern.",
      step3: "In der App unter <b>Settings</b> Client ID und Client Secret kopieren und hier einfügen.",
    },
  },
};
