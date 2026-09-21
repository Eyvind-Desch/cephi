import clsx from "clsx";
import { ChevronDown, Globe } from "lucide-react";
import { memo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { WebResult } from "../../lib/ipc";
import { openLink } from "../../lib/links";

/** Results shown before "show more" (two rows of the grid). */
const INITIAL_COUNT = 4;

/**
 * Search results as a compact source grid, so the answer below stays in view.
 * Numbers match the `[n]` citations in the answer; the excerpt is in the tooltip.
 */
export const WebResults = memo(function WebResults({ results }: { results: WebResult[] }) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(false);
  if (results.length === 0) return null;

  const visible = expanded ? results : results.slice(0, INITIAL_COUNT);
  const hidden = results.length - visible.length;

  return (
    <div>
      <ol className="grid grid-cols-2 gap-1.5">
        {visible.map((result, index) => (
          <li key={result.url} className="min-w-0">
            <SourceCard result={result} number={index + 1} />
          </li>
        ))}
      </ol>
      {results.length > INITIAL_COUNT && (
        <button
          type="button"
          onClick={() => setExpanded((v) => !v)}
          className="mt-1 flex items-center gap-1 rounded-md px-1.5 py-1 text-hint text-subtle hover:text-fg"
        >
          <ChevronDown size={13} className={clsx("transition-transform", expanded && "rotate-180")} />
          {expanded ? t("webResults.less") : t("webResults.more", { count: hidden })}
        </button>
      )}
    </div>
  );
});

function SourceCard({ result, number }: { result: WebResult; number: number }) {
  const host = hostOf(result.url);
  const tooltip = [result.title, result.snippet, result.url].filter(Boolean).join("\n\n");

  return (
    <a
      href={result.url}
      title={tooltip}
      onClick={(e) => {
        e.preventDefault();
        void openLink(result.url);
      }}
      className="flex h-full flex-col gap-1 rounded-xl border border-line bg-raised px-2.5 py-2 transition-colors hover:bg-sunken"
    >
      <span className="line-clamp-2 text-body leading-snug text-fg">{result.title}</span>
      <span className="mt-auto flex items-center gap-1.5 text-caption text-subtle">
        <Favicon host={host} />
        <span className="min-w-0 flex-1 truncate">{host}</span>
        <span className="shrink-0 tabular-nums">{number}</span>
      </span>
    </a>
  );
}

/**
 * A site's initial on a colour derived from its name. Drawn here on purpose: loading real favicons
 * from an icon service would tell that service every site your local search turned up.
 */
function Favicon({ host }: { host: string }) {
  if (!host) return <Globe size={13} className="shrink-0" />;
  const name = host.replace(/^www\./, "");
  // Same host, same colour, every time.
  const hue = [...name].reduce((sum, char) => (sum * 31 + char.charCodeAt(0)) % 360, 7);
  return (
    <span
      aria-hidden
      className="grid size-3 shrink-0 place-items-center rounded-[3px] text-[8px] font-semibold text-white"
      style={{ backgroundColor: `oklch(0.62 0.12 ${hue})` }}
    >
      {name.charAt(0).toUpperCase()}
    </span>
  );
}

function hostOf(url: string) {
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return "";
  }
}
