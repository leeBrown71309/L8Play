import { useLingui } from "@lingui/react/macro";
import { useLayoutEffect, useRef } from "react";

import { DsIcon } from "@/components/ds/ds-icon";
import { useCloudSummary } from "@/components/shell/use-cloud-summary";
import { useConsoleNav } from "@/features/console/core/nav-store";
import { SECTIONS, type ConsoleSection } from "@/features/console/core/navigation";
import { useConsoleClock } from "@/features/console/core/use-console-clock";
import { useGamepadStatus } from "@/features/console/input";
import { cn } from "@/lib/cn";

/** Tabs with a sliding underline, the controller, the sync state and the clock. */
export function ConsoleTopBar() {
  const { t, i18n } = useLingui();
  const section = useConsoleNav((state) => state.section);
  const dispatch = useConsoleNav((state) => state.dispatch);
  const clock = useConsoleClock(i18n.locale);
  const pad = useGamepadStatus();
  const cloud = useCloudSummary();
  const tabs = useRef<HTMLDivElement>(null);

  const labels: Record<ConsoleSection, string> = {
    library: t`Library`,
    saves: t`Saves`,
    cloud: t`Cloud`,
    settings: t`Settings`,
  };

  // One measurement per section change: the underline then slides with a plain CSS transition.
  useLayoutEffect(() => {
    const container = tabs.current;
    const active = container?.querySelector<HTMLElement>("[aria-selected='true']");
    if (!container || !active) return;
    container.style.setProperty("--tab-x", `${active.offsetLeft}px`);
    container.style.setProperty("--tab-w", `${active.offsetWidth}px`);
  }, [section, i18n.locale]);

  return (
    <header className="cs-topbar">
      <div ref={tabs} className="cs-tabs" role="tablist" aria-label={t`Console sections`}>
        {SECTIONS.map((id) => (
          <button
            key={id}
            type="button"
            role="tab"
            aria-selected={id === section}
            className={cn("cs-tab", id === section && "is-on")}
            onClick={() => dispatch({ type: "gotoSection", section: id })}
          >
            {labels[id]}
          </button>
        ))}
        <i className="cs-tab-line" aria-hidden="true" />
      </div>
      <div className="cs-status">
        <span
          className={cn("cs-pad", pad.connected && "is-on")}
          title={pad.connected ? (pad.name ?? t`Controller connected`) : t`No controller connected`}
        >
          <DsIcon name="gamepad" size={22} />
          <span className="cs-pad-name">{pad.connected ? (pad.name ?? t`Controller`) : t`No controller`}</span>
        </span>
        <span className={cn("cs-sync", `tone-${cloud.tone}`)} title={cloud.label}>
          <DsIcon name={cloud.icon} size={22} className={cloud.state === "syncing" ? "l8-pulse" : undefined} />
        </span>
        <time key={clock} className="cs-clock">
          {clock}
        </time>
      </div>
    </header>
  );
}
