import { useLingui } from "@lingui/react/macro";

import { ListRow } from "@/features/console/components/ListRow";
import { ModalFrame } from "@/features/console/components/modals/ModalFrame";
import { useConsoleNav } from "@/features/console/core/nav-store";
import { consoleToast } from "@/features/console/core/toast-store";
import { useModalList } from "@/features/console/core/use-modal-list";
import { rumble } from "@/features/console/input";
import { consoleSound } from "@/features/console/sound";
import { useSnapshotLabels } from "@/features/saves/snapshot-labels";
import { useRestoreSnapshot, useSnapshots } from "@/features/saves/use-saves";

/**
 * The only way to restore from console mode, and it uses the same safe path as the desktop: the current save is
 * copied to a safety snapshot first. The focus starts on Cancel so a stray confirm never overwrites anything.
 */
export function ConfirmRestoreModal({ snapshotId }: { snapshotId: string }) {
  const { t } = useLingui();
  const labels = useSnapshotLabels();
  const dispatch = useConsoleNav((state) => state.dispatch);
  const snapshot = useSnapshots(null).data?.find((entry) => entry.id === snapshotId);
  const restore = useRestoreSnapshot();

  const leave = () => {
    dispatch({ type: "popModal" });
    dispatch({ type: "popModal" });
  };
  const run = () => {
    if (!snapshot || restore.isPending) return;
    const name = labels.name(snapshot);
    restore.mutate(snapshot.id, {
      onSuccess: (report) => {
        consoleSound.play("launch");
        rumble({ strong: 0.3, weak: 0.6, durationMs: 200 });
        consoleToast({
          tone: "success",
          title: t`Snapshot restored`,
          body: report.safetySnapshotId
            ? t`“${name}” is back in place. Your previous save was kept as a safety copy.`
            : t`“${name}” is back in place.`,
        });
        leave();
      },
      onError: (error) => {
        consoleSound.play("error");
        consoleToast({ tone: "danger", title: t`Nothing was restored`, body: labels.error(error) });
      },
    });
  };

  // An imported snapshot needs a destination folder to be chosen: that is only offered on the desktop.
  const needsDestination = snapshot?.imported === true;
  const cancel = { label: t`Cancel`, icon: "x" as const, run: () => dispatch({ type: "popModal" }) };
  const items = needsDestination
    ? [cancel]
    : [{ label: t`Back up current save & restore`, icon: "restore" as const, run }, cancel];
  const list = useModalList({
    count: items.length,
    initialIndex: items.length - 1,
    onConfirm: (index) => items[index]?.run(),
    onBack: () => dispatch({ type: "popModal" }),
  });
  if (!snapshot) return null;

  return (
    <ModalFrame
      title={t`Restore “${labels.name(snapshot)}”?`}
      subtitle={snapshot.gameTitle}
      icon="restore"
      tone="warning"

      footer={
        <>
          {items.map((item, index) => (
            <ListRow
              key={item.label}
              label={item.label}
              icon={item.icon}
              disabled={restore.isPending && index === 0}
              focused={list.index === index}
              onFocus={() => list.setIndex(index)}
              onActivate={item.run}
            />
          ))}
        </>
      }
    >
      <p className="cs-note">
        {needsDestination
          ? t`This snapshot was imported from an old backup. Restore it from the desktop interface, where you choose the folder it goes to.`
          : t`Your current save is copied to a safety snapshot first, then replaced by this one. The game must be closed.`}
      </p>
    </ModalFrame>
  );
}
