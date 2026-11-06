import { useLingui } from "@lingui/react/macro";
import { useState } from "react";

import { DsIcon } from "@/components/ds/ds-icon";
import { Checkbox, OptionCard, TextField } from "@/components/ds/form-fields";
import { ProgressBar } from "@/components/ds/progress-bar";
import { notify } from "@/components/feedback/toast-store";
import { Button } from "@/components/ui/button";
import { Dialog, Kbd } from "@/components/ui/dialog";
import { useImportStore } from "@/features/saves-import/import-store";
import {
  buildRows,
  isImportable,
  setLabel,
  toRequests,
  totalSnapshots,
  withReplannedGame,
  type ReviewRow,
} from "@/features/saves-import/review-state";
import {
  useImportBackupGames,
  useImportProgress,
  useReplanBackupGame,
  useScanBackupFolder,
} from "@/features/saves-import/use-import";
import { useFolderPicker } from "@/features/saves/use-folder-picker";
import type { ImportPlan } from "@/lib/bindings/ImportPlan";
import type { ImportReport } from "@/lib/bindings/ImportReport";
import type { ScanMode } from "@/lib/bindings/ScanMode";
import { cn } from "@/lib/cn";
import { formatBytes, formatDateTime } from "@/lib/format";

const CANCEL_HINT = <Kbd>Esc</Kbd>;

/** First step: which kind of folder the backups are in. */
function ChooseStage() {
  const { t } = useLingui();
  const { close, analyzing, review } = useImportStore.getState();
  const pickFolder = useFolderPicker();
  const scan = useScanBackupFolder();

  const start = async (mode: ScanMode | null, title: string) => {
    const folder = await pickFolder(title);
    if (!folder) return;
    analyzing(folder);
    // `mutateAsync`, not `mutate` callbacks: this dialog unmounts as soon as the next step is shown.
    try {
      const plan = await scan.mutateAsync({ folder, mode });
      // Cancelled meanwhile: the result is dropped.
      if (useImportStore.getState().stage.kind === "analyzing") review(plan);
    } catch {
      if (useImportStore.getState().stage.kind !== "analyzing") return;
      notify({ tone: "danger", title: t`The folder could not be read` });
      useImportStore.getState().open();
    }
  };

  return (
    <Dialog
      open
      onOpenChange={(open) => !open && close()}
      width={560}
      icon={<DsIcon name="snapshot" size={22} />}
      title={t`Import existing saves`}
      description={t`Bring in backups you made by hand, even for games that are not installed. Your original files are never changed.`}
      hint={
        <>
          {CANCEL_HINT} {t`cancel`}
        </>
      }
      actions={
        <Button variant="ghost" onClick={close}>
          {t`Cancel`}
        </Button>
      }
    >
      <div className="grid gap-2" role="group">
        <OptionCard
          selected={false}
          icon="folderOpen"
          tone="flare"
          title={t`A folder with all my saves`}
          description={t`One sub-folder per game, like Save Games/Elden Ring. L8Play finds each game for you.`}
          onClick={() => void start(null, t`Choose the folder that contains your saves`)}
        />
        <OptionCard
          selected={false}
          icon="gamepad"
          tone="local"
          title={t`The saves of one game`}
          description={t`Choose the backup folder of a single game. Sub-folders become separate named snapshots.`}
          onClick={() => void start("one_game", t`Choose the backup folder of the game`)}
        />
      </div>
    </Dialog>
  );
}

function AnalyzingStage({ folder }: { folder: string }) {
  const { t } = useLingui();
  const close = useImportStore((state) => state.close);
  const minimize = useImportStore((state) => state.minimize);
  return (
    <Dialog
      open
      // Closing the window only hides it: reading the folder goes on, and an indicator shows it.
      onOpenChange={(open) => !open && minimize()}
      width={560}
      icon={<DsIcon name="scan" size={22} />}
      title={t`Reading your saves…`}
      description={folder}
      actions={
        <>
          <Button variant="ghost" onClick={close}>
            {t`Cancel`}
          </Button>
          <Button variant="secondary" onClick={minimize}>
            {t`Keep going in the background`}
          </Button>
        </>
      }
    >
      <ProgressBar indeterminate label={t`Looking for games and snapshots`} />
      <p className="m-0 text-[12.5px] text-ink-faint">
        {t`A big folder can take a few minutes. You can hide this window: the work continues and you will be told when it is ready.`}
      </p>
    </Dialog>
  );
}

