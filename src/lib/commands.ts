/**
 * Slash commands typed into the composer, like in Claude Code: `/clear`, `/web cats`, `/model smart`, …
 * They run in the UI and never reach the model.
 */
import { useChat } from "../store/chat";
import { emptyNote, useNotes } from "../store/notes";
import { useTranslate } from "../store/translate";
import { useUi } from "../store/ui";
import { api } from "./ipc";

export interface SlashCommand {
  name: string;
  aliases?: string[];
  /** What the argument means (shown as a hint). `profile` offers the model profiles to pick from. */
  arg?: "query" | "text" | "profile";
  description:
    | "commands.clear"
    | "commands.history"
    | "commands.screenshot"
    | "commands.web"
    | "commands.translate"
    | "commands.notes"
    | "commands.model"
    | "commands.tools"
    | "commands.settings"
    | "commands.shortcuts"
    | "commands.keys"
    | "commands.mcp"
    | "commands.pin"
    | "commands.config"
    | "commands.reload";
  run: (arg: string) => void;
}

export const COMMANDS: SlashCommand[] = [
  {
    name: "clear",
    aliases: ["new"],
    description: "commands.clear",
    run: () => {
      useUi.getState().setView("chat");
      void useChat.getState().reset();
    },
  },
  { name: "history", aliases: ["chats"], description: "commands.history", run: () => useUi.getState().setView("history") },
  {
    name: "web",
    aliases: ["g"],
    arg: "query",
    description: "commands.web",
    run: (query) => {
      useUi.getState().setView("web");
      if (query) void api.webNavigate(query);
    },
  },
  {
    name: "notes",
    aliases: ["note"],
    arg: "text",
    description: "commands.notes",
    run: (text) => {
      useUi.getState().setView("notes");
      if (text.trim()) void useNotes.getState().save({ ...emptyNote(), text: text.trim() }).catch(() => undefined);
    },
  },
  {
    name: "translate",
    aliases: ["tr"],
    arg: "text",
    description: "commands.translate",
    run: (text) => {
      if (text) useTranslate.setState({ text });
      useUi.getState().setView("translate");
    },
  },
  {
    name: "model",
    arg: "profile",
    description: "commands.model",
    run: (name) => {
      const ui = useUi.getState();
      const wanted = name.toLowerCase();
      const profile = ui.config?.profiles.find((p) => p.id === wanted || p.label.toLowerCase() === wanted);
      if (!profile) return;
      if (profile.available) ui.setProfile(profile.id);
      else ui.openSettings("keys");
    },
  },
  {
    name: "screenshot",
    aliases: ["shot"],
    description: "commands.screenshot",
    run: () => void useUi.getState().captureScreenshot(),
  },
  { name: "tools", description: "commands.tools", run: () => useUi.getState().setView("tools") },
  { name: "settings", description: "commands.settings", run: () => useUi.getState().openSettings("general") },
  {
    name: "shortcuts",
    description: "commands.shortcuts",
    run: () => useUi.getState().openSettings("shortcuts"),
  },
  { name: "keys", description: "commands.keys", run: () => useUi.getState().openSettings("keys") },
  { name: "mcp", description: "commands.mcp", run: () => useUi.getState().openSettings("mcp") },
  { name: "pin", description: "commands.pin", run: () => useUi.getState().togglePinned() },
  { name: "config", description: "commands.config", run: () => void api.configOpenFolder() },
  {
    name: "reload",
    description: "commands.reload",
    run: () => void api.configReload().then(useUi.getState().applyConfig),
  },
];

export interface ParsedCommand {
  /** What was typed after the slash, up to the first space. */
  name: string;
  arg: string;
  /** Whether a space follows the name, i.e. the argument is being typed. */
  hasArg: boolean;
}

/** `/web cats` → `{ name: "web", arg: "cats" }`. `null` if the input is not a command. */
export function parseCommand(input: string): ParsedCommand | null {
  if (!input.startsWith("/")) return null;
  const body = input.slice(1);
  const space = body.search(/\s/);
  if (space < 0) return { name: body.toLowerCase(), arg: "", hasArg: false };
  return { name: body.slice(0, space).toLowerCase(), arg: body.slice(space + 1).trim(), hasArg: true };
}

/** Commands whose name or alias starts with `prefix`; an exact match comes first. */
export function matchCommands(prefix: string): SlashCommand[] {
  const names = (command: SlashCommand) => [command.name, ...(command.aliases ?? [])];
  const exact = COMMANDS.filter((c) => names(c).includes(prefix));
  const rest = COMMANDS.filter((c) => !exact.includes(c) && names(c).some((n) => n.startsWith(prefix)));
  return [...exact, ...rest];
}
