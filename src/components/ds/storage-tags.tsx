import { useLingui } from "@lingui/react/macro";

import { DsIcon } from "@/components/ds/ds-icon";
import { cn } from "@/lib/cn";

export type CloudCopyState = boolean | "uploading" | "pending";

interface StorageTagsProps {
  local: boolean;
  cloud: CloudCopyState;
  compact?: boolean;
  className?: string;
}

/** Where a snapshot lives: one tag per copy, each with its own icon and word. */
export function StorageTags({ local, cloud, compact = false, className }: StorageTagsProps) {
  const { t } = useLingui();
  return (
    <span className={cn("l8-stags", className)}>
      {local && (
        <span className="l8-stag l8-stag-local" title={t`Stored on this PC`}>
          <DsIcon name="drive" size={14} />
          {compact ? null : t`PC`}
        </span>
      )}
      {cloud === true && (
        <span className="l8-stag l8-stag-cloud" title={t`Stored in Google Drive`}>
          <DsIcon name="cloud" size={14} />
          {compact ? null : t`Drive`}
        </span>
      )}
      {cloud === "uploading" && (
        <span className="l8-stag l8-stag-cloud is-busy" title={t`Uploading to Google Drive`}>
          <DsIcon name="cloudUp" size={14} className="l8-pulse" />
          {compact ? null : t`Uploading`}
        </span>
      )}
      {cloud === "pending" && (
        <span className="l8-stag l8-stag-pending" title={t`Waiting to upload`}>
          <DsIcon name="clock" size={14} />
          {compact ? null : t`Queued`}
        </span>
      )}
    </span>
  );
}

type ProtectionPart = boolean | "pending" | "conflict";

interface ProtectionProps {
  local: ProtectionPart;
  cloud: ProtectionPart;
  tone?: "plain" | "stage";
  showLabel?: boolean;
  label?: string;
  className?: string;
}

/** The protection indicator: two halves, this PC (left) and Google Drive (right). */
export function Protection({ local, cloud, tone = "plain", showLabel = false, label, className }: ProtectionProps) {
  const { t } = useLingui();
  const words = `${local === true ? t`On this PC` : t`Not on this PC`} · ${
    cloud === true
      ? t`In Google Drive`
      : cloud === "pending"
        ? t`Upload pending`
        : cloud === "conflict"
          ? t`Sync conflict`
          : t`Not in Google Drive`
  }`;
  const part = (kind: "local" | "cloud", value: ProtectionPart) => (
    <i
      className={cn("l8-prot-part", `l8-prot-${kind}`, `is-${value === true ? "on" : value ? String(value) : "off"}`)}
    />
  );
  return (
    <span
      className={cn("l8-prot", tone === "stage" && "l8-prot-stage", className)}
      title={words}
      role="img"
      aria-label={words}
    >
      <span className="l8-prot-bar">
        {part("local", local)}
        {part("cloud", cloud)}
      </span>
      {showLabel ? <span className="l8-prot-label">{label ?? words}</span> : null}
    </span>
  );
}
