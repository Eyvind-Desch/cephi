import clsx from "clsx";
import { ArrowRight, Check, Copy, Loader2, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { api, onWindowShown, type Translation } from "../../lib/ipc";
import { languageName } from "./languages";

/** Pause after typing before translating. LibreTranslate answers in well under a second. */
const DEBOUNCE_MS = 300;

type Direction = "auto" | "en" | "de";

/** Translator tab: type or paste, the translation follows while typing. */
export function TranslateView() {
  const { t } = useTranslation();
  const [text, setText] = useState("");
  const [direction, setDirection] = useState<Direction>("auto");
  const [result, setResult] = useState<Translation | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const request = useRef(0);

  useEffect(() => {
    inputRef.current?.focus();
    const unlisten = onWindowShown(() =>
      requestAnimationFrame(() => {
        inputRef.current?.focus();
        inputRef.current?.select();
      }),
    );
    return () => void unlisten.then((fn) => fn());
  }, []);

  useEffect(() => {
    const input = text.trim();
    const id = ++request.current;
    if (!input) {
      setResult(null);
      setError(null);
      setLoading(false);
      return;
    }
    const timer = window.setTimeout(async () => {
      setLoading(true);
      try {
        const translation = await api.translate(input, direction === "auto" ? null : direction);
        if (id !== request.current) return;
        setResult(translation);
        setError(null);
      } catch (e) {
        if (id !== request.current) return;
        setError(String(e));
      } finally {
        if (id === request.current) setLoading(false);
      }
    }, DEBOUNCE_MS);
    return () => window.clearTimeout(timer);
  }, [text, direction]);

  const directions: { id: Direction; label: string }[] = [
    { id: "auto", label: t("translate.auto") },
    { id: "en", label: `→ ${languageName("en")}` },
    { id: "de", label: `→ ${languageName("de")}` },
  ];

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-3">
      <div role="radiogroup" aria-label={t("translate.direction")} className="flex items-center gap-1 self-start rounded-lg bg-sunken p-0.5">
        {directions.map((d) => (
          <button
            key={d.id}
            type="button"
            role="radio"
            aria-checked={direction === d.id}
            onClick={() => setDirection(d.id)}
            className={clsx(
              "h-7 rounded-md px-2.5 text-[12.5px] transition-colors",
              direction === d.id ? "bg-raised text-fg shadow-sm shadow-black/10" : "text-muted hover:text-fg",
            )}
          >
            {d.label}
          </button>
        ))}
      </div>

      <div className="relative rounded-2xl border border-line bg-raised transition-colors focus-within:border-accent/50">
        <textarea
          ref={inputRef}
          value={text}
          onChange={(e) => setText(e.target.value)}
          placeholder={t("translate.placeholder")}
          spellCheck={false}
          rows={4}
          className="block max-h-56 min-h-24 w-full resize-none bg-transparent px-3.5 py-3 pr-10 text-[15px] leading-relaxed text-fg outline-none placeholder:text-subtle"
        />
        {text && (
          <button
            type="button"
            onClick={() => {
              setText("");
              inputRef.current?.focus();
            }}
            title={t("translate.clear")}
            className="absolute top-2.5 right-2.5 grid size-6 place-items-center rounded-md text-subtle hover:bg-sunken hover:text-fg"
          >
            <X size={14} />
          </button>
        )}
      </div>

      {(result || loading || error) && (
        <div className="selectable rounded-2xl border border-line bg-sunken px-3.5 py-3">
          <div className="mb-1.5 flex h-5 items-center gap-1.5 text-[11.5px] text-subtle select-none">
            {result && (
              <>
                {languageName(result.from)} <ArrowRight size={11} /> {languageName(result.to)}
              </>
            )}
            {loading && <Loader2 size={12} className="ml-auto animate-spin" />}
          </div>
          {error ? (
            <p className="text-[13px] leading-relaxed text-danger">{error}</p>
          ) : (
            result && (
              <div className={clsx("transition-opacity", loading && "opacity-60")}>
                <div className="flex items-start gap-2">
                  <p className="min-w-0 flex-1 text-[15px] leading-relaxed whitespace-pre-wrap text-fg">{result.text}</p>
                  <CopyButton text={result.text} />
                </div>
                {result.alternatives.length > 0 && <Alternatives items={result.alternatives} />}
              </div>
            )
          )}
        </div>
      )}
    </div>
  );
}

export function Alternatives({ items }: { items: string[] }) {
  const { t } = useTranslation();
  return (
    <div className="mt-2.5 flex flex-wrap items-center gap-1.5">
      <span className="text-[11.5px] text-subtle select-none">{t("translate.alternatives")}</span>
      {items.map((item) => (
        <CopyChip key={item} text={item} />
      ))}
    </div>
  );
}

function CopyChip({ text }: { text: string }) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  return (
    <button
      type="button"
      title={t("common.copy")}
      onClick={async () => {
        await navigator.clipboard.writeText(text);
        setCopied(true);
        setTimeout(() => setCopied(false), 1200);
      }}
      className="inline-flex items-center gap-1 rounded-md border border-line bg-raised px-2 py-0.5 text-[12.5px] text-muted hover:text-fg"
    >
      {copied && <Check size={12} className="text-success" />}
      {text}
    </button>
  );
}

export function CopyButton({ text }: { text: string }) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  return (
    <button
      type="button"
      title={copied ? t("common.copied") : t("common.copy")}
      onClick={async () => {
        await navigator.clipboard.writeText(text);
        setCopied(true);
        setTimeout(() => setCopied(false), 1500);
      }}
      className="grid size-7 shrink-0 place-items-center rounded-md text-subtle hover:bg-raised hover:text-fg"
    >
      {copied ? <Check size={14} className="text-success" /> : <Copy size={14} />}
    </button>
  );
}
