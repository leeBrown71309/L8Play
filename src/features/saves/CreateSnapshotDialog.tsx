import { useLingui } from "@lingui/react/macro";
import { useMemo, useState } from "react";
import { Link } from "react-router";

import { ROUTES } from "@/app/routes";
import { DsIcon } from "@/components/ds/ds-icon";
import { Checkbox, OptionCard, TextAreaField, TextField } from "@/components/ds/form-fields";
import { StatusBadge } from "@/components/ds/status-badge";
import { notify } from "@/components/feedback/toast-store";
import { GameArt } from "@/components/game/GameArt";
import { Button } from "@/components/ui/button";
import { Dialog, Kbd } from "@/components/ui/dialog";
import { useCloudStatus } from "@/features/cloud/use-cloud";
import { useSnapshotLabels } from "@/features/saves/snapshot-labels";
import { useCreateSnapshot, useCurrentSave, useSnapshotFolder } from "@/features/saves/use-saves";
import "@/features/saves/snapshot-dialogs.css";
import { useSettings } from "@/features/settings/use-settings";
import type { GameSummary } from "@/lib/bindings/GameSummary";
import type { SnapshotStorage } from "@/lib/bindings/SnapshotStorage";
import { formatBytes, formatRelative } from "@/lib/format";

const MAX_NAME = 80;

