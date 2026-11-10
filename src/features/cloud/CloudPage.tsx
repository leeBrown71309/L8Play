import { useLingui } from "@lingui/react/macro";
import { useState } from "react";

import { DsIcon } from "@/components/ds/ds-icon";
import { PageHeader } from "@/components/ds/page-blocks";
import { StatusBadge } from "@/components/ds/status-badge";
import { notify } from "@/components/feedback/toast-store";
import { Button } from "@/components/ui/button";
import { Dialog, Kbd } from "@/components/ui/dialog";
import { Switch } from "@/components/ui/switch";
import { AdvancedClient } from "@/features/cloud/AdvancedClient";
import { ConflictDialog } from "@/features/cloud/ConflictDialog";
import { ConflictNotices } from "@/features/cloud/ConflictNotices";
import { DriveFolders, StorageCard } from "@/features/cloud/StoragePanels";
import { TransferList } from "@/features/cloud/TransferList";
import {
  useCloudErrorText,
  useCloudQuota,
  useCloudStatus,
  useConflicts,
  useConnectGoogleDrive,
  useDisconnectGoogleDrive,
  useRetrySync,
  useSyncNow,
  useSyncOperations,
} from "@/features/cloud/use-cloud";
import { useGames, useRunningGames } from "@/features/library/use-library";
import { useSnapshots } from "@/features/saves/use-saves";
import { useSettings, useUpdateSettings } from "@/features/settings/use-settings";
import type { CloudStatus } from "@/lib/bindings/CloudStatus";
import type { ConflictInfo } from "@/lib/bindings/ConflictInfo";
import "@/features/cloud/cloud-page.css";

/** "12 Sep 2026" in the user's language. */
function formatDay(iso: string, locale: string): string {
  return new Intl.DateTimeFormat(locale, { day: "numeric", month: "short", year: "numeric" }).format(new Date(iso));
}

interface ConnectCardProps {
  connecting: boolean;
  onConnect: () => void;
}

/** Shown while no account is connected and a Google client is available. */
function ConnectCard({ connecting, onConnect }: ConnectCardProps) {
  const { t } = useLingui();
  return (
    <>
      <section className="cloud-acct is-idle">
        <span className="av">
          <DsIcon name="cloud" size={26} />
        </span>
        <div className="who">
          <b>{t`Google Drive isn’t connected`}</b>
          <div className="sub">
            {t`Everything works on this PC without it. Connecting keeps a second, verified copy of your snapshots and lets you continue on another PC.`}
          </div>
        </div>
        <div className="push">
          <Button variant="primary" icon="cloud" loading={connecting} onClick={onConnect}>
            {connecting ? t`Waiting for Google…` : t`Connect Google Drive`}
          </Button>
        </div>
      </section>
      <div className="cloud-note">
        <p>
          {t`L8Play only accesses the files it creates, in a folder named “L8Play” in your Drive. It cannot see anything else.`}
        </p>
        <p>
          {t`Google may show an “unverified app” warning while the app is not yet verified: choose “Advanced”, then “Continue”.`}
        </p>
      </div>
    </>
  );
}

interface AccountCardProps {
  data: CloudStatus;
  connected: boolean;
  connecting: boolean;
  onConnect: () => void;
  onDisconnect: () => void;
}

/** The connected Google Drive account: email, since when, and the sync badges. */
function AccountCard({ data, connected, connecting, onConnect, onDisconnect }: AccountCardProps) {
  const { t, i18n } = useLingui();
  const account = data.account;
  if (!account) return null;
  return (
    <section className="cloud-acct">
      <span className="av">
        <DsIcon name={connected ? "cloudCheck" : "cloudOff"} size={26} />
      </span>
      <div className="who">
        <b>Google Drive</b>
        <div className="sub">
          {t`${account.email} · connected since ${formatDay(account.connectedAt, i18n.locale)}`}
        </div>
        <div className="meta">
          {connected ? (
            <StatusBadge status="synced">{t`Connected`}</StatusBadge>
          ) : (
            <StatusBadge status="unprotected">{t`Sign-in needed`}</StatusBadge>
          )}
          {data.running > 0 && <StatusBadge status="uploading">{t`${data.running} in progress`}</StatusBadge>}
          {data.queued > 0 && <StatusBadge status="pending">{t`${data.queued} waiting`}</StatusBadge>}
          {data.failed > 0 && <StatusBadge status="error">{t`${data.failed} failed`}</StatusBadge>}
          {data.openConflicts > 0 && (
            <StatusBadge status="conflict">
              {data.openConflicts === 1 ? t`1 conflict` : t`${data.openConflicts} conflicts`}
            </StatusBadge>
          )}
        </div>
        {data.lastSyncAt ? null : <div className="sub">{t`Not synced yet`}</div>}
      </div>
      <div className="push">
        {!connected && (
          <Button variant="primary" size="sm" loading={connecting} onClick={onConnect}>
            {connecting ? t`Waiting for Google…` : t`Sign in again`}
          </Button>
        )}
        <Button variant="ghost" size="sm" onClick={onDisconnect}>
          {t`Disconnect`}
        </Button>
      </div>
    </section>
  );
}

