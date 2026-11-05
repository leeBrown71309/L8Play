import { useLingui } from "@lingui/react/macro";
import { useState } from "react";

import { DsIcon } from "@/components/ds/ds-icon";
import { OptionCard } from "@/components/ds/form-fields";
import { Banner } from "@/components/ds/page-blocks";
import { notify } from "@/components/feedback/toast-store";
import { Button } from "@/components/ui/button";
import { Dialog, Kbd } from "@/components/ui/dialog";
import { useSnapshotLabels } from "@/features/saves/snapshot-labels";
import { useDeleteSnapshot } from "@/features/saves/use-saves";
import "@/features/saves/snapshot-dialogs.css";
import type { SnapshotSummary } from "@/lib/bindings/SnapshotSummary";
import { formatDateTime } from "@/lib/format";

interface DeleteSnapshotDialogProps {
  snapshot: SnapshotSummary | null;
  onClose: () => void;
}

type Scope = "pc" | "everywhere";

/** Destructive confirmation that names exactly where the snapshot is removed from and what stays. */
export function DeleteSnapshotDialog({ snapshot, onClose }: DeleteSnapshotDialogProps) {
  const { t, i18n } = useLingui();
  const labels = useSnapshotLabels();
  const remove = useDeleteSnapshot();
  const [choice, setChoice] = useState<Scope | null>(null);
  if (!snapshot) return null;
  const name = labels.name(snapshot);

  const onPc = snapshot.status !== "missing_local";
  const inDrive = snapshot.cloud === "synced" || snapshot.cloud === "queued" || snapshot.cloud === "uploading";
  // With a Drive copy the user chooses; a snapshot that only lives in Drive can only be deleted there.
  const scope: Scope = !inDrive ? "pc" : !onPc ? "everywhere" : (choice ?? "pc");
  const alsoFromDrive = scope === "everywhere";

  const confirmLabel = !alsoFromDrive
    ? t`Delete from this PC`
    : onPc
      ? t`Delete from PC and Drive`
      : t`Delete from Drive`;

  const confirm = () =>
    remove.mutate(
      { snapshotId: snapshot.id, alsoFromDrive },
      {
        onSuccess: () => {
          notify({
            tone: "neutral",
            title: t`Snapshot deleted`,
            body:
              alsoFromDrive && onPc
                ? t`“${name}” was removed from this PC and from Google Drive.`
                : alsoFromDrive
                  ? t`“${name}” was removed from Google Drive.`
                  : t`“${name}” was removed from this PC.`,
          });
          setChoice(null);
          onClose();
        },
        onError: (error) =>
          notify({ tone: "danger", title: t`The snapshot was not deleted`, body: labels.error(error) }),
      },
    );

  const close = () => {
    setChoice(null);
    onClose();
  };

  return (
    <Dialog
      open
      onOpenChange={(open) => !open && close()}
      tone="danger"
      width={520}
      icon={<DsIcon name="trash" size={22} />}
      title={t`Delete “${name}”?`}
      description={`${snapshot.gameTitle} · ${formatDateTime(snapshot.createdAt, i18n.locale)}`}
      hint={
        <>
          <Kbd>Esc</Kbd> {t`cancel`}
        </>
      }
      actions={
        <>
          <Button variant="ghost" onClick={close}>
            {t`Keep it`}
          </Button>
          <Button variant="danger" size="lg" icon="trash" onClick={confirm} loading={remove.isPending}>
            {confirmLabel}
          </Button>
        </>
      }
    >
      {inDrive && onPc ? (
        <div className="sd-opts" role="radiogroup">
          <OptionCard
            selected={scope === "pc"}
            onClick={() => setChoice("pc")}
            icon="drive"
            tone="local"
            title={t`Delete from this PC`}
            description={t`The copy in Google Drive stays, so you can still restore it later.`}
          />
          <OptionCard
            selected={scope === "everywhere"}
            onClick={() => setChoice("everywhere")}
            icon="cloud"
            tone="cloud"
            title={t`Delete from PC and Drive`}
            description={t`Removes both copies. Nothing is kept anywhere.`}
          />
        </div>
      ) : (
        <p className="m-0 text-sm leading-[21px] text-ink-muted">
          {inDrive
            ? t`This removes the snapshot from Google Drive. It is not on this PC.`
            : t`This removes the snapshot archive from this PC.`}
        </p>
      )}
      <p className="m-0 text-[13px] leading-[19px] text-ink-muted">
        {t`Your current save is not affected. This can’t be undone.`}
      </p>
      {snapshot.pinned ? (
        <Banner tone="warning" title={t`This snapshot is marked “never delete automatically”.`} />
      ) : null}
    </Dialog>
  );
}
