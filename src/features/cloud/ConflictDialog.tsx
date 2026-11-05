import { useLingui } from "@lingui/react/macro";
import { useState } from "react";

import { DsIcon } from "@/components/ds/ds-icon";
import { OptionCard } from "@/components/ds/form-fields";
import { StatusBadge } from "@/components/ds/status-badge";
import { notify } from "@/components/feedback/toast-store";
import { Button } from "@/components/ui/button";
import { Dialog } from "@/components/ui/dialog";
import { useCloudErrorText, useResolveConflict } from "@/features/cloud/use-cloud";
import type { ConflictInfo } from "@/lib/bindings/ConflictInfo";
import type { ConflictResolution } from "@/lib/bindings/ConflictResolution";
import type { ConflictSide } from "@/lib/bindings/ConflictSide";
import { formatBytes, formatDateTime, formatPlaytime } from "@/lib/format";
import "@/features/cloud/conflict-dialog.css";

interface ConflictDialogProps {
  conflict: ConflictInfo | null;
  running: boolean;
  onClose: () => void;
}

type Age = "newer" | "older" | "same";

function ageOf(side: ConflictSide, other: ConflictSide): Age {
  const mine = new Date(side.createdAt).getTime();
  const theirs = new Date(other.createdAt).getTime();
  return mine === theirs ? "same" : mine > theirs ? "newer" : "older";
}

interface VersionCardProps {
  where: "pc" | "drive";
  side: ConflictSide;
  age: Age;
  picked: boolean;
}

/** One side of the conflict: where it lives, when it was saved and how big it is. */
function VersionCard({ where, side, age, picked }: VersionCardProps) {
  const { t, i18n } = useLingui();
  const files = side.totalFiles === 1 ? t`1 file` : t`${side.totalFiles} files`;
  return (
    <div className={`conflict-ver${picked ? " pick" : ""}`}>
      <div className="hd">
        <DsIcon
          name={where === "pc" ? "drive" : "cloud"}
          size={18}
          className={where === "pc" ? "ico-local" : "ico-cloud"}
        />
        <b>{where === "pc" ? t`On this PC` : t`In Google Drive`}</b>
        {age === "newer" && <StatusBadge status="synced" size="sm">{t`Newer`}</StatusBadge>}
        {age === "older" && <StatusBadge status="local" size="sm">{t`Older`}</StatusBadge>}
      </div>
      <div className="dev">{side.name ? `${side.deviceName} · ${side.name}` : side.deviceName}</div>
      <dl>
        <dt>{t`Saved`}</dt>
        <dd>{formatDateTime(side.createdAt, i18n.locale)}</dd>
        {side.playtimeSeconds !== null && (
          <>
            <dt>{t`Playtime`}</dt>
            <dd>{formatPlaytime(side.playtimeSeconds)}</dd>
          </>
        )}
        <dt>{t`Size`}</dt>
        <dd>{`${formatBytes(side.totalBytes, i18n.locale)} · ${files}`}</dd>
      </dl>
    </div>
  );
}

/** Two saves grew apart on two devices. Nothing is overwritten: the user picks which one wins. */
export function ConflictDialog({ conflict, running, onClose }: ConflictDialogProps) {
  const { t, i18n } = useLingui();
  const errorText = useCloudErrorText();
  const resolve = useResolveConflict();
  const [picked, setPicked] = useState<ConflictResolution | null>(null);
  if (!conflict) return null;

  const { local, remote } = conflict;
  const remoteAge = ageOf(remote, local);
  // Pre-select the side with more progress; nothing happens until the confirm button is pressed.
  const choice: ConflictResolution = picked ?? (remoteAge === "newer" ? "use_remote" : "use_local");
  const blocked = running && choice === "use_remote";

  const confirm = () =>
    resolve.mutate(
      { conflictId: conflict.id, resolution: choice },
      {
        onSuccess: () => {
          notify({ tone: "success", title: t`Conflict resolved`, body: conflict.gameTitle });
          setPicked(null);
          onClose();
        },
        onError: (error) => notify({ tone: "danger", title: t`The conflict was not resolved`, body: errorText(error) }),
      },
    );

  const confirmLabel =
    choice === "use_remote" ? t`Use Drive version` : choice === "use_local" ? t`Use PC version` : t`Keep both apart`;

  return (
    <Dialog
      open
      onOpenChange={(open) => !open && onClose()}
      tone="warning"
      width={820}
      icon={<DsIcon name="merge" size={22} />}
      title={t`Two different saves for ${conflict.gameTitle}`}
      description={t`Detected ${formatDateTime(conflict.detectedAt, i18n.locale)}. Nothing has been overwritten.`}
      hint={
        <>
          <DsIcon name="shield" size={16} />
          {t`The version you don’t use is kept as a snapshot.`}
        </>
      }
      actions={
        <>
          <Button variant="ghost" onClick={onClose}>
            {t`Decide later`}
          </Button>
          <Button variant="primary" size="lg" disabled={resolve.isPending || blocked} onClick={confirm}>
            {confirmLabel}
          </Button>
        </>
      }
    >
      <div className="conflict-cmp">
        <VersionCard where="pc" side={local} age={ageOf(local, remote)} picked={choice === "use_local"} />
        <VersionCard where="drive" side={remote} age={remoteAge} picked={choice === "use_remote"} />
      </div>
      {blocked && (
        <p className="m-0 flex items-center gap-2 text-[13px] font-semibold text-warning">
          <DsIcon name="alert" size={16} />
          {t`Close the game before using the Drive version.`}
        </p>
      )}
      <div className="conflict-opts" role="radiogroup">
        <OptionCard
          selected={choice === "use_remote"}
          onClick={() => setPicked("use_remote")}
          icon="cloudDown"
          tone="cloud"
          title={t`Use the Drive version`}
          {...(remoteAge === "newer" ? { badge: t`More progress` } : {})}
          description={t`Your PC save is backed up first, then replaced by the Drive version.`}
        />
        <OptionCard
          selected={choice === "use_local"}
          onClick={() => setPicked("use_local")}
          icon="drive"
          tone="local"
          title={t`Use the version on this PC`}
          description={t`The Drive save stays in your Drive as a conflict copy you can restore later.`}
        />
        <OptionCard
          selected={choice === "keep_both"}
          onClick={() => setPicked("keep_both")}
          icon="copy"
          tone="neutral"
          title={t`Keep both, change nothing yet`}
          description={t`Keeps this PC’s save and downloads the other one as a conflict copy you can restore later.`}
        />
      </div>
    </Dialog>
  );
}
