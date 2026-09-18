import { Check, Copy } from "lucide-react";
import { isValidElement, memo, useMemo, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import ReactMarkdown, { type Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import { highlight } from "../../lib/highlight";
import type { WebResult } from "../../lib/ipc";
import { openLink } from "../../lib/links";

/** A fenced code block: highlighted, with the language and a copy button in its header. */
function CodeBlock({ code, fence }: { code: string; fence: string | null }) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  const text = code.replace(/\n$/, "");
  const highlighted = useMemo(() => highlight(text, fence), [text, fence]);
  const label = highlighted?.language ?? fence;

  const copy = async () => {
    await navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div className="overflow-hidden rounded-xl border border-line bg-sunken">
      <div className="flex h-7 items-center justify-between border-b border-line pr-1 pl-3">
        <span className="text-[11px] text-subtle">{label}</span>
        <button
          type="button"
          onClick={copy}
          title={t("common.copy")}
          className="flex items-center gap-1 rounded-md px-1.5 py-0.5 text-[11px] text-subtle transition-colors hover:bg-raised hover:text-fg"
        >
          {copied ? <Check size={12} /> : <Copy size={12} />}
          {copied ? t("common.copied") : t("common.copy")}
        </button>
      </div>
      {highlighted ? (
        // highlight.js escapes the code; the markup only adds its own `hljs-*` spans.
        <pre
          className="hljs overflow-x-auto px-3.5 py-3 font-mono text-[12.5px] leading-relaxed"
          dangerouslySetInnerHTML={{ __html: highlighted.html }}
        />
      ) : (
        <pre className="overflow-x-auto px-3.5 py-3 font-mono text-[12.5px] leading-relaxed">{text}</pre>
      )}
    </div>
  );
}

/** The text and the language after the fences (```ts → "ts") of the `<code>` inside a `<pre>`. */
function codeOf(children: ReactNode): { code: string; fence: string | null } | null {
  if (!isValidElement<{ className?: string; children?: ReactNode }>(children)) return null;
  const { className, children: content } = children.props;
  const code = Array.isArray(content) ? content.join("") : typeof content === "string" ? content : null;
  if (code === null) return null;
  const fence = /language-(\S+)/.exec(className ?? "")?.[1]?.toLowerCase() ?? null;
  return { code, fence };
}

const components: Components = {
  pre: ({ children }) => {
    const block = codeOf(children);
    return block ? (
      <CodeBlock code={block.code} fence={block.fence} />
    ) : (
      <pre className="overflow-x-auto rounded-xl border border-line bg-sunken px-3.5 py-3 font-mono text-[12.5px]">
        {children}
      </pre>
    );
  },
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
    <div className="prose-cephi">
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
