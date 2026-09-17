import clsx from "clsx";
import type { LucideIcon } from "lucide-react";
import type { ButtonHTMLAttributes } from "react";

interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  icon: LucideIcon;
  label: string;
  active?: boolean;
  variant?: "ghost" | "accent";
}

export function IconButton({ icon: Icon, label, active, variant = "ghost", className, ...rest }: IconButtonProps) {
  return (
    <button
      type="button"
      title={label}
      aria-label={label}
      aria-pressed={active}
      className={clsx(
        "grid size-8 shrink-0 place-items-center rounded-lg transition-colors duration-150 disabled:opacity-40",
        variant === "accent"
          ? "bg-accent text-accent-fg hover:brightness-110"
          : "text-muted hover:bg-sunken hover:text-fg",
        active && variant === "ghost" && "bg-accent-soft text-accent hover:bg-accent-soft hover:text-accent",
        className,
      )}
      {...rest}
    >
      <Icon size={16} strokeWidth={2} />
    </button>
  );
}