interface CreateSnapshotDialogProps {
  game: GameSummary;
  running: boolean;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

/** A name suggested from the moment of the backup, e.g. "Sekiro · 4 Nov, 21:12". */
function suggestName(title: string, locale: string): string {
  const when = new Intl.DateTimeFormat(locale, {
    day: "numeric",
    month: "short",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date());
  return `${title} · ${when}`.slice(0, MAX_NAME);
}

/** Source save, name (suggested), optional note, where it is kept, and pinning; then a verified snapshot. */
export function CreateSnapshotDialog({ game, running, open, onOpenChange }: CreateSnapshotDialogProps) {
  const { t, i18n } = useLingui();
  const labels = useSnapshotLabels();
  const current = useCurrentSave(game.id);
  const folder = useSnapshotFolder();
  const settings = useSettings();
  const cloud = useCloudStatus().data;
  const create = useCreateSnapshot();
  const [typedName, setTypedName] = useState<string | null>(null);
  const [note, setNote] = useState("");
  const [pinned, setPinned] = useState(false);
  const [storage, setStorage] = useState<SnapshotStorage>("both");

  // eslint-disable-next-line react-hooks/exhaustive-deps -- a fresh suggestion each time the dialog opens
  const suggestion = useMemo(() => suggestName(game.title, i18n.locale), [game.title, i18n.locale, open]);
  const name = typedName ?? suggestion;
  const trimmed = name.trim();
  const driveOn = cloud?.account?.status === "connected" && settings.cloudAutoSync;
  const size = current.data ? formatBytes(current.data.totalBytes, i18n.locale) : null;

  const close = () => {
    onOpenChange(false);
    setTypedName(null);
    setNote("");
    setPinned(false);
    setStorage("both");
  };

  const submit = () => {
    if (trimmed === "" || running || create.isPending) return;
    create.mutate(
      {
        gameId: game.id,
        name: trimmed,
        description: note.trim() === "" ? null : note.trim(),
        pinned,
        // Without a usable Drive connection the only possible home is this PC.
        storage: driveOn ? storage : "local",
      },
      {
        onSuccess: (result) => {
          if (result.status === "created") {
            const where = !driveOn
              ? t`on this PC`
              : storage === "cloud"
                ? t`queued for Google Drive only`
                : storage === "local"
                  ? t`on this PC only`
                  : t`on this PC and queued for Google Drive`;
            notify({
              tone: "success",
              title: t`Snapshot created`,
              body: t`${game.title} · “${trimmed}” · ${formatBytes(result.snapshot.totalBytes, i18n.locale)} · ${where}`,
            });
            close();
          } else {
            notify({
              tone: "warning",
              title: t`Nothing to back up yet`,
              body: t`The save folder of ${game.title} is empty.`,
            });
          }
        },
        onError: (error) =>
          notify({ tone: "danger", title: t`The snapshot was not created`, body: labels.error(error) }),
      },
    );
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => (next ? onOpenChange(true) : close())}
      icon={<DsIcon name="snapshot" size={22} />}
      title={t`Create snapshot`}
      description={t`A full copy of your current save, kept until you delete it or retention removes it.`}
      width={560}
      hint={
        <>
          <Kbd>Enter</Kbd> {t`create`} <Kbd>Esc</Kbd> {t`cancel`}
        </>
      }
      actions={
        <>
          <Button variant="ghost" onClick={close}>
            {t`Cancel`}
          </Button>
          <Button
            variant="primary"
            size="lg"
            icon="snapshot"
            onClick={submit}
            loading={create.isPending}
            disabled={trimmed === "" || running}
          >
            {create.isPending ? t`Creating…` : t`Create snapshot`}
          </Button>
        </>
      }
    >
      <form
        className="sd-form"
        onSubmit={(event) => {
          event.preventDefault();
          submit();
        }}
      >
        <div className="sd-src">
          <div className="ic">
            <GameArt game={game} kind="icon" showTitle={false} />
          </div>
          <div>
            <b>{game.title}</b>
            <span className="m">
              {current.data
                ? `${labels.contents(current.data.totalFiles, formatBytes(current.data.totalBytes, i18n.locale))}${
                    current.data.lastModifiedAt
                      ? ` · ${t`modified ${formatRelative(current.data.lastModifiedAt, i18n.locale)}`}`
                      : ""
                  }`
                : t`Reading the current save…`}
            </span>
          </div>
          {running ? (
            <StatusBadge status="unprotected">{t`Close the game first`}</StatusBadge>
          ) : (
            <StatusBadge status="verified">{t`Game is closed`}</StatusBadge>
          )}
        </div>

        <TextField
          label={t`Name`}
          value={name}
          maxLength={MAX_NAME}
          autoFocus
          onChange={(event) => setTypedName(event.target.value)}
          hint={
            typedName === null
              ? t`Suggested from the date. ${name.length} / ${MAX_NAME}`
              : t`${name.length} / ${MAX_NAME}`
          }
        />
        <TextAreaField
          label={t`Note`}
          optional={t`Optional`}
          rows={1}
          value={note}
          onChange={(event) => setNote(event.target.value)}
          placeholder={t`What makes this save worth keeping? Build, route, items…`}
        />

        <div>
          <div className="sd-lbl">{t`Keep it`}</div>
          <div className="sd-opts" role="radiogroup">
            {driveOn ? (
              <>
                <OptionCard
                  selected={storage === "both"}
                  onClick={() => setStorage("both")}
                  icon="cloudCheck"
                  tone="cloud"
                  title={t`On this PC and in Google Drive`}
                  badge={t`Recommended`}
                  description={t`Survives a broken drive or a new PC.`}
                  {...(size ? { meta: `+${size}` } : {})}
                />
                <OptionCard
                  selected={storage === "local"}
                  onClick={() => setStorage("local")}
                  icon="drive"
                  tone="local"
                  title={t`On this PC only`}
                  description={folder.data ?? t`In your snapshot folder`}
                />
                <OptionCard
                  selected={storage === "cloud"}
                  onClick={() => setStorage("cloud")}
                  icon="cloud"
                  tone="cloud"
                  title={t`In Google Drive only`}
                  description={t`Frees this PC once the upload is confirmed.`}
                />
              </>
            ) : (
              <>
                <OptionCard
                  selected
                  onClick={() => undefined}
                  icon="drive"
                  tone="local"
                  title={t`On this PC`}
                  description={
                    cloud?.account?.status === "connected"
                      ? t`Google Drive uploads are switched off in Settings.`
                      : (folder.data ?? t`In your snapshot folder`)
                  }
                  {...(size ? { meta: `+${size}` } : {})}
                />
                {cloud?.account?.status !== "connected" ? (
                  <p className="sd-hint">
                    {t`Connect Google Drive to also keep snapshots there, or only there.`}{" "}
                    <Link to={ROUTES.cloud} onClick={close}>
                      {t`Open Cloud Storage`}
                    </Link>
                  </p>
                ) : null}
              </>
            )}
          </div>
        </div>

        <Checkbox
          checked={pinned}
          onChange={setPinned}
          label={t`Never delete automatically`}
          description={t`Retention keeps the last ${settings.keepAutomatic} automatic snapshots per game. Pinned ones are always kept.`}
        />
      </form>
    </Dialog>
  );
}
