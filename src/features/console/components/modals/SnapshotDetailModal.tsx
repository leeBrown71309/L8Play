import { useLingui } from "@lingui/react/macro";

import { ListRow } from "@/features/console/components/ListRow";
import { ModalFrame } from "@/features/console/components/modals/ModalFrame";
import { storedIn } from "@/features/console/components/modals/snapshot-text";
import { useConsoleNav } from "@/features/console/core/nav-store";
import { useModalList } from "@/features/console/core/use-modal-list";
import { useSnapshotLabels } from "@/features/saves/snapshot-labels";
import { useSnapshots } from "@/features/saves/use-saves";
import { formatBytes, formatDateTime } from "@/lib/format";

/** What a snapshot is, and the way to restore it. Restoring always goes through the safe confirmation next. */
export function SnapshotDetailModal({ snapshotId }: { snapshotId: string }) {
  const { t, i18n } = useLingui();
  const labels = useSnapshotLabels();
  const dispatch = useConsoleNav((state) => state.dispatch);
  const snapshot = useSnapshots(null).data?.find((entry) => entry.id === snapshotId);
  const where = { pc: t`This PC`, drive: t`Google Drive`, both: t`PC and Drive` };

  const items = [
    {
      label: t`Restore…`,
      icon: "restore" as const,
      run: () => dispatch({ type: "pushModal", modal: { kind: "confirm-restore", snapshotId } }),
    },
    { label: t`Close`, icon: "x" as const, run: () => dispatch({ type: "popModal" }) },
  ];
  const list = useModalList({
    count: items.length,
    onConfirm: (index) => items[index]?.run(),
    onBack: () => dispatch({ type: "popModal" }),
  });
  if (!snapshot) return null;

  const facts: [string, string][] = [
    [t`Type`, labels.kind(snapshot.kind)],
    [t`Created`, formatDateTime(snapshot.createdAt, i18n.locale)],
    [
      t`Size`,
      `${formatBytes(snapshot.archiveSizeBytes, i18n.locale)} · ${labels.contents(snapshot.totalFiles, formatBytes(snapshot.totalBytes, i18n.locale))}`,
    ],
    [t`Stored in`, storedIn(snapshot, where)],
    [t`Device`, snapshot.deviceName],
  ];

  return (
    <ModalFrame
      title={labels.name(snapshot)}
      subtitle={snapshot.gameTitle}
      icon="snapshot"
      wide
      footer={
        <>
          {items.map((item, index) => (
            <ListRow
              key={item.label}
              label={item.label}
              icon={item.icon}
              focused={list.index === index}
              onFocus={() => list.setIndex(index)}
              onActivate={item.run}
            />
          ))}
        </>
      }
    >
      <dl className="cs-facts">
        {facts.map(([label, value]) => (
          <div key={label}>
            <dt>{label}</dt>
            <dd>{value}</dd>
          </div>
        ))}
      </dl>
    </ModalFrame>
  );
}
