import { useLingui } from "@lingui/react/macro";
import { useEffect, useRef, useState } from "react";

import { DsIcon, type DsIconName } from "@/components/ds/ds-icon";
import { IconButton } from "@/components/ui/icon-button";
import { cn } from "@/lib/cn";

export interface SnapshotMenuItem {
  id: string;
  label: string;
  icon: DsIconName;
  danger?: boolean;
  divider?: boolean;
  onSelect: () => void;
}

/** "More snapshot actions" button with a floating menu; Esc or an outside click closes it. */
export function SnapshotMenu({ items }: { items: SnapshotMenuItem[] }) {
  const { t } = useLingui();
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onPointer = (event: PointerEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    document.addEventListener("pointerdown", onPointer);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("pointerdown", onPointer);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  return (
    <div ref={root} className="relative">
      <IconButton
        icon="more"
        label={t`More snapshot actions`}
        size="sm"
        onClick={(event) => {
          event.stopPropagation();
          setOpen(!open);
        }}
      />
      {open ? (
        <div className="l8-menu is-right absolute top-full right-0 z-30 mt-1" role="menu">
          {items.map((item) => (
            <div key={item.id}>
              {item.divider ? <div className="l8-menu-div" role="separator" /> : null}
              <div
                role="menuitem"
                tabIndex={0}
                className={cn("l8-menu-item", item.danger && "is-danger")}
                onClick={(event) => {
                  event.stopPropagation();
                  setOpen(false);
                  item.onSelect();
                }}
                onKeyDown={(event) => {
                  if (event.key === "Enter" || event.key === " ") {
                    event.preventDefault();
                    setOpen(false);
                    item.onSelect();
                  }
                }}
              >
                <DsIcon name={item.icon} size={16} />
                <span className="l8-menu-label">{item.label}</span>
              </div>
            </div>
          ))}
        </div>
      ) : null}
    </div>
  );
}
