import type { ReactNode } from "react";

import { pointerMoved } from "@/features/console/core/pointer-move";
import { DsIcon, type DsIconName } from "@/components/ds/ds-icon";
import { cn } from "@/lib/cn";

interface ListRowProps {
  label: string;
  detail?: ReactNode;
  icon?: DsIconName;
  /** Right-hand side: a value, a switch, arrows… */
  trailing?: ReactNode;
  focused: boolean;
  disabled?: boolean | undefined;
  tone?: "default" | "danger" | undefined;
  /** The pointer moved over the row: let the focus follow it. */
  onFocus: () => void;
  onActivate: () => void;
}

/** One focusable row. The pad moves the focus; the mouse can hover to focus and click to confirm. */
export function ListRow({ label, detail, icon, trailing, focused, disabled, tone, onFocus, onActivate }: ListRowProps) {
  return (
    <div
      role="button"
      tabIndex={-1}
      aria-disabled={disabled || undefined}
      data-focused={focused || undefined}
      aria-label={label}
      className={cn("cs-row", focused && "is-focused", disabled && "is-disabled", tone === "danger" && "is-danger")}
      onPointerMove={(event) => !focused && pointerMoved(event) && onFocus()}
      onClick={() => !disabled && onActivate()}
    >
      {icon ? (
        <span className="cs-row-icon">
          <DsIcon name={icon} size={26} />
        </span>
      ) : null}
      <span className="cs-row-text">
        <b>{label}</b>
        {detail ? <span>{detail}</span> : null}
      </span>
      {trailing ? <span className="cs-row-trailing">{trailing}</span> : null}
    </div>
  );
}
