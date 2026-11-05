import { useLingui } from "@lingui/react/macro";

import type { DsIconName } from "@/components/ds/ds-icon";
import { useCloudSummary } from "@/components/shell/use-cloud-summary";
import { ListRow } from "@/features/console/components/ListRow";
import { useZoneFocus } from "@/features/console/core/nav-store";
import { consoleToast } from "@/features/console/core/toast-store";
import { useViewInput } from "@/features/console/core/view-input";
import { consoleSound } from "@/features/console/sound";
import {
  useCloudErrorText,
  useCloudStatus,
  useConnectGoogleDrive,
  useRetrySync,
  useSyncNow,
} from "@/features/cloud/use-cloud";
import { formatBytes, formatRelative } from "@/lib/format";
import { cn } from "@/lib/cn";

interface Action {
  id: string;
  label: string;
  detail?: string | undefined;
  icon: DsIconName;
  disabled?: boolean | undefined;
  run: () => void;
}

/** Google Drive at a glance, with the few actions that make sense from the couch. */
export function CloudView() {
  const { t, i18n } = useLingui();
  const status = useCloudStatus().data;
  const summary = useCloudSummary();
  const errorText = useCloudErrorText();
  const sync = useSyncNow();
  const retry = useRetrySync();
  const connect = useConnectGoogleDrive();
  const account = status?.account ?? null;

  const fail = (title: string, error: unknown) => {
    consoleSound.play("error");
    consoleToast({ tone: "danger", title, body: errorText(error) });
  };

  const actions: Action[] = account
    ? [
        {
          id: "sync",
          label: sync.isPending ? t`Syncing…` : t`Sync now`,
          detail: t`Upload new snapshots and look for ones from other PCs`,
          icon: "sync",
          disabled: sync.isPending,
          run: () =>
            sync.mutate(undefined, {
              onSuccess: (report) =>
                consoleToast({
                  tone: report.failed > 0 ? "warning" : "success",
                  title: t`Sync finished`,
                  body: t`${report.uploaded} uploaded · ${report.downloaded} downloaded`,
                }),
              onError: (error) => fail(t`Sync failed`, error),
            }),
        },
        ...((status?.failed ?? 0) > 0
          ? [
              {
                id: "retry",
                label: t`Retry failed uploads`,
                detail: t`${status?.failed ?? 0} uploads failed`,
                icon: "cloudUp" as const,
                disabled: retry.isPending,
                run: () =>
                  retry.mutate(undefined, {
                    onSuccess: () => consoleToast({ tone: "neutral", title: t`Uploads queued again` }),
                    onError: (error) => fail(t`Retry failed`, error),
                  }),
              },
            ]
          : []),
      ]
    : [
        {
          id: "connect",
          label: connect.isPending ? t`Waiting for Google…` : t`Connect Google Drive`,
          detail: t`Opens your browser to sign in. L8Play only sees the files it creates.`,
          icon: "cloud",
          disabled: connect.isPending,
          run: () =>
            connect.mutate(undefined, {
              onSuccess: () => consoleToast({ tone: "success", title: t`Google Drive connected` }),
              onError: (error) => fail(t`Google Drive was not connected`, error),
            }),
        },
      ];

  const zone = useZoneFocus("cloud.actions", actions.length);
  useViewInput((action) => {
    if (action === "up" || action === "down") {
      const next = Math.min(actions.length - 1, Math.max(0, zone.index + (action === "down" ? 1 : -1)));
      if (next !== zone.index) consoleSound.play("move");
      zone.set(next);
    } else if (action === "confirm") {
      const item = actions[zone.index];
      if (item && !item.disabled) {
        consoleSound.play("select");
        item.run();
      }
    }
  });

  const quota = status?.quota ?? null;
  const facts: [string, string][] = [
    [t`Account`, account ? account.email : t`Not connected`],
    [
      t`Google storage`,
      quota
        ? quota.limitBytes
          ? t`${formatBytes(quota.usedBytes, i18n.locale)} of ${formatBytes(quota.limitBytes, i18n.locale)}`
          : formatBytes(quota.usedBytes, i18n.locale)
        : "—",
    ],
    [t`Waiting to upload`, String((status?.queued ?? 0) + (status?.running ?? 0))],
    [t`Failed uploads`, String(status?.failed ?? 0)],
    [t`Sync conflicts`, String(status?.openConflicts ?? 0)],
    [t`Last sync`, status?.lastSyncAt ? formatRelative(status.lastSyncAt, i18n.locale) : "—"],
  ];

  return (
    <section className="cs-section cs-two" aria-label={t`Cloud`}>
      <div>
        <h1 className="cs-section-title">{t`Cloud`}</h1>
        <p className={cn("cs-state", `tone-${summary.tone}`)}>{summary.label}</p>
        <dl className="cs-facts is-big">
          {facts.map(([label, value]) => (
            <div key={label}>
              <dt>{label}</dt>
              <dd>{value}</dd>
            </div>
          ))}
        </dl>
        {(status?.openConflicts ?? 0) > 0 ? (
          <p className="cs-note">{t`Resolve sync conflicts in the desktop interface.`}</p>
        ) : null}
      </div>
      <div className="cs-actions" role="menu">
        {actions.map((item, index) => (
          <ListRow
            key={item.id}
            label={item.label}
            detail={item.detail}
            icon={item.icon}
            disabled={item.disabled}
            focused={zone.index === index}
            onFocus={() => zone.set(index)}
            onActivate={() => !item.disabled && item.run()}
          />
        ))}
      </div>
    </section>
  );
}
