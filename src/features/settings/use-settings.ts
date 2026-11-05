import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { applyTheme } from "@/features/settings/apply-theme";
import { DEFAULT_SETTINGS } from "@/features/settings/defaults";
import { activateLocale } from "@/i18n/i18n";
import type { AppSettings } from "@/lib/bindings/AppSettings";
import type { SettingsPatch } from "@/lib/bindings/SettingsPatch";
import { queryKeys } from "@/lib/query-keys";
import { getSettings, updateSettings } from "@/lib/tauri/commands";

export function useSettings(): AppSettings {
  const { data } = useQuery({ queryKey: queryKeys.settings, queryFn: getSettings });
  return data ?? DEFAULT_SETTINGS;
}

/**
 * Persists a settings change. Theme and language apply immediately (optimistic) and roll back
 * if the backend refuses the write.
 */
export function useUpdateSettings() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (patch: SettingsPatch) => updateSettings(patch),
    onMutate: async (patch) => {
      await client.cancelQueries({ queryKey: queryKeys.settings });
      const previous = client.getQueryData<AppSettings>(queryKeys.settings) ?? DEFAULT_SETTINGS;
      const next: AppSettings = { ...previous, ...stripUndefined(patch) };
      client.setQueryData(queryKeys.settings, next);
      await applyVisibleEffects(next);
      return { previous };
    },
    onError: async (error, _patch, context) => {
      console.error("Settings update failed; reverting.", error);
      if (!context) return;
      client.setQueryData(queryKeys.settings, context.previous);
      await applyVisibleEffects(context.previous);
    },
    onSuccess: (saved) => client.setQueryData(queryKeys.settings, saved),
  });
}

async function applyVisibleEffects(settings: AppSettings): Promise<void> {
  applyTheme(settings.theme);
  await activateLocale(settings.locale);
}

function stripUndefined(patch: SettingsPatch): Partial<AppSettings> {
  return Object.fromEntries(Object.entries(patch).filter(([, value]) => value !== undefined)) as Partial<AppSettings>;
}
