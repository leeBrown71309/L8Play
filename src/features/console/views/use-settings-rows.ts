import { useLingui } from "@lingui/react/macro";

import { requestConsoleExit } from "@/features/console/core/use-request-exit";
import { useSoundSettings } from "@/features/console/sound";
import type { SettingsRow } from "@/features/console/views/settings-rows";
import { cloudRetentionChoices, pcRetentionChoices } from "@/features/settings/retention-options";
import { useSettings, useUpdateSettings } from "@/features/settings/use-settings";
import { useAppUpdate } from "@/features/updates/use-app-update";
import type { Locale } from "@/lib/bindings/Locale";
import type { Theme } from "@/lib/bindings/Theme";

/** The rows of the console settings list, built from the real settings so both interfaces stay in step. */
export function useSettingsRows(): SettingsRow[] {
  const { t } = useLingui();
  const settings = useSettings();
  const update = useUpdateSettings().mutate;
  const sound = useSoundSettings();
  const updater = useAppUpdate();

  const updateRow: SettingsRow =
    updater.phase === "available"
      ? {
          kind: "action",
          id: "update",
          label: t`Update now`,
          detail: updater.update ? t`Version ${updater.update.version} is ready. L8Play will restart.` : "",
          icon: "download",
          run: () => void updater.install(),
        }
      : updater.phase === "downloading"
        ? {
            kind: "action",
            id: "update",
            label: t`Downloading update…`,
            detail: `${updater.progress ?? 0}%`,
            icon: "download",
            disabled: true,
            run: () => undefined,
          }
        : {
            kind: "action",
            id: "update",
            label: updater.phase === "checking" ? t`Checking…` : t`Check for updates`,
            detail:
              updater.phase === "upToDate"
                ? t`You are running the latest version.`
                : updater.phase === "error"
                  ? t`The check failed. Try again.`
                  : updater.currentVersion
                    ? t`Version ${updater.currentVersion}`
                    : undefined,
            icon: "sync",
            disabled: updater.phase === "checking",
            run: () => void updater.check(),
          };

  return [
    { kind: "toggle", id: "sound", label: t`Sound effects`, icon: "bell", value: sound.enabled, set: sound.setEnabled },
    {
      kind: "slider",
      id: "volume",
      label: t`Volume`,
      icon: "bell",
      value: sound.volume,
      set: sound.setVolume,
      disabled: !sound.enabled,
    },
    {
      kind: "toggle",
      id: "ambient",
      label: t`Ambient sound`,
      detail: t`A soft background tone while console mode is open`,
      icon: "sparkle",
      value: sound.ambient,
      set: sound.setAmbient,
      disabled: !sound.enabled,
    },
    {
      kind: "toggle",
      id: "start-in-console",
      label: t`Start in console mode`,
      detail: t`Open this interface as soon as L8Play starts`,
      icon: "gamepad",
      value: settings.startInConsole,
      set: (value) => update({ startInConsole: value }),
    },
    {
      kind: "picker",
      id: "language",
      label: t`Language`,
      icon: "palette",
      options: [
        { value: "en", label: "English" },
        { value: "fr", label: "Français" },
      ],
      value: settings.locale,
      set: (value) => update({ locale: value as Locale }),
    },
    {
      kind: "picker",
      id: "theme",
      label: t`Theme`,
      icon: "palette",
      options: [
        { value: "ember", label: t`Ember (dark)` },
        { value: "rose", label: t`Rosé (light)` },
      ],
      value: settings.theme,
      set: (value) => update({ theme: value as Theme }),
    },
    {
      kind: "toggle",
      id: "snapshot-on-exit",
      label: t`Snapshot when a game closes`,
      icon: "snapshot",
      value: settings.snapshotOnExit,
      set: (value) => update({ snapshotOnExit: value }),
    },
    {
      kind: "picker",
      id: "keep-pc",
      label: t`Keep on this PC`,
      detail: t`Automatic snapshots per game`,
      icon: "drive",
      options: pcRetentionChoices(settings.keepAutomatic).map((value) => ({
        value: String(value),
        label: String(value),
      })),
      value: String(settings.keepAutomatic),
      set: (value) => update({ keepAutomatic: Number(value) }),
    },
    {
      kind: "picker",
      id: "keep-cloud",
      label: t`Keep in Google Drive`,
      detail: t`Automatic snapshots per game`,
      icon: "cloud",
      options: cloudRetentionChoices(settings.keepAutomatic, settings.keepCloud).map((value) => ({
        value: String(value),
        label: value === 0 ? t`Same as this PC` : String(value),
      })),
      value: String(settings.keepCloud),
      set: (value) => update({ keepCloud: Number(value) }),
    },
    updateRow,
    { kind: "action", id: "exit", label: t`Exit console mode`, icon: "arrowRight", run: requestConsoleExit },
  ];
}
