import { useLingui } from "@lingui/react/macro";
import type { ReactNode } from "react";

import { DsIcon, type DsIconName } from "@/components/ds/ds-icon";
import { cn } from "@/lib/cn";

export type StatusKind =
  | "synced"
  | "created"
  | "verified"
  | "uploading"
  | "downloading"
  | "pending"
  | "conflict"
  | "unprotected"
  | "error"
  | "offline"
  | "local"
  | "cloud"
  | "manual"
  | "auto"
  | "safety"
  | "info";

type Tone = "success" | "cloud" | "warning" | "danger" | "neutral" | "local" | "flare";

const STATUS: Record<StatusKind, { tone: Tone; icon: DsIconName | null }> = {
  synced: { tone: "success", icon: "cloudCheck" },
  created: { tone: "success", icon: "checkCircle" },
  verified: { tone: "success", icon: "shieldCheck" },
  uploading: { tone: "cloud", icon: "cloudUp" },
  downloading: { tone: "cloud", icon: "cloudDown" },
  pending: { tone: "warning", icon: "clock" },
  conflict: { tone: "warning", icon: "merge" },
  unprotected: { tone: "warning", icon: "alert" },
  error: { tone: "danger", icon: "alertCircle" },
  offline: { tone: "neutral", icon: "cloudOff" },
  local: { tone: "local", icon: "drive" },
  cloud: { tone: "cloud", icon: "cloud" },
  manual: { tone: "flare", icon: null },
  auto: { tone: "neutral", icon: null },
  safety: { tone: "local", icon: "shield" },
  info: { tone: "neutral", icon: "info" },
};

/** Default words for each status (always an icon and a word, never colour alone). */
export function useStatusLabels(): Record<StatusKind, string> {
  const { t } = useLingui();
  return {
    synced: t`In sync`,
    created: t`Created`,
    verified: t`Verified`,
    uploading: t`Uploading`,
    downloading: t`Downloading`,
    pending: t`Waiting to upload`,
    conflict: t`Sync conflict`,
    unprotected: t`Not protected`,
    error: t`Failed`,
    offline: t`Offline`,
    local: t`This PC`,
    cloud: t`Google Drive`,
    manual: t`Manual`,
    auto: t`Auto`,
    safety: t`Safety copy`,
    info: t`Info`,
  };
}

interface StatusBadgeProps {
  status: StatusKind;
  size?: "sm" | "md";
  className?: string;
  children?: ReactNode;
}

/** Status pill: icon + word on a soft tone ground. */
export function StatusBadge({ status, size = "md", className, children }: StatusBadgeProps) {
  const labels = useStatusLabels();
  const { tone, icon } = STATUS[status];
  const busy = status === "uploading" || status === "downloading";
  return (
    <span className={cn("l8-status", `tone-${tone}`, size === "sm" && "l8-status-sm", className)}>
      {icon ? <DsIcon name={icon} size={size === "sm" ? 13 : 14} className={busy ? "l8-pulse" : undefined} /> : null}
      <span>{children ?? labels[status]}</span>
    </span>
  );
}
