import { queryClient } from "@/app/query-client";
import { applyTheme } from "@/features/settings/apply-theme";
import { DEFAULT_SETTINGS } from "@/features/settings/defaults";
import { activateLocale } from "@/i18n/i18n";
import type { AppSettings } from "@/lib/bindings/AppSettings";
import { queryKeys } from "@/lib/query-keys";
import { getSettings } from "@/lib/tauri/commands";

/**
 * Loads persisted settings and applies theme + language before React renders.
 * If the backend cannot answer, the app still starts with defaults (and logs why).
 */
export async function bootstrap(): Promise<void> {
  let settings: AppSettings = DEFAULT_SETTINGS;
  try {
    settings = await getSettings();
  } catch (error) {
    console.error("Could not load settings; starting with defaults.", error);
  }
  applyTheme(settings.theme);
  await activateLocale(settings.locale);
  queryClient.setQueryData(queryKeys.settings, settings);
}
