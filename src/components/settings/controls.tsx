import clsx from "clsx";
import type { ReactNode } from "react";

/** Shared building blocks of the settings page. */

export const inputClass =
  "h-8 w-full rounded-lg border border-line bg-sunken px-2.5 text-[12.5px] text-fg outline-none transition-colors placeholder:text-subtle focus:border-accent/40";

export const textButtonClass =
  "rounded-md px-2 py-1 text-[12px] text-muted transition-colors hover:bg-sunken hover:text-fg disabled:opacity-40";

export const primaryButtonClass =
  "rounded-lg bg-accent px-3 py-1.5 text-[12.5px] font-medium text-accent-fg transition hover:brightness-110 disabled:opacity-50";

export function Card({ title, children }: { title?: string; children: ReactNode }) {
  return (
    <section className="space-y-1.5">
      {title && (
        <h3 className="px-1 text-[10.5px] font-medium tracking-wider text-subtle uppercase">{title}</h3>
      )}
      <div className="divide-y divide-line overflow-hidden rounded-xl border border-line bg-raised">{children}</div>
    </section>
  );
}

/** One setting: label and hint on the left, the control on the right. */
export function Row({
  label,
  hint,
  error,
  children,
}: {
  label: string;
  hint?: string;
  error?: string | null;
  children: ReactNode;
}) {
  return (
    <div className="px-3 py-2.5">
      <div className="flex items-center gap-3">
        <div className="min-w-0 flex-1">
          <div className="text-[13px] text-fg">{label}</div>
          {hint && <div className="text-[11.5px] leading-snug text-subtle">{hint}</div>}
        </div>
        <div className="shrink-0">{children}</div>
      </div>
      {error && <p className="mt-1 text-[12px] leading-snug text-danger select-text">{error}</p>}
    </div>
  );
}

export function Toggle({
  checked,
  onChange,
  disabled,
  label,
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  label: string;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      title={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={clsx(
        "relative inline-flex h-5 w-9 shrink-0 items-center rounded-full transition-colors disabled:opacity-40",
        checked ? "bg-accent" : "bg-sunken ring-1 ring-line ring-inset",
      )}
    >
      <span
        className={clsx(
          "absolute size-3.5 rounded-full shadow-sm transition-all",
          checked ? "left-[18px] bg-white" : "left-[3px] bg-subtle",
        )}
      />
    </button>
  );
}

export function Select<T extends string>({
  value,
  options,
  onChange,
  label,
}: {
  value: T;
  options: { value: T; label: string }[];
  onChange: (value: T) => void;
  label: string;
}) {
  return (
    <select
      value={value}
      aria-label={label}
      onChange={(e) => onChange(e.target.value as T)}
      className="h-8 max-w-44 rounded-lg border border-line bg-sunken px-2 text-[12.5px] text-fg outline-none focus:border-accent/40"
    >
      {options.map((option) => (
        <option key={option.value} value={option.value}>
          {option.label}
        </option>
      ))}
    </select>
  );
}
