import { useLingui } from "@lingui/react/macro";
import { useState } from "react";

import { DsIcon } from "@/components/ds/ds-icon";
import { Checkbox, OptionCard, TextField } from "@/components/ds/form-fields";
import { notify } from "@/components/feedback/toast-store";
import { Button } from "@/components/ui/button";
import { Dialog, Kbd } from "@/components/ui/dialog";
import { useAddGameStore } from "@/features/library/add-game-store";
import { useAddExecutableFlow } from "@/features/library/use-library-flows";
import { useImportStore } from "@/features/saves-import/import-store";
import { useAddGames, useScanGameFolder } from "@/features/library/use-library";
import { useFolderPicker } from "@/features/saves/use-folder-picker";
import type { FolderScanReport } from "@/lib/bindings/FolderScanReport";
import type { GameFolderCandidate } from "@/lib/bindings/GameFolderCandidate";
import { cn } from "@/lib/cn";
import { formatBytes } from "@/lib/format";

/** The user's choice for one scanned game. */
interface Choice {
  selected: boolean;
  title: string;
  executable: string;
}

function initialChoices(report: FolderScanReport): Record<string, Choice> {
  return Object.fromEntries(
    report.candidates.map((candidate) => [
      candidate.folder,
      {
        selected: true,
        title: candidate.suggestedTitle,
        executable: (candidate.executables.find((exe) => exe.suggested) ?? candidate.executables[0])?.path ?? "",
      },
    ]),
  );
}

function CandidateRow({
  candidate,
  choice,
  onChange,
}: {
  candidate: GameFolderCandidate;
  choice: Choice;
  onChange: (next: Choice) => void;
}) {
  const { t, i18n } = useLingui();
  return (
    <li className={cn("rounded-md bg-surface-100 p-3", !choice.selected && "opacity-55")}>
      <div className="grid min-w-0 gap-2.5">
        <Checkbox
          checked={choice.selected}
          onChange={(selected) => onChange({ ...choice, selected })}
          label={candidate.suggestedTitle}
        />
        <TextField
          label={t`Title`}
          value={choice.title}
          maxLength={120}
          onChange={(event) => onChange({ ...choice, title: event.target.value })}
        />
        <div className="l8-field">
          <label className="l8-field-label" htmlFor={`program-${candidate.folder}`}>
            {t`Program to launch`}
          </label>
          <div className="l8-field-box">
            <select
              id={`program-${candidate.folder}`}
              value={choice.executable}
              onChange={(event) => onChange({ ...choice, executable: event.target.value })}
              className="l8-input"
            >
              {candidate.executables.map((executable) => (
                <option key={executable.path} value={executable.path}>
                  {executable.relativePath} · {formatBytes(executable.sizeBytes, i18n.locale)}
                  {executable.suggested ? ` · ${t`suggested`}` : ""}
                </option>
              ))}
            </select>
          </div>
        </div>
        <span className="truncate font-mono text-[11.5px] text-ink-faint" title={candidate.folder}>
          {candidate.folder}
        </span>
      </div>
    </li>
  );
}

