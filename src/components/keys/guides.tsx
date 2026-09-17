import { Check, Copy } from "lucide-react";
import { useState, type ComponentType, type ReactNode } from "react";
import { Trans, useTranslation } from "react-i18next";

/** Step-by-step help per service, keyed by group id (`secrets/catalog.rs`). */
export const guides: Record<string, ComponentType> = {
  spotify: SpotifyGuide,
};

const SPOTIFY_REDIRECT_URI = "http://127.0.0.1:43821/callback";

function SpotifyGuide() {
  const { t } = useTranslation();
  const components = { b: <b />, uri: <CopyValue value={SPOTIFY_REDIRECT_URI} /> };
  return (
    <Guide title={t("keysGuide.spotify.title")}>
      <li>
        <Trans i18nKey="keysGuide.spotify.step1" components={components} />
      </li>
      <li>
        <Trans i18nKey="keysGuide.spotify.step2" components={components} />
      </li>
      <li>
        <Trans i18nKey="keysGuide.spotify.step3" components={components} />
      </li>
    </Guide>
  );
}

function Guide({ title, children }: { title: string; children: ReactNode }) {
  return (
    <details className="group text-[12px] leading-relaxed">
      <summary className="cursor-pointer text-muted select-none marker:text-subtle hover:text-fg">{title}</summary>
      <ol className="mt-1.5 list-decimal space-y-1 pl-4 text-subtle [&_b]:font-medium [&_b]:text-muted">
        {children}
      </ol>
    </details>
  );
}

/** Inline code with a copy button. */
function CopyValue({ value }: { value: string }) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  return (
    <span className="inline-flex items-center gap-1 rounded-md border border-line bg-raised py-px pr-0.5 pl-1.5 align-middle">
      <code className="font-mono text-[11.5px] text-fg">{value}</code>
      <button
        type="button"
        title={t("common.copy")}
        onClick={async () => {
          await navigator.clipboard.writeText(value);
          setCopied(true);
          setTimeout(() => setCopied(false), 1500);
        }}
        className="grid size-5 place-items-center rounded text-subtle hover:bg-sunken hover:text-fg"
      >
        {copied ? <Check size={12} className="text-success" /> : <Copy size={12} />}
      </button>
    </span>
  );
}
