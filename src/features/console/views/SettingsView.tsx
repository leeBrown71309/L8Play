import { useLingui } from "@lingui/react/macro";
import { useRef } from "react";

import { ListRow } from "@/features/console/components/ListRow";
import { useZoneFocus } from "@/features/console/core/nav-store";
import { useScrollFocused } from "@/features/console/core/use-scroll-focused";
import { useViewInput } from "@/features/console/core/view-input";
import { consoleSound } from "@/features/console/sound";
import { cycleOption, stepVolume, type SettingsRow } from "@/features/console/views/settings-rows";
import { useSettingsRows } from "@/features/console/views/use-settings-rows";
import { cn } from "@/lib/cn";

function Trailing({ row }: { row: SettingsRow }) {
  switch (row.kind) {
    case "toggle":
      return (
        <span className={cn("cs-switch", row.value && "is-on")} aria-hidden="true">
          <i />
        </span>
      );
    case "picker":
      return (
        <span className="cs-picker-value" aria-hidden="true">
          ‹ <b>{row.options.find((option) => option.value === row.value)?.label ?? row.value}</b> ›
        </span>
      );
    case "slider":
      return (
        <span className="cs-slider" aria-hidden="true">
          <span className="cs-slider-track">
            <i style={{ width: `${Math.round(row.value * 100)}%` }} />
          </span>
          <b>{Math.round(row.value * 100)}%</b>
        </span>
      );
    default:
      return null;
  }
}

/** A vertical list: left and right change a value in place, confirm flips a switch or runs an action. */
export function SettingsView() {
  const { t } = useLingui();
  const rows = useSettingsRows();
  const zone = useZoneFocus("settings.rows", rows.length);
  const list = useRef<HTMLDivElement>(null);
  useScrollFocused(list, zone.index);

  const adjust = (row: SettingsRow, delta: 1 | -1) => {
    if (row.kind === "toggle" && !row.disabled && row.value !== (delta === 1)) {
      consoleSound.play("toggle");
      row.set(delta === 1);
    } else if (row.kind === "picker") {
      const next = cycleOption(row.options, row.value, delta);
      if (next) {
        consoleSound.play("move");
        row.set(next.value);
      }
    } else if (row.kind === "slider" && !row.disabled) {
      row.set(stepVolume(row.value, delta));
      consoleSound.play("move");
    }
  };

  const activate = (row: SettingsRow | undefined) => {
    if (!row) return;
    if (row.kind === "toggle" && !row.disabled) {
      consoleSound.play("toggle");
      row.set(!row.value);
    } else if (row.kind === "picker") adjust(row, 1);
    else if (row.kind === "action" && !row.disabled) {
      consoleSound.play("select");
      row.run();
    }
  };

  useViewInput((action) => {
    const row = rows[zone.index];
    if (action === "up" || action === "down") {
      const next = Math.min(rows.length - 1, Math.max(0, zone.index + (action === "down" ? 1 : -1)));
      if (next !== zone.index) consoleSound.play("move");
      zone.set(next);
    } else if (row && (action === "left" || action === "right")) adjust(row, action === "right" ? 1 : -1);
    else if (action === "confirm") activate(row);
  });

  return (
    <section className="cs-section" aria-label={t`Settings`}>
      <h1 className="cs-section-title">{t`Settings`}</h1>
      <div ref={list} className="cs-scroll">
        {rows.map((row, index) => (
          <ListRow
            key={row.id}
            label={row.label}
            detail={"detail" in row ? row.detail : undefined}
            icon={row.icon}
            disabled={"disabled" in row ? row.disabled : undefined}
            focused={zone.index === index}
            trailing={<Trailing row={row} />}
            onFocus={() => zone.set(index)}
            onActivate={() => activate(row)}
          />
        ))}
      </div>
    </section>
  );
}