function SnapshotLines({
  row,
  onChange,
}: {
  row: ReviewRow;
  onChange: (next: ReviewRow) => void;
}) {
  const { t, i18n } = useLingui();
  return (
    <ul className="m-0 grid list-none gap-1.5 p-0">
      {row.game.snapshots.map((snapshot, index) => (
        <li key={snapshot.root} className="flex min-w-0 items-center gap-2.5">
          <div className="min-w-0 flex-1">
            <TextField
              label={t`Snapshot name`}
              value={row.labels[index] ?? ""}
              maxLength={120}
              placeholder={t`Imported backup`}
              onChange={(event) => onChange(setLabel(row, index, event.target.value))}
            />
          </div>
          <span className="w-[230px] shrink-0 text-right text-[12px] text-ink-faint">
            {snapshot.files === 1 ? t`1 file` : t`${snapshot.files} files`} · {formatBytes(snapshot.bytes, i18n.locale)}
            {snapshot.modifiedAt ? ` · ${formatDateTime(snapshot.modifiedAt, i18n.locale)}` : ""}
          </span>
        </li>
      ))}
    </ul>
  );
}

function GameRowEditor({
  row,
  busy,
  onChange,
  onToggleLayout,
}: {
  row: ReviewRow;
  busy: boolean;
  onChange: (next: ReviewRow) => void;
  onToggleLayout: () => void;
}) {
  const { t } = useLingui();
  const { game } = row;
  const split = game.layout === "variants";
  return (
    <li className={cn("rounded-md bg-surface-100 p-3", !row.include && "opacity-55")}>
      <div className="grid min-w-0 gap-2.5">
        <Checkbox
          checked={row.include}
          onChange={(include) => onChange({ ...row, include })}
          label={game.folderName}
        />
        <TextField
          label={t`Game title`}
          value={row.title}
          maxLength={120}
          onChange={(event) => onChange({ ...row, title: event.target.value })}
        />
        {game.suggestedTitle === null && game.altTitles.length > 0 && (
          <div className="flex flex-wrap items-center gap-1.5 text-[12px] text-ink-muted">
            <span>{t`Other names found:`}</span>
            {game.altTitles.map((alt) => (
              <button
                key={alt}
                type="button"
                className="l8-chip"
                onClick={() => onChange({ ...row, title: alt })}
              >
                {alt}
              </button>
            ))}
          </div>
        )}
        <SnapshotLines row={row} onChange={onChange} />
        <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-[12px] text-ink-faint">
          {game.duplicateOf !== null && (
            <span className="text-warning">
              {t`Same game as another folder: their snapshots will be joined.`}
            </span>
          )}
          {game.skippedFiles > 0 && (
            <span>
              {game.skippedFiles === 1
                ? t`1 log or screenshot left out`
                : t`${game.skippedFiles} logs and screenshots left out`}
            </span>
          )}
          {game.emptySnapshots.length > 0 && (
            <span>{t`Empty folders skipped: ${game.emptySnapshots.join(", ")}`}</span>
          )}
          {(split || game.snapshots.length === 1) && (
            <Button variant="ghost" size="sm" disabled={busy} onClick={onToggleLayout}>
              {split ? t`Keep as one snapshot` : t`Split by sub-folder`}
            </Button>
          )}
        </div>
        <span className="truncate font-mono text-[11.5px] text-ink-faint" title={game.folder}>
          {game.folder}
        </span>
      </div>
    </li>
  );
}

function ReviewStage({ plan }: { plan: ImportPlan }) {
  const { t } = useLingui();
  const { close, importing, done } = useImportStore.getState();
  const [rows, setRows] = useState<ReviewRow[]>(() => buildRows(plan));
  const replan = useReplanBackupGame();
  const importGames = useImportBackupGames();

  const chosen = rows.filter(isImportable).length;
  const snapshotCount = totalSnapshots(rows);

  const update = (key: string, next: ReviewRow) =>
    setRows((current) => current.map((row) => (row.game.key === key ? next : row)));

  const toggleLayout = (row: ReviewRow) =>
    replan.mutate(
      {
        folder: row.game.folder,
        key: row.game.key,
        fromCollection: row.game.fromCollection,
        layout: row.game.layout === "variants" ? "single" : "variants",
      },
      {
        onSuccess: (game) => {
          if (game) update(row.game.key, withReplannedGame(row, game));
        },
        onError: () => notify({ tone: "danger", title: t`The folder could not be read again` }),
      },
    );

  const submit = async () => {
    importing();
    // `mutateAsync`, not `mutate` callbacks: this dialog unmounts as soon as the import starts.
    try {
      done(await importGames.mutateAsync(toRequests(rows)));
    } catch {
      notify({ tone: "danger", title: t`The saves were not imported`, body: t`Check the log file for details.` });
      close();
    }
  };

  return (
    <Dialog
      open
      onOpenChange={(open) => !open && close()}
      dismissOnOutsideClick={false}
      width={760}
      icon={<DsIcon name="snapshot" size={22} />}
      title={t`Review before importing`}
      description={plan.source}
      hint={
        <>
          {CANCEL_HINT} {t`cancel`}
        </>
      }
      actions={
        <>
          <Button variant="ghost" onClick={close}>
            {t`Cancel`}
          </Button>
          <Button variant="primary" size="lg" icon="plus" disabled={chosen === 0} onClick={() => void submit()}>
            {chosen === 1 ? t`Import 1 game` : t`Import ${chosen} games`}
          </Button>
        </>
      }
    >
      {rows.length === 0 ? (
        <p className="m-0 rounded-md bg-surface-100 p-4 text-[13.5px] leading-5 text-ink-muted">
          {t`No saves found in this folder. Choose the folder that holds your backups, one sub-folder per game.`}
        </p>
      ) : (
        <>
          <p className="m-0 text-[13px] leading-[19px] text-ink-muted">
            {t`Check each game’s title and the name of its snapshots. Games you import appear in your library as “Saves only” until you install them.`}
          </p>
          <ul className="m-0 flex max-h-[420px] list-none flex-col gap-2 overflow-y-auto p-0">
            {rows.map((row) => (
              <GameRowEditor
                key={row.game.key}
                row={row}
                busy={replan.isPending}
                onChange={(next) => update(row.game.key, next)}
                onToggleLayout={() => toggleLayout(row)}
              />
            ))}
          </ul>
          <p className="m-0 text-[12.5px] text-ink-faint">
            {snapshotCount === 1 ? t`1 snapshot will be created.` : t`${snapshotCount} snapshots will be created.`}
          </p>
        </>
      )}
      {plan.warnings.map((warning) => (
        <p key={warning} className="m-0 text-[12.5px] text-ink-faint">
          {warning}
        </p>
      ))}
    </Dialog>
  );
}

