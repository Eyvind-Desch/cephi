import clsx from "clsx";
import { useId } from "react";

interface CephiMarkProps {
  size?: number;
  /** Sways the tentacles while the assistant is working. */
  active?: boolean;
  className?: string;
}

/** Cephi logo: a friendly octopus with a gradient body. Same shape as the app icon. */
export function CephiMark({ size = 28, active = false, className }: CephiMarkProps) {
  // Several marks can be on screen at once; each needs its own gradient id.
  const gradient = `cephi-body-${useId().replace(/:/g, "")}`;

  return (
    <svg width={size} height={size} viewBox="0 0 32 32" className={clsx("shrink-0", className)} aria-hidden>
      <defs>
        <linearGradient id={gradient} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0%" stopColor="var(--glow-a)" />
          <stop offset="100%" stopColor="var(--glow-b)" />
        </linearGradient>
      </defs>
      <g
        fill="none"
        stroke={`url(#${gradient})`}
        strokeWidth="2.7"
        strokeLinecap="round"
        style={{
          transformOrigin: "16px 18px",
          animation: active ? "cephi-sway 0.9s ease-in-out infinite alternate" : undefined,
        }}
      >
        <path d="M10.6 17.4c-1.1 2.8-3.5 3.6-4.4 6.1-.6 1.6.5 2.8 1.8 2" />
        <path d="M14 18.8c-.3 2.6-1.7 3.9-1.5 6.1.1 1.3 1.5 1.6 2 .5" />
        <path d="M18 18.8c.3 2.6 1.7 3.9 1.5 6.1-.1 1.3-1.5 1.6-2 .5" />
        <path d="M21.4 17.4c1.1 2.8 3.5 3.6 4.4 6.1.6 1.6-.5 2.8-1.8 2" />
      </g>
      <path
        d="M16 3.5c-5 0-8.5 3.6-8.5 8.3 0 3.4 1.9 6 4.6 7.1 1.2.5 2.5.7 3.9.7s2.7-.2 3.9-.7c2.7-1.1 4.6-3.7 4.6-7.1 0-4.7-3.5-8.3-8.5-8.3z"
        fill={`url(#${gradient})`}
      />
      <circle cx="13.1" cy="12.6" r="1.35" fill="#15151a" />
      <circle cx="18.9" cy="12.6" r="1.35" fill="#15151a" />
    </svg>
  );
}
