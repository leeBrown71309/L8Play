import { Slot } from "@radix-ui/react-slot";
import { Children, cloneElement, isValidElement } from "react";
import type { ButtonHTMLAttributes, ReactElement, ReactNode } from "react";

import { DsIcon, type DsIconName } from "@/components/ds/ds-icon";
import { cn } from "@/lib/cn";

export type ButtonVariant = "primary" | "secondary" | "ghost" | "danger" | "stage" | "play" | "light" | "cream";
export type ButtonSize = "sm" | "md" | "lg" | "xl";

const ICON_SIZE: Record<ButtonSize, number> = { sm: 16, md: 18, lg: 18, xl: 22 };

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  /** A design-system icon name, or an icon element (it gets the `l8-icon` class). */
  icon?: DsIconName | ReactNode;
  iconRight?: DsIconName;
  /** Full width. */
  block?: boolean;
  loading?: boolean;
  /** Renders the single child (e.g. a router `Link`) with button styling; icons are ignored. */
  asChild?: boolean;
}

function renderIcon(icon: DsIconName | ReactNode, size: number): ReactNode {
  if (typeof icon === "string") return <DsIcon name={icon as DsIconName} size={size} />;
  if (isValidElement(icon)) {
    const element = icon as ReactElement<{ className?: string }>;
    return cloneElement(element, { className: cn("l8-icon", element.props.className) });
  }
  return icon;
}

/**
 * Pill button from the design system (`l8-btn`). One `primary` or `play` button per view;
 * `light` and `play` wrap their leading icon in a round chip.
 */
export function Button({
  className,
  variant = "secondary",
  size = "md",
  icon,
  iconRight,
  block = false,
  loading = false,
  asChild = false,
  children,
  type,
  disabled,
  ...props
}: ButtonProps) {
  const classes = cn(
    "l8-btn",
    `l8-btn-${variant}`,
    `l8-btn-${size}`,
    block && "l8-btn-block",
    loading && "is-loading",
    className,
  );
  if (asChild) {
    return (
      <Slot className={classes} {...props}>
        {children}
      </Slot>
    );
  }
  const hasLabel = Children.count(children) > 0;
  return (
    <button
      type={type ?? "button"}
      className={classes}
      disabled={disabled || loading}
      aria-busy={loading || undefined}
      {...props}
    >
      {loading ? <span className="l8-spin" role="status" /> : renderIcon(icon, ICON_SIZE[size])}
      {hasLabel ? <span className="l8-btn-label">{children}</span> : null}
      {iconRight ? <DsIcon name={iconRight} size={ICON_SIZE[size]} /> : null}
    </button>
  );
}
