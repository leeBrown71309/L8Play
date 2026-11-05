import type { ButtonHTMLAttributes, ReactNode } from "react";

import { DsIcon, type DsIconName } from "@/components/ds/ds-icon";
import { Tooltip } from "@/components/ui/tooltip";
import { cn } from "@/lib/cn";

interface IconButtonProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, "aria-label"> {
  /** Required: becomes the accessible name and the tooltip. */
  label: string;
  /** A design-system icon name, or an icon element. */
  icon: DsIconName | ReactNode;
  variant?: "ghost" | "secondary" | "primary" | "cream" | "stage";
  size?: "sm" | "md" | "lg";
  /** Pressed state (favourite stars, toggles). */
  active?: boolean;
  badge?: boolean;
  tooltipSide?: "top" | "right" | "bottom" | "left";
}

const GLYPH_SIZE = { sm: 16, md: 18, lg: 22 } as const;

/** Round icon-only button (`l8-ibtn`) with a tooltip. */
export function IconButton({
  label,
  icon,
  variant = "ghost",
  size = "md",
  active,
  badge = false,
  tooltipSide = "bottom",
  className,
  ...props
}: IconButtonProps) {
  return (
    <Tooltip content={label} side={tooltipSide}>
      <button
        type="button"
        aria-label={label}
        aria-pressed={active ?? undefined}
        className={cn("l8-ibtn", `l8-ibtn-${variant}`, `l8-ibtn-${size}`, active && "is-on", className)}
        {...props}
      >
        {typeof icon === "string" ? <DsIcon name={icon as DsIconName} size={GLYPH_SIZE[size]} /> : icon}
        {badge ? <i className="l8-ibtn-badge" /> : null}
      </button>
    </Tooltip>
  );
}
