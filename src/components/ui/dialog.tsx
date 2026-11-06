import * as DialogPrimitive from "@radix-ui/react-dialog";
import { useLingui } from "@lingui/react/macro";
import type { ReactNode } from "react";

import { DsIcon } from "@/components/ds/ds-icon";
import { cn } from "@/lib/cn";

type Tone = "flare" | "warning" | "danger" | "success" | "cloud";

interface DialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description?: string | undefined;
  icon: ReactNode;
  tone?: Tone;
  width?: number;
  /** Keyboard hint shown left of the actions. */
  hint?: ReactNode;
  actions: ReactNode;
  children: ReactNode;
  /** `false` ignores clicks outside the dialog (work in progress the user must not lose by mistake). */
  dismissOnOutsideClick?: boolean;
}

/**
 * Modal dialog from the design system (`l8-dialog`): tone-tinted icon tile, title, body, and a
 * footer with a hint and the actions. Esc and the close button cancel; the last action confirms.
 */
export function Dialog({
  open,
  onOpenChange,
  title,
  description,
  icon,
  tone = "flare",
  width = 580,
  hint,
  actions,
  children,
  dismissOnOutsideClick = true,
}: DialogProps) {
  const { t } = useLingui();
  return (
    <DialogPrimitive.Root open={open} onOpenChange={onOpenChange}>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay className="l8-scrim fixed inset-0 z-40 bg-scrim backdrop-blur-[4px]" />
        <DialogPrimitive.Content
          style={{ width }}
          onInteractOutside={(event) => {
            if (!dismissOnOutsideClick) event.preventDefault();
          }}
          className="l8-dialog fixed top-1/2 left-1/2 z-50 -translate-x-1/2 -translate-y-1/2"
        >
          <header className="l8-dialog-head">
            <span className={cn("l8-dialog-icon", `tone-${tone}`)}>{icon}</span>
            <div className="l8-dialog-titles">
              <DialogPrimitive.Title asChild>
                <h2>{title}</h2>
              </DialogPrimitive.Title>
              {description ? (
                <DialogPrimitive.Description asChild>
                  <p>{description}</p>
                </DialogPrimitive.Description>
              ) : (
                <DialogPrimitive.Description className="sr-only">{title}</DialogPrimitive.Description>
              )}
            </div>
            <DialogPrimitive.Close
              aria-label={t`Close`}
              title={t`Close (Esc)`}
              className="l8-ibtn l8-ibtn-sm l8-ibtn-secondary l8-dialog-close"
            >
              <DsIcon name="x" size={16} />
            </DialogPrimitive.Close>
          </header>
          <div className="l8-dialog-body">{children}</div>
          <footer className="l8-dialog-foot">
            <div className="l8-dialog-hint">{hint}</div>
            <div className="l8-dialog-actions">{actions}</div>
          </footer>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}

/** Keyboard key hint (`l8-keys`). */
export function Kbd({ children }: { children: ReactNode }) {
  return (
    <span className="l8-keys">
      <kbd>{children}</kbd>
    </span>
  );
}
