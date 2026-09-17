import { api } from "./ipc";

/** Opens a link from the chat: web pages in the web tab, everything else (e.g. mailto:) externally. */
export function openLink(url: string) {
  return /^https?:\/\//i.test(url) ? api.webOpen(url) : api.openExternal(url);
}
