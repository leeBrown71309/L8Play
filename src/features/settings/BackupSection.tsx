import { useLingui } from "@lingui/react/macro";
import { useQueryClient } from "@tanstack/react-query";

import { Dropdown, TextField, type DropdownOption } from "@/components/ds/form-fields";
import { notify } from "@/components/feedback/toast-store";
import { Button } from "@/components/ui/button";
import { useFolderPicker } from "@/features/saves/use-folder-picker";
import { useRevealSnapshot, useSnapshotFolder, useSnapshots } from "@/features/saves/use-saves";
import { AlwaysOn, SettingGroup, SettingRow, ToggleSetting } from "@/features/settings/setting-blocks";
import { cloudRetentionChoices, pcRetentionChoices } from "@/features/settings/retention-options";
import { useSettings, useUpdateSettings } from "@/features/settings/use-settings";
import { formatBytes } from "@/lib/format";
import { queryKeys } from "@/lib/query-keys";

/** Automatic snapshot triggers, retention on this PC and in Drive, and where archives are written. */
export function BackupSection() {
  const { t, i18n } = useLingui();
  const settings = useSettings();
  const update = useUpdateSettings();
  const folder = useSnapshotFolder();
  const snapshots = useSnapshots(null);
  const reveal = useRevealSnapshot();
  const pickFolder = useFolderPicker();
  const client = useQueryClient();

  const lastN = (count: number): DropdownOption<number> => ({ id: count, label: t`Last ${count} per game` });
  const pcOptions = pcRetentionChoices(settings.keepAutomatic).map(lastN);
  const cloudOptions: DropdownOption<number>[] = cloudRetentionChoices(settings.keepAutomatic, settings.keepCloud).map(
    (choice) => (choice === 0 ? { id: 0, label: t`Same as this PC` } : lastN(choice)),
  );

  const usedBytes = (snapshots.data ?? []).reduce((sum, snapshot) => sum + snapshot.archiveSizeBytes, 0);

  const changeFolder = async () => {
    const picked = await pickFolder(t`Choose where snapshots are stored`, folder.data);
    if (!picked) return;
    update.mutate(
      { snapshotFolder: picked },
      {
        onSuccess: () => {
          void client.invalidateQueries({ queryKey: queryKeys.snapshotFolder });
          notify({ tone: "success", title: t`Snapshot folder changed`, body: t`New snapshots go to ${picked}.` });
        },
        onError: () =>
          notify({ tone: "danger", title: t`This folder can’t be used`, body: t`Choose a folder on a local drive.` }),
      },
    );
  };

  return (
    <>
      <SettingGroup title={t`Automatic snapshots`}>
        <ToggleSetting
          title={t`When a game closes`}
          description={t`Copies the save a few seconds after you stop playing, if it changed.`}
          checked={settings.snapshotOnExit}
          onChange={(snapshotOnExit) => update.mutate({ snapshotOnExit })}
        />
        <ToggleSetting
          title={t`Before launching a game`}
          description={t`Copies the save just before L8Play starts a game. Launching takes a moment longer.`}
          checked={settings.snapshotBeforeLaunch}
          onChange={(snapshotBeforeLaunch) => update.mutate({ snapshotBeforeLaunch })}
        />
        <SettingRow
          title={t`Before every restore`}
          description={t`Your current save is always copied first, so a restore can be undone.`}
          control={<AlwaysOn label={t`Always on`} />}
        />
      </SettingGroup>

      <SettingGroup title={t`Retention`}>
        <SettingRow
          title={t`Keep on this PC`}
          description={t`Oldest auto snapshots are removed first.`}
          control={
            <Dropdown<number>
              label={t`Keep on this PC`}
              value={settings.keepAutomatic}
              options={pcOptions}
              onChange={(keepAutomatic) => update.mutate({ keepAutomatic })}
            />
          }
        />
        <SettingRow
          title={t`Keep in Google Drive`}
          description={t`Older automatic snapshots are removed from Google Drive too, so it never piles up.`}
          control={
            <Dropdown<number>
              label={t`Keep in Google Drive`}
              value={settings.keepCloud}
              options={cloudOptions}
              onChange={(keepCloud) => update.mutate({ keepCloud })}
            />
          }
        />
        <SettingRow
          title={t`Manual, pinned and safety snapshots`}
          description={t`Only you can remove a snapshot you named or pinned yourself.`}
          control={<AlwaysOn label={t`Always kept`} />}
        />
      </SettingGroup>

      <SettingGroup title={t`Storage`}>
        <div className="st-path">
          <TextField
            label={t`Snapshot folder`}
            value={folder.data ?? ""}
            readOnly
            mono
            icon="folder"
            hint={
              usedBytes > 0
                ? t`${formatBytes(usedBytes, i18n.locale)} used · existing snapshots stay where they are`
                : t`Existing snapshots stay where they are.`
            }
          />
          <Button variant="ghost" onClick={() => reveal.mutate(null)}>
            {t`Open`}
          </Button>
          <Button variant="secondary" onClick={() => void changeFolder()}>
            {t`Change…`}
          </Button>
        </div>
        <SettingRow
          title={t`Verify every snapshot`}
          description={t`Checks a SHA-256 hash after writing, so a damaged copy is caught immediately.`}
          control={<AlwaysOn label={t`Always on`} />}
        />
      </SettingGroup>
    </>
  );
}