/** Cloud Storage: Google Drive account, transfers, storage and the folders L8Play keeps in Drive. */
export function CloudPage() {
  const { t } = useLingui();
  const errorText = useCloudErrorText();
  const settings = useSettings();
  const updateSettings = useUpdateSettings();
  const status = useCloudStatus();
  const account = status.data?.account ?? null;
  const connected = account?.status === "connected";
  const quota = useCloudQuota(connected);
  const operations = useSyncOperations(account !== null);
  const conflicts = useConflicts(account !== null);
  const snapshots = useSnapshots(null);
  const games = useGames();
  const running = useRunningGames();
  const connect = useConnectGoogleDrive();
  const disconnect = useDisconnectGoogleDrive();
  const syncNow = useSyncNow();
  const retry = useRetrySync();
  const [open, setOpen] = useState<ConflictInfo | null>(null);
  const [confirmDisconnect, setConfirmDisconnect] = useState(false);

  const startConnect = () =>
    connect.mutate(undefined, {
      onSuccess: () => notify({ tone: "success", title: t`Google Drive connected` }),
      onError: (error) => notify({ tone: "danger", title: t`Google Drive was not connected`, body: errorText(error) }),
    });

  const sync = () =>
    syncNow.mutate(undefined, {
      onSuccess: (report) =>
        notify({
          tone: report.failed > 0 ? "warning" : "success",
          title: t`Sync finished`,
          body: t`${report.uploaded} uploaded · ${report.imported} found in Drive · ${report.failed} failed`,
        }),
      onError: (error) => notify({ tone: "danger", title: t`Sync stopped`, body: errorText(error) }),
    });

  const data = status.data;
  const header = (
    <PageHeader
      title={t`Cloud Storage`}
      subtitle={t`Snapshots live in a private “L8Play” folder in your own Google Drive. Free, and only this app can see it.`}
      actions={
        connected ? (
          <Button variant="primary" icon="sync" loading={syncNow.isPending} onClick={sync}>
            {t`Sync now`}
          </Button>
        ) : undefined
      }
    />
  );

  if (!data) return header;

  const allSnapshots = snapshots.data ?? [];
  const allGames = games.data ?? [];

  return (
    <>
      {header}

      {account && conflicts.data ? <ConflictNotices conflicts={conflicts.data} onResolve={setOpen} /> : null}

      {account ? (
        <div className="cloud-cols">
          <div className="cloud-col">
            <AccountCard
              data={data}
              connected={connected}
              connecting={connect.isPending}
              onConnect={startConnect}
              onDisconnect={() => setConfirmDisconnect(true)}
            />
            <TransferList
              operations={operations.data ?? []}
              games={allGames}
              canRetry={data.failed > 0 && connected}
              retrying={retry.isPending}
              onRetry={() =>
                retry.mutate(undefined, {
                  onError: (error) => notify({ tone: "danger", title: t`Retry failed`, body: errorText(error) }),
                })
              }
            />
            <label className="l8-setting">
              <span className="l8-setting-text">
                <span className="l8-setting-label">{t`Upload automatically`}</span>
                <span className="l8-setting-desc">
                  {t`New snapshots are copied to Drive in the background. Safety copies stay on this PC.`}
                </span>
              </span>
              <Switch
                label={t`Upload automatically`}
                checked={settings.cloudAutoSync}
                onChange={(cloudAutoSync) => updateSettings.mutate({ cloudAutoSync })}
              />
            </label>
          </div>
          <div className="cloud-col">
            {connected && <StorageCard quota={quota.data} snapshots={allSnapshots} />}
            <DriveFolders snapshots={allSnapshots} games={allGames} />
          </div>
        </div>
      ) : (
        data.clientSource !== "none" && (
          <div className="cloud-single">
            <ConnectCard connecting={connect.isPending} onConnect={startConnect} />
          </div>
        )
      )}

      <AdvancedClient source={data.clientSource} connected={account !== null} />

      <ConflictDialog conflict={open} running={open ? running.has(open.gameId) : false} onClose={() => setOpen(null)} />

      <Dialog
        open={confirmDisconnect}
        onOpenChange={setConfirmDisconnect}
        tone="warning"
        width={480}
        icon={<DsIcon name="cloud" size={22} />}
        title={t`Disconnect Google Drive?`}
        description={account?.email}
        hint={
          <>
            <Kbd>Esc</Kbd> {t`cancel`}
          </>
        }
        actions={
          <>
            <Button variant="ghost" onClick={() => setConfirmDisconnect(false)}>
              {t`Stay connected`}
            </Button>
            <Button
              variant="primary"
              size="lg"
              disabled={disconnect.isPending}
              onClick={() =>
                disconnect.mutate(undefined, {
                  onSuccess: () => {
                    setConfirmDisconnect(false);
                    notify({ tone: "neutral", title: t`Google Drive disconnected` });
                  },
                  onError: (error) => notify({ tone: "danger", title: t`Disconnect failed`, body: errorText(error) }),
                })
              }
            >
              {t`Disconnect`}
            </Button>
          </>
        }
      >
        <p className="m-0 text-sm leading-[21px] text-ink-muted">
          {t`Uploads stop and L8Play’s access to your Drive is revoked. Your snapshots on this PC and the files already in Drive are not deleted.`}
        </p>
      </Dialog>
    </>
  );
}
