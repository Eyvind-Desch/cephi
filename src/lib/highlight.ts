/**
 * Syntax highlighting for code blocks in answers.
 *
 * highlight.js and its languages are a good 100 KB — loaded only when an answer actually contains a
 * code block, so the window opens without them. Only the languages a Windows assistant typically
 * shows are registered (the full bundle is ~10× larger).
 */
import type { HLJSApi } from "highlight.js";

/** Longer blocks are shown plain: guessing and colouring them costs more than it helps. */
const MAX_CHARS = 20_000;

export interface Highlighted {
  /** Escaped HTML with `hljs-*` spans. */
  html: string;
  /** Display name of the language used (named after the fences or detected), e.g. "PowerShell". */
  language: string;
}

let loading: Promise<HLJSApi> | null = null;

/** Loads highlight.js once; every later call gets the same instance. */
function highlighter(): Promise<HLJSApi> {
  loading ??= (async () => {
    const [{ default: hljs }, ...languages] = await Promise.all([
      import("highlight.js/lib/core"),
      import("highlight.js/lib/languages/bash"),
      import("highlight.js/lib/languages/cpp"),
      import("highlight.js/lib/languages/csharp"),
      import("highlight.js/lib/languages/css"),
      import("highlight.js/lib/languages/diff"),
      import("highlight.js/lib/languages/dos"),
      import("highlight.js/lib/languages/go"),
      import("highlight.js/lib/languages/ini"),
      import("highlight.js/lib/languages/java"),
      import("highlight.js/lib/languages/javascript"),
      import("highlight.js/lib/languages/json"),
      import("highlight.js/lib/languages/markdown"),
      import("highlight.js/lib/languages/powershell"),
      import("highlight.js/lib/languages/python"),
      import("highlight.js/lib/languages/rust"),
      import("highlight.js/lib/languages/sql"),
      import("highlight.js/lib/languages/typescript"),
      import("highlight.js/lib/languages/xml"),
      import("highlight.js/lib/languages/yaml"),
    ]);
    const names = [
      "bash",
      "cpp",
      "csharp",
      "css",
      "diff",
      "dos",
      "go",
      "ini",
      "java",
      "javascript",
      "json",
      "markdown",
      "powershell",
      "python",
      "rust",
      "sql",
      "typescript",
      "xml",
      "yaml",
    ];
    names.forEach((name, index) => hljs.registerLanguage(name, languages[index].default));
    // Names models like to write after the fences.
    hljs.registerAliases(["ps", "ps1", "pwsh"], { languageName: "powershell" });
    hljs.registerAliases(["cmd", "bat", "batch"], { languageName: "dos" });
    hljs.registerAliases(["sh", "shell", "zsh", "console"], { languageName: "bash" });
    hljs.registerAliases(["toml"], { languageName: "ini" });
    return hljs;
  })();
  return loading;
}

/**
 * `language` comes from the fences (```ts). Without one, the language is only guessed for
 * blocks that look like code with reasonable confidence; prose stays plain.
 */
export async function highlight(code: string, language: string | null): Promise<Highlighted | null> {
  if (code.length > MAX_CHARS) return null;
  const hljs = await highlighter();
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
