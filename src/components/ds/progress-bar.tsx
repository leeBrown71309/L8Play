import type { ReactNode } from "react";

import { cn } from "@/lib/cn";

type ProgressTone = "flare" | "cloud" | "cream" | "success" | "warning" | "local";

interface ProgressBarProps {
  /** 0–100. Ignored when `indeterminate`. */
  value?: number;
  tone?: ProgressTone;
  size?: "sm" | "md";
  label?: ReactNode;
  meta?: ReactNode;
  indeterminate?: boolean;
  className?: string;
}

/** Pill progress bar with an optional label row. */
export function ProgressBar({
  value = 0,
  tone = "flare",
  size = "md",
  label,
  meta,
  indeterminate,
  className,
}: ProgressBarProps) {
  const clamped = Math.max(0, Math.min(100, value));
  return (
    <div className={cn("l8-progress", `tone-${tone}`, size === "sm" && "l8-progress-sm", className)}>
      {label || meta ? (
        <div className="l8-progress-head">
          <span className="l8-progress-label">{label}</span>
          <span className="l8-progress-meta">{meta}</span>
        </div>
      ) : null}
      <div
        className="l8-progress-track"
        role="progressbar"
        aria-valuemin={0}
        aria-valuemax={100}
        {...(indeterminate ? {} : { "aria-valuenow": clamped })}
        aria-label={typeof label === "string" ? label : undefined}
      >
        <div
          className={cn("l8-progress-fill", indeterminate && "is-indeterminate")}
          style={indeterminate ? undefined : { width: `${clamped}%` }}
        />
      </div>
    </div>
  );
}

interface StorageSegment {
  label: string;
  /** Amount in the same unit as `total`. */
  value: number;
  display: string;
  tone: "flare" | "other";
}

interface StorageMeterProps {
  total: number;
  segments: StorageSegment[];
  freeLabel?: string;
  /** Accessible summary, e.g. "4.82 of 15 GB used". */
  summary: string;
  className?: string;
  freeText: string;
}

/** Stacked storage meter with a legend. */
export function StorageMeter({ total, segments, freeLabel, summary, freeText, className }: StorageMeterProps) {
  return (
    <div className={cn("l8-meter", className)}>
      <div className="l8-meter-track" role="img" aria-label={summary}>
        {segments.map((segment) => (
          <i
            key={segment.label}
            className={`l8-meter-seg tone-${segment.tone}`}
            style={{ width: `${total > 0 ? (segment.value / total) * 100 : 0}%` }}
          />
        ))}
      </div>
      <div className="l8-meter-legend">
        {segments.map((segment) => (
          <span key={segment.label}>
            <i className={`l8-meter-dot tone-${segment.tone}`} />
            {segment.label}
            <b>{segment.display}</b>
          </span>
        ))}
        {freeLabel ? (
          <span>
            <i className="l8-meter-dot tone-free" />
            {freeText}
            <b>{freeLabel}</b>
          </span>
        ) : null}
      </div>
    </div>
  );
}
