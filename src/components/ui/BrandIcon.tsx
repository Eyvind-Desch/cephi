import clsx from "clsx";
import { Bot, Sparkles, Wrench } from "lucide-react";
import { siClaude, siGooglegemini, siOllama, siOpenrouter, siSpotify, type SimpleIcon } from "simple-icons";

/** Monochrome brand logos (Simple Icons, CC0) with neutral fallbacks. */
const logos: Record<string, SimpleIcon> = {
  anthropic: siClaude,
  gemini: siGooglegemini,
  openrouter: siOpenrouter,
  ollama: siOllama,
  music: siSpotify,
  spotify: siSpotify,
};

interface BrandIconProps {
  name: string;
  size?: number;
  className?: string;
}

/** Renders in `currentColor`, so it follows the surrounding text colour. */
export function BrandIcon({ name, size = 16, className }: BrandIconProps) {
  const logo = logos[name];
  if (logo) {
    return (
      <svg
        role="img"
        aria-label={logo.title}
        viewBox="0 0 24 24"
        width={size}
        height={size}
        fill="currentColor"
        className={clsx("shrink-0", className)}
      >
        <path d={logo.path} />
      </svg>
    );
  }
  const Fallback = name === "wrench" ? Wrench : name === "openai" ? Bot : Sparkles;
  return <Fallback size={size} strokeWidth={1.6} className={clsx("shrink-0", className)} />;
}
