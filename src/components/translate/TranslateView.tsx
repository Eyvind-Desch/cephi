import clsx from "clsx";
import { Loader2, X } from "lucide-react";
import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { api, onWindowShown } from "../../lib/ipc";
import { useTranslate, type Direction } from "../../store/translate";
import { languageName } from "./languages";
import { TranslationView } from "./TranslationView";

/** Pause after typing before translating. LibreTranslate answers in well under a second. */
const DEBOUNCE_MS = 300;

/** Translator tab: type or paste, the translation follows while typing. */
export function TranslateView() {
  const { t } = useTranslation();
  const { text, direction, result, error, loading } = useTranslate();
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
    const { result: current } = useTranslate.getState();
    if (!input) {
      useTranslate.setState({ result: null, error: null, loading: false });
      return;
    }
    // Already translated, e.g. opened from a chat card.
    if (current?.source === input && (direction === "auto" || current.to === direction)) return;

    const timer = window.setTimeout(async () => {
      useTranslate.setState({ loading: true });
      try {
        const translation = await api.translate(input, direction === "auto" ? null : direction);
        if (id === request.current) useTranslate.setState({ result: translation, error: null });
      } catch (e) {
        if (id === request.current) useTranslate.setState({ error: String(e) });
      } finally {
        if (id === request.current) useTranslate.setState({ loading: false });
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
      <div
        role="radiogroup"
        aria-label={t("translate.direction")}
        className="flex items-center gap-1 self-start rounded-lg bg-sunken p-0.5"
      >
        {directions.map((d) => (
          <button
            key={d.id}
            type="button"
            role="radio"
            aria-checked={direction === d.id}
            onClick={() => useTranslate.setState({ direction: d.id })}
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
          onChange={(e) => useTranslate.setState({ text: e.target.value })}
          placeholder={t("translate.placeholder")}
          spellCheck={false}
          rows={4}
          className="block max-h-56 min-h-24 w-full resize-none bg-transparent px-3.5 py-3 pr-10 text-[15px] leading-relaxed text-fg outline-none placeholder:text-subtle"
        />
        {text && (
          <button
            type="button"
            onClick={() => {
              useTranslate.setState({ text: "" });
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
        <div className="selectable relative rounded-2xl border border-line bg-sunken px-3.5 py-3">
          {loading && <Loader2 size={12} className="absolute top-3.5 right-3.5 animate-spin text-subtle" />}
          {error ? (
            <p className="text-[13px] leading-relaxed text-danger">{error}</p>
          ) : (
            result && <TranslationView translation={result} dimmed={loading} />
          )}
        </div>
      )}
    </div>
  );
}
