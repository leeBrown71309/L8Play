import { useLingui } from "@lingui/react/macro";
import { useEffect, useState, type ReactNode } from "react";
import { useNavigate } from "react-router";

import { ROUTES } from "@/app/routes";
import { Checkbox } from "@/components/ds/form-fields";
import { ProgressBar } from "@/components/ds/progress-bar";
import { Banner } from "@/components/ds/page-blocks";
import { notify } from "@/components/feedback/toast-store";
import { CaptionBar } from "@/components/shell/CaptionBar";
import { Mark } from "@/components/shell/Mark";
import { Button } from "@/components/ui/button";
import { Kbd } from "@/components/ui/dialog";
import { useCloudErrorText, useCloudStatus, useConnectGoogleDrive } from "@/features/cloud/use-cloud";
import { useGames, useScanLibraries } from "@/features/library/use-library";
import { useAddGameFlow } from "@/features/library/use-library-flows";
import { CoverWall, DoneList, PlatformList, ScanGlass } from "@/features/onboarding/OnboardingParts";
import { useSaveStatuses } from "@/features/saves/use-saves";
import { ToggleSetting } from "@/features/settings/setting-blocks";
import { useSettings, useUpdateSettings } from "@/features/settings/use-settings";
import type { PlatformScan } from "@/lib/bindings/PlatformScan";
import { cn } from "@/lib/cn";
import "@/features/onboarding/onboarding.css";

const STEP_COUNT = 6;
const LAST = STEP_COUNT - 1;

/** Interactive controls handle Enter themselves; the shortcut must not double-fire on them. */
function isInteractive(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLElement && target.closest("button, input, textarea, select, a, [role=checkbox]") !== null
  );
}

