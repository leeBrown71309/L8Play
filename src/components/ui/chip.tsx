import type { ReactNode } from "react";

import { cn } from "@/lib/cn";

interface ChipProps {
  selected: boolean;
  onClick: () => void;
  icon?: ReactNode;
  count?: number;
  children: ReactNode;
}

/** Pill filter chip (`l8-chip`); the selected chip inverts to ink on the canvas colour. */
export function Chip({ selected, onClick, icon, count, children }: ChipProps) {
  return (
    <button type="button" aria-pressed={selected} onClick={onClick} className={cn("l8-chip", selected && "is-on")}>
      {icon}
      {children}
      {count !== undefined && <span className="l8-chip-count">{count}</span>}
    </button>
  );
}
