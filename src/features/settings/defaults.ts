import type { AppSettings } from "@/lib/bindings/AppSettings";

/** Mirrors `AppSettings::default()` in crates/l8-core/src/settings.rs. */
export const DEFAULT_SETTINGS: AppSettings = {
  theme: "ember",
  locale: "en",
  navExpanded: false,
  snapshotOnExit: true,
  snapshotBeforeLaunch: false,
  keepAutomatic: 10,
  keepCloud: 0,
  snapshotFolder: "",
  cloudAutoSync: true,
  closeToTray: true,
  launchAtStartup: false,
  desktopNotifications: true,
  onboardingDone: false,
  showOwnedNotInstalled: true,
};
