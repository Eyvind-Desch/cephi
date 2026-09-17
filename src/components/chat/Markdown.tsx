import { Check, Copy } from "lucide-react";
import { memo, useRef, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import ReactMarkdown, { type Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import type { WebResult } from "../../lib/ipc";
import { openLink } from "../../lib/links";

function CodeBlock({ children }: { children?: ReactNode }) {
  const { t } = useTranslation();
  const ref = useRef<HTMLPreElement>(null);
  const [copied, setCopied] = useState(false);

  const copy = async () => {
    await navigator.clipboard.writeText(ref.current?.textContent ?? "");
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div className="group relative">
      <pre
        ref={ref}
        className="overflow-x-auto rounded-xl border border-line bg-sunken px-3.5 py-3 font-mono text-[12.5px] leading-relaxed"
      >
        {children}
      </pre>
      <button
        type="button"
        onClick={copy}
        title={t("common.copy")}
        className="absolute top-2 right-2 grid size-7 place-items-center rounded-md bg-raised text-muted opacity-0 transition-opacity group-hover:opacity-100 hover:text-fg"
      >
        {copied ? <Check size={14} /> : <Copy size={14} />}
      </button>
    </div>
  );
}

const components: Components = {
  pre: ({ children }) => <CodeBlock>{children}</CodeBlock>,
  // Links must never navigate the app's webview; they open in the web tab.
  a: ({ href, children }) => {
    const citation = typeof children === "string" && /^\d+$/.test(children);
    return (
      <a
        href={href}
        title={citation ? href : undefined}
        className={citation ? "citation" : undefined}
        onClick={(e) => {
          e.preventDefault();
          if (href) void openLink(href);
        }}
      >
        {children}
      </a>
    );
  },
};

interface MarkdownProps {
  text: string;
  /** Web results that `[n]` citations in the text refer to. */
  sources?: WebResult[];
}

export const Markdown = memo(function Markdown({ text, sources }: MarkdownProps) {
  return (
    <div className="prose-orbit">
      <ReactMarkdown remarkPlugins={[remarkGfm]} components={components}>
        {sources ? linkCitations(text, sources) : text}
      </ReactMarkdown>
    </div>
  );
});

/** Turns `[1]`, `[1, 3]` and `[1][2]` into links to the matching sources. Unknown numbers stay text. */
export function linkCitations(text: string, sources: WebResult[]) {
  return text.replace(/\[(\d+(?:\s*,\s*\d+)*)\](?!\()/g, (match, numbers: string) => {
    const links = numbers.split(",").map((n) => {
      const source = sources[Number(n.trim()) - 1];
      return source ? `[${n.trim()}](<${source.url}>)` : null;
    });
    return links.every(Boolean) ? links.join("") : match;
  });
}
