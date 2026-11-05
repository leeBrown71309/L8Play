import { useLingui } from "@lingui/react/macro";
import { useEffect, useRef } from "react";
import { useStore } from "zustand";

import { notify } from "@/components/feedback/toast-store";
import { useSettings } from "@/features/settings/use-settings";
import { previewScenario } from "@/features/updates/update-service";
import { updateStore } from "@/features/updates/update-store";
import { isTauri } from "@/lib/tauri/runtime";

const AUTO_CHECK_STARTUP_DELAY_MS = 5000;
const PREVIEW_CHECK_STARTUP_DELAY_MS = 1200;
const AUTO_CHECK_INTERVAL_MS = 6 * 60 * 60 * 1000;
const NOTIFIED_VERSION_KEY = "l8play.updateNotifiedVersion";

/**
 * Automatic checks: once shortly after startup (when the setting is on), then every six hours.
 * Manual checks always work. A toast announces a new version once per version; nothing blocks.
 */
export function useAppUpdateWatcher(): void {
  const { t } = useLingui();
  const settings = useSettings();
  const phase = useStore(updateStore, (state) => state.phase);
  const update = useStore(updateStore, (state) => state.update);
  const started = useRef(false);

  useEffect(() => {
    if (!settings.updatesAutoCheck) return;
    if (!isTauri() && previewScenario() === "idle") return;
    if (started.current) return;
    started.current = true;
    const check = (): void => {
      void updateStore.getState().check();
    };
    const delay = isTauri() ? AUTO_CHECK_STARTUP_DELAY_MS : PREVIEW_CHECK_STARTUP_DELAY_MS;
    const first = window.setTimeout(check, delay);
    const interval = window.setInterval(check, AUTO_CHECK_INTERVAL_MS);
    return () => {
      started.current = false;
      window.clearTimeout(first);
      window.clearInterval(interval);
    };
  }, [settings.updatesAutoCheck]);

  // Browser preview only: `?update=downloading` and `?update=ready` walk through the install flow.
  // It waits for the next "available" state so it does not race the automatic check.
  useEffect(() => {
    if (isTauri()) return;
    const scenario = previewScenario();
    if (scenario !== "downloading" && scenario !== "ready") return;
    let started = false;
    const unsubscribe = updateStore.subscribe((state) => {
      if (started || state.phase !== "available") return;
      started = true;
      unsubscribe();
      void updateStore.getState().install();
    });
    const timer = window.setTimeout(() => {
      if (updateStore.getState().phase === "idle") void updateStore.getState().check();
    }, 100);
    return () => {
      unsubscribe();
      window.clearTimeout(timer);
    };
  }, []);

  useEffect(() => {
    if (phase !== "available" || !update) return;
    try {
      if (window.localStorage.getItem(NOTIFIED_VERSION_KEY) === update.version) return;
      window.localStorage.setItem(NOTIFIED_VERSION_KEY, update.version);
    } catch {
      // A locked-down WebView storage only costs the once-per-version guard.
    }
    notify({
      tone: "neutral",
      title: t`Update available`,
      body: t`L8Play ${update.version} is ready to download. Open Settings to install it.`,
    });
  }, [phase, update, t]);
}
