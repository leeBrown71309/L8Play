import { useLingui } from "@lingui/react/macro";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { DsIcon } from "@/components/ds/ds-icon";
import { Banner } from "@/components/ds/page-blocks";
import { ProgressBar } from "@/components/ds/progress-bar";
import { StatusBadge, type StatusKind } from "@/components/ds/status-badge";
import { Button } from "@/components/ui/button";
import { Dialog } from "@/components/ui/dialog";
import { useCloudStatus } from "@/features/cloud/use-cloud";
import { useRunningGames } from "@/features/library/use-library";
import { useSettings, useUpdateSettings } from "@/features/settings/use-settings";
import { SettingGroup, ToggleSetting } from "@/features/settings/setting-blocks";
import { cleanReleaseNotes, formatUpdateDate, type UpdateErrorCode } from "@/features/updates/update-format";
import { useAppUpdate } from "@/features/updates/use-app-update";
import { formatBytes } from "@/lib/format";
import { queryKeys } from "@/lib/query-keys";
import { getAppInfo } from "@/lib/tauri/commands";

import "@/features/updates/updates.css";

/** Settings › Updates: version, check, release notes, download progress and the auto-check switch. */
export function UpdatesSection() {
  const { t, i18n } = useLingui();
  const settings = useSettings();
  const saveSettings = useUpdateSettings();
  const update = useAppUpdate();
  const { data: appInfo } = useQuery({ queryKey: queryKeys.appInfo, queryFn: getAppInfo });
  const runningGames = useRunningGames();
  const cloud = useCloudStatus().data;
  const [confirmOpen, setConfirmOpen] = useState(false);

  const currentVersion = appInfo?.version ?? update.currentVersion;
  const published = formatUpdateDate(update.update?.date, i18n.locale);
  const notes = cleanReleaseNotes(update.update?.body);
  const gameActive = runningGames.size > 0;
  const syncActive = (cloud?.queued ?? 0) + (cloud?.running ?? 0) > 0;
  const busy = gameActive || syncActive;
  const busyText =
    gameActive && syncActive
      ? t`A game is running and a backup is in progress. Restarting now may interrupt both.`
      : gameActive
        ? t`A game is running. Restarting now closes L8Play and may interrupt automatic snapshots.`
        : t`A backup or Drive upload is in progress. Restarting now may interrupt it.`;

  const status: { kind: StatusKind; label: string } = (() => {
    switch (update.phase) {
      case "idle":
        return { kind: "info", label: t`Not checked yet` };
      case "checking":
        return { kind: "pending", label: t`Checking…` };
      case "available":
        return { kind: "info", label: t`Update available` };
      case "downloading":
        return { kind: "downloading", label: t`Downloading` };
      case "ready":
        return { kind: "verified", label: t`Ready to install` };
      case "upToDate":
        return { kind: "verified", label: t`Up to date` };
      case "error":
        return { kind: "error", label: t`Check failed` };
    }
  })();

  const errorCopy: Record<UpdateErrorCode, { title: string; body: string }> = {
    offline: { title: t`You are offline`, body: t`Check your connection, then try again.` },
    signature: {
      title: t`The update signature is invalid`,
      body: t`The downloaded file was discarded. Try again, or download the installer from GitHub.`,
    },
    unreachable: {
      title: t`GitHub could not be reached`,
      body: t`Check your connection or try again later.`,
    },
    unknown: {
      title: t`The update check failed`,
      body: t`Try again. If it keeps failing, download the installer from GitHub.`,
    },
  };

  const downloadMeta =
    update.totalBytes !== null
      ? `${formatBytes(update.downloadedBytes, i18n.locale)} / ${formatBytes(update.totalBytes, i18n.locale)}`
      : formatBytes(update.downloadedBytes, i18n.locale);

  const requestInstall = (): void => {
    if (busy) {
      setConfirmOpen(true);
      return;
    }
    void update.install();
  };

  const confirmInstall = (): void => {
    setConfirmOpen(false);
    void update.install();
  };

  return (
    <>
      <SettingGroup>
        <div className="st-card upd-card" data-phase={update.phase}>
          <div className="head">
            <DsIcon name="download" size={16} />
            {t`L8Play updates`}
            <StatusBadge status={status.kind}>{status.label}</StatusBadge>
          </div>
          <dl className="upd-meta">
            <dt>{t`Current version`}</dt>
            <dd>{currentVersion || "…"}</dd>
            {update.update ? (
              <>
                <dt>{t`New version`}</dt>
                <dd>{update.update.version}</dd>
              </>
            ) : null}
            {published ? (
              <>
                <dt>{t`Published`}</dt>
                <dd>{published}</dd>
              </>
            ) : null}
          </dl>

          {notes ? <div className="upd-notes">{notes}</div> : null}
          {update.phase === "upToDate" ? <p className="upd-hint">{t`You are running the latest version.`}</p> : null}

          {update.phase === "error" && update.error ? (
            <Banner
              tone="danger"
              title={errorCopy[update.error].title}
              action={
                <Button size="sm" onClick={() => void update.check()}>
                  {t`Retry`}
                </Button>
              }
            >
              {errorCopy[update.error].body}
            </Banner>
          ) : null}

          {update.phase === "downloading" ? (
            <ProgressBar
              className="upd-progress"
              value={update.progress ?? 0}
              indeterminate={update.progress === null}
              label={t`Downloading the update`}
              meta={downloadMeta}
            />
          ) : null}

          {update.phase === "ready" ? (
            <Banner tone="success" title={t`The update is installed`}>
              {t`L8Play will restart to finish.`}
            </Banner>
          ) : null}

          <div className="upd-actions">
            {update.phase === "available" ? (
              <Button variant="primary" icon="download" onClick={requestInstall}>
                {t`Update now`}
              </Button>
            ) : null}
            {update.phase === "ready" ? (
              <Button variant="primary" icon="sync" onClick={() => void update.relaunch()}>
                {t`Restart now`}
              </Button>
            ) : null}
            <Button
              icon="sync"
              loading={update.phase === "checking"}
              disabled={update.phase === "downloading" || update.phase === "ready"}
              onClick={() => void update.check()}
            >
              {t`Check for updates`}
            </Button>
          </div>

          {update.phase === "available" && update.update ? (
            <p className="upd-hint">{t`L8Play will close, install version ${update.update.version} and restart.`}</p>
          ) : null}
        </div>

        <ToggleSetting
          title={t`Check for updates automatically`}
          description={t`Looks for a newer version when L8Play starts and every 6 hours. Nothing is downloaded or installed until you ask.`}
          checked={settings.updatesAutoCheck}
          onChange={(updatesAutoCheck) => saveSettings.mutate({ updatesAutoCheck })}
        />
      </SettingGroup>

      <Dialog
        open={confirmOpen}
        onOpenChange={setConfirmOpen}
        title={t`Install the update now?`}
        description={t`L8Play will close and restart to finish.`}
        icon={<DsIcon name="download" size={20} />}
        tone="warning"
        width={520}
        hint={t`Esc cancels`}
        actions={
          <>
            <Button variant="ghost" onClick={() => setConfirmOpen(false)}>
              {t`Later`}
            </Button>
            <Button variant="primary" onClick={confirmInstall}>
              {t`Update anyway`}
            </Button>
          </>
        }
      >
        <p className="upd-dialog-text">{busyText}</p>
      </Dialog>
    </>
  );
}