/** Review of a folder scan: pick which games to add and which program launches each one. */
function ScanResultsDialog({ folder, report }: { folder: string; report: FolderScanReport }) {
  const { t } = useLingui();
  const close = useAddGameStore((state) => state.close);
  const addGames = useAddGames();
  const [choices, setChoices] = useState(() => initialChoices(report));

  const chosen = report.candidates.filter((candidate) => {
    const choice = choices[candidate.folder];
    return choice?.selected && choice.executable !== "" && choice.title.trim() !== "";
  });

  const submit = () =>
    addGames.mutate(
      chosen.map((candidate) => ({
        folder: candidate.folder,
        executable: choices[candidate.folder]?.executable ?? "",
        title: (choices[candidate.folder]?.title ?? candidate.suggestedTitle).trim(),
      })),
      {
        onSuccess: (result) => {
          notify({
            tone: result.failed > 0 ? "warning" : "success",
            title: result.added.length === 1 ? t`1 game added` : t`${result.added.length} games added`,
            body:
              result.failed > 0 || result.skipped > 0
                ? t`${result.skipped} already in your library · ${result.failed} could not be added`
                : undefined,
          });
          close();
        },
        onError: () =>
          notify({ tone: "danger", title: t`The games were not added`, body: t`Check the log file for details.` }),
      },
    );

  return (
    <Dialog
      open
      onOpenChange={(open) => !open && close()}
      width={720}
      icon={<DsIcon name="folderOpen" size={22} />}
      title={t`Games found`}
      description={folder}
      hint={
        <>
          <Kbd>Esc</Kbd> {t`cancel`}
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
            icon="plus"
            loading={addGames.isPending}
            disabled={chosen.length === 0 || addGames.isPending}
            onClick={submit}
          >
            {chosen.length === 1 ? t`Add 1 game` : t`Add ${chosen.length} games`}
          </Button>
        </>
      }
    >
      {report.candidates.length === 0 ? (
        <p className="m-0 rounded-md bg-surface-100 p-4 text-[13.5px] leading-5 text-ink-muted">
          {report.alreadyAdded > 0
            ? t`Every game found in this folder is already in your library.`
            : t`No game found. Choose the folder that contains your games, one folder per game.`}
        </p>
      ) : (
        <>
          <p className="m-0 text-[13px] leading-[19px] text-ink-muted">
            {t`Untick the ones you don’t want, and choose which program starts each game. Games already in your library are not listed.`}
          </p>
          <ul className="m-0 flex max-h-[380px] list-none flex-col gap-2 overflow-y-auto p-0">
            {report.candidates.map((candidate) => (
              <CandidateRow
                key={candidate.folder}
                candidate={candidate}
                choice={
                  choices[candidate.folder] ?? {
                    selected: false,
                    title: candidate.suggestedTitle,
                    executable: candidate.executables[0]?.path ?? "",
                  }
                }
                onChange={(next) => setChoices((current) => ({ ...current, [candidate.folder]: next }))}
              />
            ))}
          </ul>
          {report.alreadyAdded > 0 && (
            <p className="m-0 text-[12.5px] text-ink-faint">
              {report.alreadyAdded === 1
                ? t`1 game was hidden because it is already in your library.`
                : t`${report.alreadyAdded} games were hidden because they are already in your library.`}
            </p>
          )}
        </>
      )}
    </Dialog>
  );
}

/** "Add a game": one executable, or a folder scan. Rendered once, in the app shell. */
export function AddGameDialogs() {
  const { t } = useLingui();
  const stage = useAddGameStore((state) => state.stage);
  const { close, showScan } = useAddGameStore.getState();
  const addExecutable = useAddExecutableFlow();
  const pickFolder = useFolderPicker();
  const scan = useScanGameFolder();

  const scanFolder = async () => {
    const folder = await pickFolder(t`Choose the folder that contains your games`);
    if (!folder) return;
    scan.mutate(folder, {
      onSuccess: (report) => showScan(folder, report),
      onError: () => notify({ tone: "danger", title: t`The folder could not be scanned` }),
    });
  };

  if (stage.kind === "scanned") return <ScanResultsDialog folder={stage.folder} report={stage.report} />;

  return (
    <Dialog
      open={stage.kind === "choose"}
      onOpenChange={(open) => !open && close()}
      width={560}
      icon={<DsIcon name="plus" size={22} />}
      title={t`Add a game`}
      description={t`Steam, Epic, Xbox and GOG games are found automatically. Use this for any other game.`}
      hint={
        <>
          <Kbd>Esc</Kbd> {t`cancel`}
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
          disabled={scan.isPending}
          icon="folderOpen"
          tone="flare"
          title={t`Scan a folder`}
          description={t`Pick the folder that holds your games. L8Play lists each game’s programs so you choose which one to launch.`}
          onClick={() => void scanFolder()}
        />
        <OptionCard
          selected={false}
          icon="gamepad"
          tone="local"
          title={t`Choose one program`}
          description={t`Select a game’s .exe file directly.`}
          onClick={() => {
            close();
            void addExecutable.start();
          }}
        />
        <OptionCard
          selected={false}
          icon="snapshot"
          tone="cloud"
          title={t`Import saves only`}
          description={t`For games you no longer have installed: bring in the backups you made by hand and keep them with the others.`}
          onClick={() => {
            close();
            useImportStore.getState().open();
          }}
        />
        {scan.isPending && <div className="l8-skel h-2 rounded-pill" />}
      </div>
    </Dialog>
  );
}
