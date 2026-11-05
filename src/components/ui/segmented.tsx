import type { ReactNode } from "react";

import { cn } from "@/lib/cn";

interface SegmentedOption<T extends string> {
  value: T;
  /** Text, or the accessible name when `icon` is given. */
  label: string;
  /** Icon-only item: the label becomes its tooltip. */
  icon?: ReactNode;
}

interface SegmentedProps<T extends string> {
  label: string;
  value: T;
  options: readonly SegmentedOption<T>[];
  onChange: (value: T) => void;
}

/** Pill segmented control (`l8-seg`); the selected item fills with flare. */
export function Segmented<T extends string>({ label, value, options, onChange }: SegmentedProps<T>) {
  return (
    <div role="radiogroup" aria-label={label} className="l8-seg">
      {options.map((option) => (
        <button
          key={option.value}
          type="button"
          role="radio"
          aria-checked={option.value === value}
          title={option.icon ? option.label : undefined}
          aria-label={option.icon ? option.label : undefined}
          onClick={() => onChange(option.value)}
          className={cn("l8-seg-item", option.value === value && "is-on", option.icon && "is-icon")}
        >
          {option.icon ?? option.label}
        </button>
      ))}
    </div>
  );
}
