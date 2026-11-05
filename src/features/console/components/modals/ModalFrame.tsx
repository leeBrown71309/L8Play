import type { ReactNode } from "react";

import { DsIcon, type DsIconName } from "@/components/ds/ds-icon";
import { cn } from "@/lib/cn";

interface ModalFrameProps {
  title: string;
  subtitle?: string | undefined;
  icon: DsIconName;
  tone?: "flare" | "warning" | "cloud";
  wide?: boolean;
  /** Scrolls inside the dialog when it is taller than the screen. */
  children?: ReactNode;
  /** The choices (buttons): they stay visible however long the body is. */
  footer?: ReactNode;
}

/** The shared look of every console dialog: a glass panel that scales and fades in over a dimmed screen. */
export function ModalFrame({ title, subtitle, icon, tone = "flare", wide = false, children, footer }: ModalFrameProps) {
  return (
    <div className="cs-modal-scrim">
      <section className={cn("cs-modal", wide && "is-wide")} role="dialog" aria-modal="true" aria-label={title}>
        <header className="cs-modal-head">
          <span className={cn("cs-modal-icon", `tone-${tone}`)}>
            <DsIcon name={icon} size={30} />
          </span>
          <div>
            <h2>{title}</h2>
            {subtitle ? <p>{subtitle}</p> : null}
          </div>
        </header>
        {children ? <div className="cs-modal-body">{children}</div> : null}
        {footer ? <div className="cs-modal-foot">{footer}</div> : null}
      </section>
    </div>
  );
}
