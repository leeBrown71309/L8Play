import { useLingui } from "@lingui/react/macro";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { Link, useSearchParams } from "react-router";

import { ROUTES } from "@/app/routes";
import { DsIcon, type DsIconName } from "@/components/ds/ds-icon";
import { PageHeader } from "@/components/ds/page-blocks";
import { StatusBadge } from "@/components/ds/status-badge";
import { Button } from "@/components/ui/button";
import { Segmented } from "@/components/ui/segmented";
import { useGames } from "@/features/library/use-library";
import { useAddGameFlow, useScanFlow } from "@/features/library/use-library-flows";
import { SteamKeyBlock } from "@/features/settings/SteamKeyBlock";
import { useOwnedLibraryStatus } from "@/features/settings/use-owned-library";
import { ArtworkSection } from "@/features/settings/ArtworkSection";
import { BackupSection } from "@/features/settings/BackupSection";
import { SettingGroup, SettingRow, ToggleSetting } from "@/features/settings/setting-blocks";
import { useSettings, useUpdateSettings } from "@/features/settings/use-settings";
import { UpdatesSection } from "@/features/updates/UpdatesSection";
import type { Locale } from "@/lib/bindings/Locale";
import type { Theme } from "@/lib/bindings/Theme";
import { cn } from "@/lib/cn";
import { queryKeys } from "@/lib/query-keys";
import { getAppInfo } from "@/lib/tauri/commands";
import "@/features/settings/settings-page.css";

type SectionId =
  "general" | "libraries" | "artwork" | "detection" | "backup" | "cloud" | "updates" | "appearance" | "about";

const SECTION_IDS: readonly string[] = [
  "general",
  "libraries",
  "artwork",
  "detection",
  "backup",
  "cloud",
  "updates",
  "appearance",
  "about",
];

function isSectionId(value: string | null): value is SectionId {
  return value !== null && SECTION_IDS.includes(value);
}

function AppearanceSection() {
  const { t } = useLingui();
  const settings = useSettings();
  const update = useUpdateSettings();
  return (
    <SettingGroup>
      <SettingRow
        title={t`Theme`}
        description={t`Ember is the dark theme; Rosé is its light counterpart.`}
        control={
          <Segmented<Theme>
            label={t`Theme`}
            value={settings.theme}
            onChange={(theme) => update.mutate({ theme })}
            options={[
              { value: "ember", label: "Ember" },
              { value: "rose", label: "Rosé" },
            ]}
          />
        }
      />
      <SettingRow
        title={t`Language`}
        description={t`Interface language.`}
        control={
          <Segmented<Locale>
            label={t`Language`}
            value={settings.locale}
            onChange={(locale) => update.mutate({ locale })}
            options={[
              { value: "en", label: "English" },
              { value: "fr", label: "Français" },
            ]}
          />
        }
      />
    </SettingGroup>
  );
}

function GeneralSection() {
  const { t } = useLingui();
  const settings = useSettings();
  const update = useUpdateSettings();
  return (
    <SettingGroup>
      <ToggleSetting
        title={t`Keep running in the notification area`}
        description={t`Closing the window hides it instead of quitting, so automatic backups and Drive sync keep working. Quit from the tray icon.`}
        checked={settings.closeToTray}
        onChange={(closeToTray) => update.mutate({ closeToTray })}
      />
      <ToggleSetting
        title={t`Start with Windows`}
        description={t`Starts hidden in the notification area when you sign in. No administrator rights needed.`}
        checked={settings.launchAtStartup}
        onChange={(launchAtStartup) => update.mutate({ launchAtStartup })}
      />
      <ToggleSetting
        title={t`Windows notifications`}
        description={t`Tells you about problems that need attention (a failed backup, a save conflict, a Drive sign-in) while the window is hidden.`}
        checked={settings.desktopNotifications}
        onChange={(desktopNotifications) => update.mutate({ desktopNotifications })}
      />
    </SettingGroup>
  );
}

function LibrariesSection() {
  const { t } = useLingui();
  const scan = useScanFlow();
  const addGame = useAddGameFlow();
  const settings = useSettings();
  const updateSettings = useUpdateSettings();
  const ownedStatus = useOwnedLibraryStatus();
  const games = useGames().data ?? [];
  const count = (platform: string) => games.filter((game) => game.platform === platform).length;
  return (
    <SettingGroup>
      <SettingRow
        title={t`Game launchers`}
        description={t`Found automatically from their launchers. ${count("steam")} Steam · ${count("epic")} Epic · ${count("xbox")} Xbox · ${count("gog")} GOG · ${count("standalone")} added by hand.`}
        control={
          <Button size="sm" icon="sync" loading={scan.isScanning} onClick={scan.start}>
            {t`Scan libraries`}
          </Button>
        }
      />
      <ToggleSetting
        title={t`Show games I own but haven’t installed`}
        description={t`Lists the games your Steam account owns, read from Steam with your own free Steam Web API key or from your public profile. Nothing is sent about you except the request itself. Epic and other stores don’t offer this without signing in, so only Steam is listed.`}
        note={
          settings.showOwnedNotInstalled && ownedStatus === "private"
            ? t`Your Steam profile’s game details are private: set them to public to list owned games.`
            : settings.showOwnedNotInstalled && ownedStatus === "unavailable"
              ? t`Steam’s profile page could not be read just now. Scan again later.`
              : undefined
        }
        checked={settings.showOwnedNotInstalled}
        onChange={(checked) =>
          updateSettings.mutate(
            { showOwnedNotInstalled: checked },
            { onSuccess: () => (checked ? scan.start() : undefined) },
          )
        }
      />
      {settings.showOwnedNotInstalled && <SteamKeyBlock status={ownedStatus} onChanged={scan.start} />}
      <SettingRow
        title={t`Other games`}
        description={t`Add a whole folder of games, or any game by choosing its .exe file.`}
        control={
          <Button size="sm" icon="plus" onClick={() => void addGame.start()}>
            {t`Add a game`}
          </Button>
        }
      />
    </SettingGroup>
  );
}

