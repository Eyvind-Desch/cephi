/**
 * Syntax highlighting for code blocks in answers. Only the languages a Windows assistant typically
 * shows are loaded (the full highlight.js bundle is ~10× larger).
 */
import hljs from "highlight.js/lib/core";
import bash from "highlight.js/lib/languages/bash";
import cpp from "highlight.js/lib/languages/cpp";
import csharp from "highlight.js/lib/languages/csharp";
import css from "highlight.js/lib/languages/css";
import diff from "highlight.js/lib/languages/diff";
import dos from "highlight.js/lib/languages/dos";
import go from "highlight.js/lib/languages/go";
import ini from "highlight.js/lib/languages/ini";
import java from "highlight.js/lib/languages/java";
import javascript from "highlight.js/lib/languages/javascript";
import json from "highlight.js/lib/languages/json";
import markdown from "highlight.js/lib/languages/markdown";
import powershell from "highlight.js/lib/languages/powershell";
import python from "highlight.js/lib/languages/python";
import rust from "highlight.js/lib/languages/rust";
import sql from "highlight.js/lib/languages/sql";
import typescript from "highlight.js/lib/languages/typescript";
import xml from "highlight.js/lib/languages/xml";
import yaml from "highlight.js/lib/languages/yaml";

const LANGUAGES = {
  bash,
  cpp,
  csharp,
  css,
  diff,
  dos,
  go,
  ini,
  java,
  javascript,
  json,
  markdown,
  powershell,
  python,
  rust,
  sql,
  typescript,
  xml,
  yaml,
};
for (const [name, language] of Object.entries(LANGUAGES)) hljs.registerLanguage(name, language);
// Names models like to write after the fences.
hljs.registerAliases(["ps", "ps1", "pwsh"], { languageName: "powershell" });
hljs.registerAliases(["cmd", "bat", "batch"], { languageName: "dos" });
hljs.registerAliases(["sh", "shell", "zsh", "console"], { languageName: "bash" });
hljs.registerAliases(["toml"], { languageName: "ini" });

/** Longer blocks are shown plain: guessing and colouring them costs more than it helps. */
const MAX_CHARS = 20_000;

export interface Highlighted {
  /** Escaped HTML with `hljs-*` spans. */
  html: string;
  /** Display name of the language used (named after the fences or detected), e.g. "PowerShell". */
  language: string;
}

/**
 * `language` comes from the fences (```ts). Without one, the language is only guessed for
 * blocks that look like code with reasonable confidence; prose stays plain.
 */
export function highlight(code: string, language: string | null): Highlighted | null {
  if (code.length > MAX_CHARS) return null;
  if (language && hljs.getLanguage(language)) {
    const result = hljs.highlight(code, { language, ignoreIllegals: true });
    return { html: result.value, language: hljs.getLanguage(language)?.name ?? language };
  }
  if (language) return null;
  const guess = hljs.highlightAuto(code);
  return guess.language && guess.relevance >= 5
    ? { html: guess.value, language: hljs.getLanguage(guess.language)?.name ?? guess.language }
    : null;
}
