import { useLingui } from "@lingui/react/macro";

import type { DsIconName } from "@/components/ds/ds-icon";
import { useCloudStatus } from "@/features/cloud/use-cloud";

export type CloudSummaryState = "none" | "idle" | "syncing" | "attention";

export interface CloudSummary {
  state: CloudSummaryState;
  /** Tone of the top-bar sync button. */
  tone: "success" | "cloud" | "warning" | "danger" | "neutral";
  icon: DsIconName;
  label: string;
  /** Something needs the user: a conflict, a sign-in, failed uploads. */
  attention: boolean;
  /** Share of the Google storage in use (0–100), when known. */
  usedPercent: number | null;
  usedBytes: number | null;
}

/** One reading of the cloud state for the top bar, the navigation rail and the quick-launch rail. */
export function useCloudSummary(): CloudSummary {
  const { t } = useLingui();
  const status = useCloudStatus().data;
  const account = status?.account ?? null;
  const quota = status?.quota ?? null;
  const usedPercent = quota?.limitBytes ? Math.min(100, (quota.usedBytes / quota.limitBytes) * 100) : null;
  const base = { usedPercent, usedBytes: quota?.usedBytes ?? null };

  if (!account) {
    return {
      ...base,
      state: "none",
      tone: "neutral",
      icon: "cloudOff",
      label: t`Saves are kept on this PC only`,
      attention: false,
    };
  }
  if (account.status === "needs_reauth") {
    return {
      ...base,
      state: "attention",
      tone: "warning",
      icon: "alert",
      label: t`Sign in to Google Drive again`,
      attention: true,
    };
  }
  const conflicts = status?.openConflicts ?? 0;
  if (conflicts > 0) {
    return {
      ...base,
      state: "attention",
      tone: "warning",
      icon: "merge",
      label: t`Sync conflicts: ${conflicts}`,
      attention: true,
    };
  }
  const failed = status?.failed ?? 0;
  if (failed > 0) {
    return {
      ...base,
      state: "attention",
      tone: "danger",
      icon: "alertCircle",
      label: t`Uploads failed: ${failed}`,
      attention: true,
    };
  }
  const active = (status?.queued ?? 0) + (status?.running ?? 0);
  if (active > 0) {
    return { ...base, state: "syncing", tone: "cloud", icon: "cloudUp", label: t`Syncing ${active}`, attention: false };
  }
  return { ...base, state: "idle", tone: "success", icon: "cloudCheck", label: t`All saves in sync`, attention: false };
}
