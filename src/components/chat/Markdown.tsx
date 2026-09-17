import { Check, Copy } from "lucide-react";
import { memo, useRef, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import ReactMarkdown, { type Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import { api } from "../../lib/ipc";

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
  // Links must never navigate the app's webview; open them in the default browser.
  a: ({ href, children }) => (
    <a
      href={href}
      onClick={(e) => {
        e.preventDefault();
        if (href) void api.openExternal(href);
      }}
    >
      {children}
    </a>
  ),
};

export const Markdown = memo(function Markdown({ text }: { text: string }) {
  return (
    <div className="prose-orbit">
      <ReactMarkdown remarkPlugins={[remarkGfm]} components={components}>
        {text}
      </ReactMarkdown>
    </div>
  );
});
