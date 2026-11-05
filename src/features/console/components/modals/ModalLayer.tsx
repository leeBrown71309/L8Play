import { ConfirmRestoreModal } from "@/features/console/components/modals/ConfirmRestoreModal";
import { GameSnapshotsModal } from "@/features/console/components/modals/GameSnapshotsModal";
import { LetterPicker } from "@/features/console/components/modals/LetterPicker";
import { QuickMenu } from "@/features/console/components/modals/QuickMenu";
import { SnapshotDetailModal } from "@/features/console/components/modals/SnapshotDetailModal";
import { useConsoleNav } from "@/features/console/core/nav-store";
import { cn } from "@/lib/cn";

/**
 * Renders the dialog stack. Every dialog keeps its own input hook, mounted after the console root's, so the topmost
 * one owns the buttons; the ones underneath stay mounted but dimmed so going back is instant.
 */
export function ModalLayer() {
  const modals = useConsoleNav((state) => state.modals);
  return (
    <>
      {modals.map((modal, index) => {
        const top = index === modals.length - 1;
        return (
          <div key={`${modal.kind}-${index}`} className={cn("cs-modal-layer", !top && "is-under")} inert={!top}>
            {modal.kind === "quick-menu" ? <QuickMenu /> : null}
            {modal.kind === "letters" ? <LetterPicker /> : null}
            {modal.kind === "game-snapshots" ? <GameSnapshotsModal /> : null}
            {modal.kind === "snapshot-detail" && modal.snapshotId ? (
              <SnapshotDetailModal snapshotId={modal.snapshotId} />
            ) : null}
            {modal.kind === "confirm-restore" && modal.snapshotId ? (
              <ConfirmRestoreModal snapshotId={modal.snapshotId} />
            ) : null}
          </div>
        );
      })}
    </>
  );
}