/** First launch: welcome, launchers, scan, backups, Google Drive, done (design system layout). */
export function OnboardingDialog() {
  const { t } = useLingui();
  const navigate = useNavigate();
  const settings = useSettings();
  const update = useUpdateSettings();
  const games = useGames();
  const statuses = useSaveStatuses();
  const cloud = useCloudStatus();
  const errorText = useCloudErrorText();
  const connect = useConnectGoogleDrive();
  const scanLibraries = useScanLibraries();
  const addGame = useAddGameFlow();
  const [step, setStep] = useState(0);
  const [scan, setScan] = useState<PlatformScan[] | null>(null);
  const [uploadExisting, setUploadExisting] = useState(true);

  const open = !settings.onboardingDone;
  const finish = () => update.mutate({ onboardingDone: true });
  const next = () => setStep((current) => Math.min(LAST, current + 1));
  const library = games.data ?? [];
  const saveFolders = (statuses.data ?? []).filter((status) => status.locationCount > 0).length;
  const artworkSets = library.filter((game) => game.artwork.cover !== null).length;
  const account = cloud.data?.account ?? null;
  const driveClient = cloud.data ? cloud.data.clientSource !== "none" : false;

  const startScan = () => {
    scanLibraries.mutate(undefined, {
      onSuccess: (report) => setScan(report.platforms),
      onError: () =>
        notify({ tone: "danger", title: t`The scan failed`, body: t`Check the log file for details, then try again.` }),
    });
    next();
  };

  const connectDrive = () => {
    if (!driveClient) {
      finish();
      void navigate(ROUTES.cloud);
      return;
    }
    connect.mutate(undefined, {
      onSuccess: () => {
        notify({ tone: "success", title: t`Google Drive connected` });
        next();
      },
      onError: (error) => notify({ tone: "danger", title: t`Google Drive was not connected`, body: errorText(error) }),
    });
  };

  useEffect(() => {
    if (!open) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") finish();
      if (event.key === "Enter" && !isInteractive(event.target) && step !== 1 && step !== 4 && step !== LAST) next();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
    // finish/next only close over stable mutation handles and the setter.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, step]);

  if (!open) return null;

  const em = (text: string) => <em>{text}</em>;
  const copy: { eyebrow: string; title: ReactNode; text: string }[] = [
    {
      eyebrow: t`Welcome`,
      title: (
        <>
          {t`Your entire gaming world.`} {em(t`Every save protected.`)}
        </>
      ),
      text: t`L8Play finds your games, launches them from one place and keeps a copy of every save — on this PC and, if you like, in your own Google Drive.`,
    },
    {
      eyebrow: t`Step 1 of 4 · Launchers`,
      title: t`Where are your games?`,
      text: t`We look for game launchers on this PC. Scan again any time from My Games, or add any game by hand.`,
    },
    {
      eyebrow: t`Step 2 of 4 · Library`,
      title: t`Scanning your library`,
      text: t`Finding games, their save folders and their artwork. You can keep going — this finishes in the background.`,
    },
    {
      eyebrow: t`Step 3 of 4 · Backup`,
      title: t`Protect your saves`,
      text: t`Snapshots are verified copies of your saves. Choose when L8Play makes them.`,
    },
    {
      eyebrow: t`Step 4 of 4 · Google Drive`,
      title: t`Keep a copy in Google Drive?`,
      text: t`Uses your free Google storage. L8Play gets its own private folder and cannot see your other files.`,
    },
    {
      eyebrow: t`All set`,
      title: (
        <>
          {t`You’re`} {em(t`protected.`)}
        </>
      ),
      text: t`Snapshots happen on their own when a game closes. You can change anything later in Settings.`,
    },
  ];
  const current = copy[step] ?? { eyebrow: "", title: "", text: "" };

  const doneItems = [
    library.length === 1 ? t`1 game in your library` : t`${library.length} games in your library`,
    saveFolders === 1 ? t`1 save folder protected` : t`${saveFolders} save folders protected`,
    settings.snapshotOnExit ? t`Auto snapshots when a game closes` : t`Snapshots are made on demand`,
    account ? t`Google Drive connected · ${account.email}` : t`Snapshots stay on this PC`,
  ];

  const primary: ReactNode = (() => {
    switch (step) {
      case 0:
        return (
          <Button variant="primary" size="lg" iconRight="arrowRight" onClick={next}>
            {t`Get started`}
          </Button>
        );
      case 1:
        return (
          <Button variant="primary" size="lg" iconRight="arrowRight" onClick={startScan}>
            {t`Scan my libraries`}
          </Button>
        );
      case 4:
        return account ? (
          <Button variant="primary" size="lg" iconRight="arrowRight" onClick={next}>
            {t`Continue`}
          </Button>
        ) : (
          <Button variant="primary" size="lg" icon="cloud" loading={connect.isPending} onClick={connectDrive}>
            {driveClient ? t`Connect Google Drive` : t`Set up Google Drive`}
          </Button>
        );
      case LAST:
        return (
          <Button
            variant="play"
            size="lg"
            icon="play"
            onClick={() => {
              finish();
              void navigate(ROUTES.games);
            }}
          >
            {t`Open my library`}
          </Button>
        );
      default:
        return (
          <Button variant="primary" size="lg" iconRight="arrowRight" onClick={next}>
            {step === 2 ? t`Continue` : t`Next`}
          </Button>
        );
    }
  })();

  return (
    <div className="ob" role="dialog" aria-modal="true" aria-label={t`Welcome to L8Play`}>
      <CaptionBar />
      <div className="ob-left">
        <div className="l8-logo">
          <Mark size={30} />
          <span className="l8-wordmark">
            L8<b>PLAY</b>
          </span>
        </div>
        <div className="ob-steps" role="tablist" aria-label={t`Setup steps`}>
          {copy.map((entry, index) => (
            <button
              key={entry.eyebrow}
              type="button"
              role="tab"
              aria-selected={index === step}
              aria-label={entry.eyebrow}
              className={cn(index === step ? "on" : index < step ? "done" : undefined)}
              onClick={() => setStep(index)}
            />
          ))}
        </div>
        <div>
          <div className="ob-eb">{current.eyebrow}</div>
          <h1 className="ob-h">{current.title}</h1>
          <p className="ob-p">{current.text}</p>
        </div>

        {step === 1 && (
          <div className="ob-content">
            <PlatformList games={library} scan={scan} />
            <div>
              <Button variant="ghost" size="sm" icon="plus" onClick={() => void addGame.start()}>
                {t`Add a game`}
              </Button>
            </div>
          </div>
        )}
        {step === 2 && (
          <div className="ob-content">
            <ProgressBar
              label={scanLibraries.isPending ? t`Scanning your launchers` : t`Scan finished`}
              {...(scanLibraries.isPending ? { indeterminate: true } : { value: 100, meta: "100%" })}
            />
            {!scanLibraries.isPending && (
              <Banner tone="success" icon="shieldCheck" title={t`${saveFolders} save folders found.`}>
                {t`Games without a known save folder can be completed later from their page.`}
              </Banner>
            )}
          </div>
        )}
        {step === 3 && (
          <div className="ob-content">
            <ToggleSetting
              title={t`Back up when a game closes`}
              description={t`Copies the save a few seconds after you stop playing, if it changed.`}
              checked={settings.snapshotOnExit}
              onChange={(snapshotOnExit) => update.mutate({ snapshotOnExit })}
            />
            <ToggleSetting
              title={t`Keep running in the notification area`}
              description={t`Closing the window keeps L8Play running so automatic backups still happen.`}
              checked={settings.closeToTray}
              onChange={(closeToTray) => update.mutate({ closeToTray })}
            />
            <ToggleSetting
              title={t`Start with Windows`}
              description={t`Starts hidden in the notification area when you sign in.`}
              checked={settings.launchAtStartup}
              onChange={(launchAtStartup) => update.mutate({ launchAtStartup })}
            />
          </div>
        )}
        {step === 4 && (
          <div className="ob-drive">
            <ul>
              <li>{t`Snapshots upload after they are created on this PC`}</li>
              <li>{t`15 GB free with any Google account`}</li>
              <li>{t`Disconnect any time; local snapshots stay`}</li>
            </ul>
            {driveClient && !account && (
              <Checkbox
                checked={uploadExisting}
                onChange={(value) => {
                  setUploadExisting(value);
                  update.mutate({ cloudAutoSync: value });
                }}
                label={t`Upload automatically`}
              />
            )}
          </div>
        )}
        {step === LAST && <DoneList items={doneItems} />}

        <div className="ob-foot">
          {step > 0 && (
            <Button variant="ghost" onClick={() => setStep(step - 1)}>
              {t`Back`}
            </Button>
          )}
          {primary}
          {step === 4 && !account && (
            <Button variant="ghost" onClick={next}>
              {t`Only on this PC`}
            </Button>
          )}
          {step === 0 && (
            <Button variant="ghost" onClick={finish}>
              {t`Skip`}
            </Button>
          )}
          <span className="k">
            <Kbd>Esc</Kbd>
            {t`skip the tour`}
          </span>
        </div>
      </div>

      <div className="ob-right">
        <CoverWall dimmed={step === 2} />
        {step === 2 && (
          <ScanGlass
            games={library.length}
            saveFolders={saveFolders}
            artworkSets={artworkSets}
            scanning={scanLibraries.isPending}
          />
        )}
      </div>
    </div>
  );
}
