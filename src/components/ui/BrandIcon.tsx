import clsx from "clsx";
import { Sparkles, Wrench } from "lucide-react";
import { siClaude, siGooglegemini, siOllama, siOpenrouter, siSpotify } from "simple-icons";
import { openAi, type Logo } from "./logos";

/** Monochrome brand logos (Simple Icons, CC0, plus `logos.ts`) with neutral fallbacks. */
const logos: Record<string, Logo> = {
  anthropic: siClaude,
  gemini: siGooglegemini,
  openai: openAi,
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
        viewBox={logo.viewBox ?? "0 0 24 24"}
        width={size}
        height={size}
        fill="currentColor"
        className={clsx("shrink-0", className)}
      >
        <path d={logo.path} />
      </svg>
    );
  }
  const Fallback = name === "wrench" ? Wrench : Sparkles;
  return <Fallback size={size} className={clsx("shrink-0", className)} />;
}
