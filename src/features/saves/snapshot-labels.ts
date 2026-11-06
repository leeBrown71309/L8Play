import { useLingui } from "@lingui/react/macro";

import type { SnapshotKind } from "@/lib/bindings/SnapshotKind";
import type { SnapshotSummary } from "@/lib/bindings/SnapshotSummary";
import { AppError } from "@/lib/tauri/errors";

/** Translated labels for snapshot kinds, display names and backend errors. */
export function useSnapshotLabels() {
  const { t } = useLingui();
  const kind = (value: SnapshotKind): string =>
    ({
      manual: t`Manual`,
      auto_exit: t`Auto`,
      auto_launch: t`Auto`,
      scheduled: t`Scheduled`,
      safety: t`Safety copy`,
      conflict_copy: t`Conflict copy`,
    })[value];

  /** Automatic snapshots are stored without a name; they are described by what triggered them. */
  const name = (snapshot: SnapshotSummary): string => {
    if (snapshot.name.trim() !== "") return snapshot.name;
    if (snapshot.label) return snapshot.label;
    if (snapshot.imported) return t`Imported backup`;
    return {
      manual: t`Snapshot`,
      auto_exit: t`Game closed`,
      auto_launch: t`Before launch`,
      scheduled: t`While playing`,
      safety: t`Safety copy before restore`,
      conflict_copy: t`Conflict copy`,
    }[snapshot.kind];
  };

  /** A short, user-facing reason for a failed save operation. */
  const error = (failure: unknown): string => {
    if (!(failure instanceof AppError)) return t`Something went wrong. Check the log file for details.`;
    switch (failure.code) {
      case "busy":
        return t`The game or another program is still using its save files. Close it and try again.`;
      case "integrity":
        return t`This snapshot is damaged and was not used. Your current save is untouched.`;
      case "external":
        return t`The save detector could not run. Reinstall L8Play if this keeps happening.`;
      case "validation":
      case "not_found":
        return failure.message;
      default:
        return t`Something went wrong. Check the log file for details.`;
    }
  };

  /** "1 file · 9.8 MB" / "12 files · 9.8 MB". */
  const contents = (files: number, size: string): string =>
    files === 1 ? t`1 file · ${size}` : t`${files} files · ${size}`;

  return { kind, name, error, contents };
}
