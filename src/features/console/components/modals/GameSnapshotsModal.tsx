import { useLingui } from "@lingui/react/macro";
import { useMemo } from "react";

import { ListRow } from "@/features/console/components/ListRow";
import { ModalFrame } from "@/features/console/components/modals/ModalFrame";
import { newestFirst, storedIn } from "@/features/console/components/modals/snapshot-text";
import { useConsoleNav } from "@/features/console/core/nav-store";
import { useModalList } from "@/features/console/core/use-modal-list";
import { useSnapshotLabels } from "@/features/saves/snapshot-labels";
import { useSnapshots } from "@/features/saves/use-saves";
import { formatBytes, formatRelative } from "@/lib/format";

/** The snapshots of the open game; confirming one shows its details and the restore choice. */
export function GameSnapshotsModal() {
  const { t, i18n } = useLingui();
  const labels = useSnapshotLabels();
  const gameId = useConsoleNav((state) => state.gameId);
  const dispatch = useConsoleNav((state) => state.dispatch);
  const data = useSnapshots(gameId).data;
  const snapshots = useMemo(() => newestFirst(data ?? []), [data]);
  const where = { pc: t`This PC`, drive: t`Google Drive`, both: t`PC and Drive` };

  const list = useModalList({
    count: snapshots.length,
    onConfirm: (index) => {
      const snapshot = snapshots[index];
      if (snapshot) dispatch({ type: "pushModal", modal: { kind: "snapshot-detail", snapshotId: snapshot.id } });
    },
    onBack: () => dispatch({ type: "popModal" }),
  });

  return (
    <ModalFrame title={t`Snapshots`} subtitle={snapshots[0]?.gameTitle} icon="snapshot" wide>
      {snapshots.length === 0 ? <p className="cs-empty">{t`No snapshot yet. Create one from the game page.`}</p> : null}
      <div className="cs-modal-list">
        {snapshots.map((snapshot, index) => (
          <ListRow
            key={snapshot.id}
            label={labels.name(snapshot)}
            detail={`${labels.kind(snapshot.kind)} · ${formatRelative(snapshot.createdAt, i18n.locale)} · ${formatBytes(snapshot.archiveSizeBytes, i18n.locale)} · ${storedIn(snapshot, where)}`}
            icon="snapshot"
            focused={list.index === index}
            onFocus={() => list.setIndex(index)}
            onActivate={() =>
              dispatch({ type: "pushModal", modal: { kind: "snapshot-detail", snapshotId: snapshot.id } })
            }
          />
        ))}
      </div>
    </ModalFrame>
  );
}
