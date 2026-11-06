import { useLingui } from "@lingui/react/macro";
import { useState } from "react";

import { DsIcon } from "@/components/ds/ds-icon";
import { Banner } from "@/components/ds/page-blocks";
import { OptionCard } from "@/components/ds/form-fields";
import { notify } from "@/components/feedback/toast-store";
import { Button } from "@/components/ui/button";
import { Dialog, Kbd } from "@/components/ui/dialog";
import { useSnapshotLabels } from "@/features/saves/snapshot-labels";
import { useRestoreSnapshotTo, useRestoreTargets } from "@/features/saves/use-saves";
import { useFolderPicker } from "@/features/saves/use-folder-picker";
import type { SnapshotSummary } from "@/lib/bindings/SnapshotSummary";
import { formatBytes, formatDateTime } from "@/lib/format";

interface RestoreImportedDialogProps {
  snapshot: SnapshotSummary;
  running: boolean;
  onClose: () => void;
}

/**
 * Restore of a snapshot imported from an old backup: the folder it goes to is chosen here (suggested
 * from the backup's own note and from the game's save folders). Files are laid over that folder; nothing
 * in it is deleted, and whatever would be replaced is kept in a safety copy first.
 */
export function RestoreImportedDialog({ snapshot, running, onClose }: RestoreImportedDialogProps) {
  const { t, i18n } = useLingui();
  const labels = useSnapshotLabels();
  const targets = useRestoreTargets(snapshot.id);
  const restore = useRestoreSnapshotTo();
  const pickFolder = useFolderPicker();
  const [picked, setPicked] = useState<string | null>(null);
  const name = labels.name(snapshot);
  const suggestions = targets.data ?? [];
  const target = picked ?? suggestions[0]?.path ?? null;

  const browse = async () => {
    const folder = await pickFolder(t`Choose the folder to restore into`);
    if (folder) setPicked(folder);
  };

  const confirm = () => {
    if (target === null) return;
    restore.mutate(
      { snapshotId: snapshot.id, target },
      {
        onSuccess: (report) => {
          notify({
            tone: "success",
            title: t`Snapshot restored`,
            body: report.safetySnapshotId
              ? t`“${name}” is in place. The files it replaced were kept as a safety copy.`
              : t`“${name}” is in place.`,
          });
          onClose();
        },
        onError: (error) => notify({ tone: "danger", title: t`Nothing was restored`, body: labels.error(error) }),
      },
    );
  };

  return (
    <Dialog
      open
      onOpenChange={(open) => !open && onClose()}
      tone="warning"
      width={620}
      icon={<DsIcon name="restore" size={22} />}
      title={t`Restore “${name}”?`}
      description={snapshot.gameTitle}
      hint={
        <>
          <Kbd>Esc</Kbd> {t`cancel`}
        </>
      }
      actions={
        <>
          <Button variant="ghost" onClick={onClose}>
            {t`Cancel`}
          </Button>
          <Button
            variant="primary"
            size="lg"
            icon="restore"
            onClick={confirm}
            loading={restore.isPending}
            disabled={running || target === null}
          >
            {restore.isPending ? t`Restoring…` : t`Restore here`}
          </Button>
        </>
      }
    >
      <p className="m-0 text-[13px] leading-[19px] text-ink-muted">
        {t`This snapshot came from an old backup, so L8Play does not know where the game keeps its saves. ${snapshot.totalFiles} files · ${formatBytes(snapshot.totalBytes, i18n.locale)} · ${formatDateTime(snapshot.createdAt, i18n.locale)}`}
      </p>

      <div className="grid gap-2" role="radiogroup" aria-label={t`Restore into`}>
        {suggestions.map((option) => (
          <OptionCard
            key={option.path}
            selected={target === option.path}
            icon="folderOpen"
            title={option.path}
            description={
              option.source === "hint"
                ? t`Written in the note that came with the backup`
                : option.exists
                  ? t`A save folder of this game on this PC`
                  : t`A save folder of this game (not created yet)`
            }
            onClick={() => setPicked(option.path)}
          />
        ))}
        {picked !== null && !suggestions.some((entry) => entry.path === picked) ? (
          <OptionCard selected icon="folder" title={picked} description={t`The folder you chose`} onClick={() => undefined} />
        ) : null}
        {targets.isPending ? <div className="l8-skel h-2 rounded-pill" /> : null}
        <Button variant="secondary" icon="folder" onClick={() => void browse()}>
          {t`Choose another folder…`}
        </Button>
      </div>

      <Banner tone="success" icon="shieldCheck" title={t`Nothing in that folder is deleted.`}>
        {t`The snapshot’s files are copied over it. Any file they replace is kept first in a safety copy you can restore from Saves.`}
      </Banner>

      {running ? (
        <p className="m-0 text-[12.5px] text-warning">{t`Close ${snapshot.gameTitle} before restoring.`}</p>
      ) : null}
    </Dialog>
  );
}
