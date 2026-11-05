import type { ReactNode } from "react";

import { DsIcon } from "@/components/ds/ds-icon";
import { Switch } from "@/components/ui/switch";

/** A titled group of setting rows (`h2` + rows, 8px apart). */
export function SettingGroup({ title, children }: { title?: string; children: ReactNode }) {
  return (
    <section className="st-grp">
      {title ? <h2>{title}</h2> : null}
      {children}
    </section>
  );
}

interface SettingRowProps {
  title: string;
  description?: ReactNode;
  control?: ReactNode;
}

/** A card row: title and description on the left, the control on the right. */
export function SettingRow({ title, description, control }: SettingRowProps) {
  return (
    <div className="st-row">
      <div className="grow">
        <div className="t">{title}</div>
        {description ? <div className="d">{description}</div> : null}
      </div>
      {control ? <div className="controls">{control}</div> : null}
    </div>
  );
}

interface ToggleSettingProps {
  title: string;
  description: string;
  /** An extra line under the description (a state hint). */
  note?: ReactNode;
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
}

/** An on/off row that takes effect immediately (the design system's `Toggle` with a label). */
export function ToggleSetting({ title, description, note, checked, onChange, disabled = false }: ToggleSettingProps) {
  return (
    <label className={disabled ? "l8-setting is-disabled" : "l8-setting"}>
      <span className="l8-setting-text">
        <span className="l8-setting-label">{title}</span>
        <span className="l8-setting-desc">{description}</span>
        {note ? <span className="l8-setting-desc st-note">{note}</span> : null}
      </span>
      <Switch label={title} checked={checked} onChange={onChange} disabled={disabled} />
    </label>
  );
}

/** "Always on" marker for protections that cannot be turned off. */
export function AlwaysOn({ label }: { label: string }) {
  return (
    <span className="st-lock">
      <DsIcon name="lock" size={14} />
      {label}
    </span>
  );
}