function ImportingStage() {
  const { t } = useLingui();
  const minimize = useImportStore((state) => state.minimize);
  const progress = useImportStore((state) => (state.stage.kind === "importing" ? state.stage.progress : null));
  const value = progress && progress.total > 0 ? (progress.done / progress.total) * 100 : 0;
  return (
    <Dialog
      open
      // The import cannot be stopped half way: closing the window only hides it.
      onOpenChange={(open) => !open && minimize()}
      width={560}
      icon={<DsIcon name="snapshot" size={22} />}
      title={t`Importing your saves…`}
      description={t`Each snapshot is copied and checked. You can keep using L8Play meanwhile.`}
      actions={
        <Button variant="secondary" onClick={minimize}>
          {t`Keep going in the background`}
        </Button>
      }
    >
      <ProgressBar
        value={value}
        indeterminate={progress === null}
        label={progress?.currentTitle || t`Preparing`}
        meta={progress ? `${progress.done} / ${progress.total}` : undefined}
      />
    </Dialog>
  );
}

function summarize(report: ImportReport) {
  const imported = report.games.reduce((sum, game) => sum + game.imported, 0);
  const duplicates = report.games.reduce((sum, game) => sum + game.duplicates, 0);
  const failed = report.games.filter((game) => game.errors.length > 0);
  return { imported, duplicates, failed };
}

function DoneStage({ report }: { report: ImportReport }) {
  const { t } = useLingui();
  const close = useImportStore((state) => state.close);
  const { imported, duplicates, failed } = summarize(report);
  return (
    <Dialog
      open
      onOpenChange={(open) => !open && close()}
      width={560}
      tone={failed.length > 0 ? "warning" : "success"}
      icon={<DsIcon name={failed.length > 0 ? "alert" : "check"} size={22} />}
      title={imported === 1 ? t`1 snapshot imported` : t`${imported} snapshots imported`}
      description={
        duplicates > 0 ? t`${duplicates} were already imported and were skipped.` : t`They are now in your library.`
      }
      actions={
        <Button variant="primary" size="lg" onClick={close}>
          {t`Done`}
        </Button>
      }
    >
      {failed.length > 0 && (
        <ul className="m-0 grid max-h-[260px] list-none gap-2 overflow-y-auto p-0">
          {failed.map((game) => (
            <li key={game.title} className="rounded-md bg-surface-100 p-3 text-[13px] text-ink-muted">
              <strong className="text-ink">{game.title}</strong>
              {game.errors.map((error) => (
                <div key={error} className="mt-1 text-[12px] text-danger">
                  {error}
                </div>
              ))}
            </li>
          ))}
        </ul>
      )}
    </Dialog>
  );
}

/** "Import existing saves" flow. Rendered once, in the app shell. */
export function ImportSavesDialogs() {
  useImportProgress();
  const stage = useImportStore((state) => state.stage);
  const minimized = useImportStore((state) => state.minimized);
  if (minimized) return null;
  switch (stage.kind) {
    case "closed":
      return null;
    case "choose":
      return <ChooseStage />;
    case "analyzing":
      return <AnalyzingStage folder={stage.folder} />;
    case "review":
      return <ReviewStage plan={stage.plan} />;
    case "importing":
      return <ImportingStage />;
    case "done":
      return <DoneStage report={stage.report} />;
  }
}