function DetectionSection() {
  const { t } = useLingui();
  const { data } = useQuery({ queryKey: queryKeys.appInfo, queryFn: getAppInfo });
  return (
    <SettingGroup>
      <SettingRow
        title={t`Ludusavi save database`}
        description={t`The first place L8Play looks for where a game keeps its saves. It runs from the bundled Ludusavi tool and downloads the free community manifest.`}
        control={
          data?.ludusaviAvailable ? (
            <StatusBadge status="verified">{t`Available`}</StatusBadge>
          ) : (
            <StatusBadge status="unprotected">{t`Not found`}</StatusBadge>
          )
        }
      />
      <SettingRow
        title={t`PCGamingWiki and local search`}
        description={t`For games the database does not know, “Find saves” on a game’s page asks PCGamingWiki and searches your PC. Nothing is backed up until you confirm a folder.`}
        control={<span className="st-lock">{t`On demand`}</span>}
      />
    </SettingGroup>
  );
}

function CloudSection() {
  const { t } = useLingui();
  const settings = useSettings();
  const update = useUpdateSettings();
  return (
    <SettingGroup>
      <ToggleSetting
        title={t`Upload snapshots automatically`}
        description={t`Copies new snapshots to Google Drive in the background while an account is connected.`}
        checked={settings.cloudAutoSync}
        onChange={(cloudAutoSync) => update.mutate({ cloudAutoSync })}
      />
      <SettingRow
        title={t`Google Drive account`}
        description={t`Sign-in, storage, the sync queue and conflicts are on the Cloud Storage page.`}
        control={
          <Button asChild size="sm">
            <Link to={ROUTES.cloud}>{t`Open Cloud Storage`}</Link>
          </Button>
        }
      />
    </SettingGroup>
  );
}

function AboutSection() {
  const { t } = useLingui();
  const update = useUpdateSettings();
  const { data, isPending, isError } = useQuery({ queryKey: queryKeys.appInfo, queryFn: getAppInfo });
  const rows: [string, string][] = data
    ? [
        [t`Version`, data.version],
        [t`Database`, data.databasePath],
        [t`Schema version`, String(data.schemaVersion)],
      ]
    : [];
  return (
    <div className="st-card st-about">
      {isPending && <p>{t`Loading…`}</p>}
      {isError && <p className="text-danger">{t`Application details could not be read.`}</p>}
      <dl>
        {rows.map(([label, value]) => (
          <div key={label} className="contents">
            <dt>{label}</dt>
            <dd>{value}</dd>
          </div>
        ))}
      </dl>
      <div className="mt-4">
        <Button size="sm" variant="ghost" onClick={() => update.mutate({ onboardingDone: false })}>
          {t`Show the welcome tour again`}
        </Button>
      </div>
      <p className="small">{t`Save-location knowledge comes from the Ludusavi manifest and PCGamingWiki (CC BY-NC-SA 3.0).`}</p>
    </div>
  );
}

/** Settings: a section list on the left, grouped rows on the right (design system layout). */
export function SettingsPage() {
  const { t } = useLingui();
  // `?section=artwork` lets other pages deep-link to a section.
  const requested = useSearchParams()[0].get("section");
  const [active, setActive] = useState<SectionId>(isSectionId(requested) ? requested : "appearance");
  const sections: { id: SectionId; label: string; icon: DsIconName }[] = [
    { id: "general", label: t`General`, icon: "settings" },
    { id: "libraries", label: t`Game Libraries`, icon: "layers" },
    { id: "artwork", label: t`Artwork`, icon: "image" },
    { id: "detection", label: t`Save Detection`, icon: "scan" },
    { id: "backup", label: t`Backup`, icon: "snapshot" },
    { id: "cloud", label: t`Cloud`, icon: "cloud" },
    { id: "updates", label: t`Updates`, icon: "download" },
    { id: "appearance", label: t`Appearance`, icon: "palette" },
    { id: "about", label: t`About`, icon: "info" },
  ];

  return (
    <>
      <PageHeader title={t`Settings`} />
      <div className="st">
        <nav aria-label={t`Settings sections`} className="st-nav">
          {sections.map(({ id, label, icon }) => (
            <button
              key={id}
              type="button"
              aria-current={id === active ? "page" : undefined}
              onClick={() => setActive(id)}
              className={cn("st-item", id === active && "on")}
            >
              <span className="ic">
                <DsIcon name={icon} size={16} />
              </span>
              {label}
            </button>
          ))}
        </nav>
        <div className="st-body">
          {active === "appearance" && <AppearanceSection />}
          {active === "about" && <AboutSection />}
          {active === "backup" && <BackupSection />}
          {active === "artwork" && <ArtworkSection />}
          {active === "cloud" && <CloudSection />}
          {active === "updates" && <UpdatesSection />}
          {active === "general" && <GeneralSection />}
          {active === "libraries" && <LibrariesSection />}
          {active === "detection" && <DetectionSection />}
        </div>
      </div>
    </>
  );
}
