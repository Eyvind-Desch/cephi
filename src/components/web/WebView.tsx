import clsx from "clsx";
import { ArrowLeft, ArrowRight, Expand, ExternalLink, Globe, Lock, RotateCw, Shrink, Sparkles, X } from "lucide-react";
import { useEffect, useRef, useState, type RefObject } from "react";
import { useTranslation } from "react-i18next";
import { api, onWindowShown } from "../../lib/ipc";
import { useUi } from "../../store/ui";
import { useWeb } from "../../store/web";
import { IconButton } from "../ui/IconButton";

/**
 * The web tab. The page itself is a native child webview owned by the backend; this component
 * draws the toolbar and reserves the page area, whose position it keeps in sync.
 */
export function WebView() {
  const pageRef = useRef<HTMLDivElement>(null);
  const hasPage = useWeb((s) => s.url != null);
  const covered = useUi((s) => s.overlays > 0);

  usePagePlacement(pageRef, !covered);

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <Toolbar />
      <div ref={pageRef} data-web-page={hasPage || undefined} className="relative min-h-0 flex-1">
        {!hasPage && <EmptyPage />}
      </div>
    </div>
  );
}

function Toolbar() {
  const { t } = useTranslation();
  const { url, loading } = useWeb();
  const large = useUi((s) => s.webLarge);
  const toggleLarge = useUi((s) => s.toggleWebLarge);
  const hasPage = url != null;

  return (
    <div className="flex items-center gap-0.5 border-b border-line px-2 py-1.5">
      <IconButton icon={ArrowLeft} label={t("web.back")} disabled={!hasPage} onClick={() => void api.webCommand("back")} />
      <IconButton
        icon={ArrowRight}
        label={t("web.forward")}
        disabled={!hasPage}
        onClick={() => void api.webCommand("forward")}
      />
      <IconButton
        icon={loading ? X : RotateCw}
        label={loading ? t("web.stop") : t("web.reload")}
        disabled={!hasPage}
        onClick={() => void api.webCommand(loading ? "stop" : "reload")}
      />
      <AddressBar />
      <IconButton
        icon={Sparkles}
        label={t("web.askAi")}
        disabled={!hasPage}
        onClick={() => {
          const { url: pageUrl, title } = useWeb.getState();
          if (!pageUrl) return;
          useUi.getState().setPageContext({ url: pageUrl, title });
          useUi.getState().setView("chat");
        }}
      />
      <IconButton
        icon={ExternalLink}
        label={t("web.openInBrowser")}
        disabled={!hasPage}
        onClick={() => void api.webCommand("openExternal")}
      />
      <IconButton
        icon={large ? Shrink : Expand}
        label={large ? t("web.shrink") : t("web.enlarge")}
        onClick={toggleLarge}
      />
    </div>
  );
}

function AddressBar() {
  const { t } = useTranslation();
  const { url, title, loading } = useWeb();
  const [draft, setDraft] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const editing = draft != null;

  // Ready for input when the tab opens without a page and whenever Cephi is summoned.
  useEffect(() => {
    if (!useWeb.getState().url) inputRef.current?.focus();
    const unlisten = onWindowShown(() => requestAnimationFrame(() => inputRef.current?.focus()));
    return () => void unlisten.then((fn) => fn());
  }, []);

  // A page loaded (from the address bar, a tool or a link): show its address.
  useEffect(() => {
    if (!url) return;
    setDraft(null);
    inputRef.current?.blur();
  }, [url]);

  const submit = () => {
    const input = (draft ?? "").trim();
    setDraft(null);
    inputRef.current?.blur();
    if (input) void api.webNavigate(input);
  };

  const Icon = url?.startsWith("https://") ? Lock : Globe;

  return (
    <label
      className={clsx(
        "relative mx-1 flex h-8 min-w-0 flex-1 items-center gap-2 overflow-hidden rounded-lg bg-sunken px-2.5 transition-colors",
        editing && "ring-1 ring-accent/60",
      )}
      title={url ? `${title}\n${url}` : undefined}
    >
      <Icon size={13} className="shrink-0 text-subtle" />
      <input
        ref={inputRef}
        value={editing ? draft : (url ?? "")}
        placeholder={t("web.placeholder")}
        spellCheck={false}
        onFocus={(e) => {
          setDraft(url ?? "");
          // Select after React re-rendered the input with the draft.
          const input = e.currentTarget;
          requestAnimationFrame(() => input.select());
        }}
        onBlur={() => setDraft(null)}
        onChange={(e) => setDraft(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") submit();
          if (e.key === "Escape" && editing && url) {
            // With a page: leave editing first; a second Escape goes back to the chat.
            e.stopPropagation();
            e.preventDefault();
            setDraft(null);
            inputRef.current?.blur();
          }
        }}
        className="min-w-0 flex-1 bg-transparent text-[12.5px] text-fg outline-none placeholder:text-subtle"
      />
      {loading && (
        <span className="absolute inset-x-0 bottom-0 h-0.5 overflow-hidden">
          <span className="block h-full w-1/3 animate-[cephi-progress_1.1s_ease-in-out_infinite] rounded-full bg-accent" />
        </span>
      )}
    </label>
  );
}

function EmptyPage() {
  const { t } = useTranslation();
  return (
    <div className="grid h-full place-items-center p-6 text-center select-none">
      <div className="space-y-2">
        <Globe size={28} className="mx-auto text-subtle" />
        <p className="text-[14px] text-fg">{t("web.emptyTitle")}</p>
        <p className="text-[12.5px] text-subtle">{t("web.emptyHint")}</p>
      </div>
    </div>
  );
}

/** Increases across remounts and page reloads, so the backend can drop stale layout reports. */
let layoutSeq = Date.now() * 1000;

/**
 * Keeps the native page on top of `ref`'s area while mounted and `visible`, hides it otherwise.
 * Every report carries position and visibility together, so reports cannot contradict each other.
 */
function usePagePlacement(ref: RefObject<HTMLDivElement | null>, visible: boolean) {
  const visibleRef = useRef(visible);
  const reportRef = useRef<() => void>(() => {});

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const report = () => {
      const r = el.getBoundingClientRect();
      const bounds = { x: r.left, y: r.top, width: r.width, height: r.height };
      void api.webSetLayout(bounds, visibleRef.current, ++layoutSeq);
    };
    reportRef.current = report;

    report();
    // The panel animates in with a slight scale; measure again once it settled.
    const settle = window.setTimeout(report, 220);
    const observer = new ResizeObserver(report);
    observer.observe(el);
    window.addEventListener("resize", report);
    return () => {
      window.clearTimeout(settle);
      observer.disconnect();
      window.removeEventListener("resize", report);
      void api.webSetLayout({ x: 0, y: 0, width: 0, height: 0 }, false, ++layoutSeq);
    };
  }, [ref]);

  useEffect(() => {
    visibleRef.current = visible;
    reportRef.current();
  }, [visible]);
}
