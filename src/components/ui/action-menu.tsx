import { useEffect, useRef, useState, type ReactNode } from "react";

import { DsIcon, type DsIconName } from "@/components/ds/ds-icon";
import { cn } from "@/lib/cn";

export interface ActionMenuItem {
  id: string;
  label: string;
  icon: DsIconName;
  danger?: boolean;
  divider?: boolean;
  /** Greyed out and not selectable; `hint` explains why on hover. */
  disabled?: boolean;
  hint?: string;
  onSelect: () => void;
}

interface ActionMenuProps {
  items: ActionMenuItem[];
  /** The element that opens the menu; `toggle` flips it and `open` tells whether it is showing. */
  trigger: (control: { open: boolean; toggle: () => void }) => ReactNode;
}

/** A trigger with a floating list of actions; Esc, an outside click or a selection closes it. */
export function ActionMenu({ items, trigger }: ActionMenuProps) {
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

  const select = (item: ActionMenuItem) => {
    if (item.disabled) return;
    setOpen(false);
    item.onSelect();
  };

  return (
    <div ref={root} className="relative">
      {trigger({ open, toggle: () => setOpen((current) => !current) })}
      {open ? (
        <div className="l8-menu is-right absolute top-full right-0 z-30 mt-1" role="menu">
          {items.map((item) => (
            <div key={item.id}>
              {item.divider ? <div className="l8-menu-div" role="separator" /> : null}
              <div
                role="menuitem"
                tabIndex={item.disabled ? -1 : 0}
                aria-disabled={item.disabled || undefined}
                title={item.disabled ? item.hint : undefined}
                className={cn("l8-menu-item", item.danger && "is-danger", item.disabled && "opacity-45 cursor-default")}
                onClick={(event) => {
                  event.stopPropagation();
                  select(item);
                }}
                onKeyDown={(event) => {
                  if (event.key === "Enter" || event.key === " ") {
                    event.preventDefault();
                    select(item);
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
