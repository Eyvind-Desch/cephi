import clsx from "clsx";

interface OrbitMarkProps {
  size?: number;
  /** Spins the satellite while the assistant is working. */
  active?: boolean;
  className?: string;
}

/** Orbit logo: a glowing planet with a satellite on its orbit. */
export function OrbitMark({ size = 28, active = false, className }: OrbitMarkProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 32 32" className={clsx("shrink-0", className)} aria-hidden>
      <defs>
        <linearGradient id="orbit-core" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0%" stopColor="var(--glow-a)" />
          <stop offset="100%" stopColor="var(--glow-b)" />
        </linearGradient>
      </defs>
      <circle cx="16" cy="16" r="7" fill="url(#orbit-core)" />
      <circle cx="16" cy="16" r="12.5" fill="none" stroke="currentColor" strokeOpacity="0.18" strokeWidth="1.5" />
      <g
        style={{
          transformOrigin: "16px 16px",
          animation: `orbit-spin ${active ? "1.1s" : "9s"} linear infinite`,
        }}
      >
        <circle cx="16" cy="3.5" r="2.4" fill="var(--glow-b)" />
      </g>
    </svg>
  );
}
