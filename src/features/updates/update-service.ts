import { relaunch as relaunchApp } from "@tauri-apps/plugin-process";
import { check as checkForUpdate, type DownloadEvent } from "@tauri-apps/plugin-updater";

import { isTauri } from "@/lib/tauri/runtime";

/** Bytes received so far, and the total when the server announces it. */
export interface DownloadProgress {
  downloadedBytes: number;
  totalBytes: number | null;
}

/**
 * An update announced by the configured endpoint. `install` is the only handle on the plugin's
 * native resource, so the store keeps this object private and exposes plain data to the UI.
 */
export interface AvailableUpdate {
  version: string;
  date?: string | undefined;
  body?: string | undefined;
  install: (onProgress: (progress: DownloadProgress) => void) => Promise<void>;
}

/** The small surface the update store depends on; tests provide their own implementation. */
export interface UpdateBackend {
  check: () => Promise<AvailableUpdate | null>;
  relaunch: () => Promise<void>;
}

/** Keeps the resource alive between `check` and `install`, and reports download progress. */
function toAvailableUpdate(update: Awaited<ReturnType<typeof checkForUpdate>>): AvailableUpdate | null {
  if (!update) return null;
  return {
    version: update.version,
    date: update.date,
    body: update.body,
    async install(onProgress) {
      let downloadedBytes = 0;
      let totalBytes: number | null = null;
      const report = (event: DownloadEvent): void => {
        switch (event.event) {
          case "Started":
            totalBytes = event.data.contentLength ?? null;
            onProgress({ downloadedBytes, totalBytes });
            break;
          case "Progress":
            downloadedBytes += event.data.chunkLength;
            onProgress({ downloadedBytes, totalBytes });
            break;
          case "Finished":
            onProgress({ downloadedBytes: totalBytes ?? downloadedBytes, totalBytes });
            break;
        }
      };
      // On Windows the plugin exits the app after launching the installer; `relaunch` is only
      // reached on the other platforms and is harmless when it is not.
      await update.downloadAndInstall(report, { restartAfterInstall: true });
    },
  };
}

/** Real backend: the configured GitHub Releases endpoint and the process plugin. */
function createTauriBackend(): UpdateBackend {
  return {
    async check() {
      return toAvailableUpdate(await checkForUpdate());
    },
    async relaunch() {
      await relaunchApp();
    },
  };
}

/** States the browser preview can force with `?update=<scenario>` (or `#/settings?update=...`). */
export type UpdatePreviewScenario = "idle" | "checking" | "available" | "downloading" | "ready" | "error" | "uptodate";

const PREVIEW_SCENARIOS: readonly UpdatePreviewScenario[] = [
  "idle",
  "checking",
  "available",
  "downloading",
  "ready",
  "error",
  "uptodate",
];

function isPreviewScenario(value: string | null): value is UpdatePreviewScenario {
  return value !== null && PREVIEW_SCENARIOS.includes(value as UpdatePreviewScenario);
}

function previewScenarioFromUrl(): string | null {
  if (typeof window === "undefined") return null;
  const direct = new URLSearchParams(window.location.search).get("update");
  if (direct) return direct;
  const hash = window.location.hash;
  const queryIndex = hash.indexOf("?");
  if (queryIndex === -1) return null;
  return new URLSearchParams(hash.slice(queryIndex + 1)).get("update");
}

/** The scenario requested by the URL, or `available` so the section is easy to review. */
export function previewScenario(): UpdatePreviewScenario {
  const requested = previewScenarioFromUrl();
  return isPreviewScenario(requested) ? requested : "available";
}

const PREVIEW_NOTES = [
  "Faster library scan on large Steam libraries.",
  "Fixed restoring a snapshot on folders with accented characters.",
  "Updated French translations and keyboard hints.",
].join("\n\n");

const PREVIEW_UPDATE: AvailableUpdate = {
  version: "0.1.4",
  date: "2026-11-05T10:00:00Z",
  body: PREVIEW_NOTES,
  install(onProgress) {
    return new Promise((resolve) => {
      const scenario = previewScenario();
      const totalBytes = 18_400_000;
      // `downloading` stays slow enough to review the progress bar; `ready` jumps to the end.
      const steps = scenario === "downloading" ? 80 : scenario === "ready" ? 8 : 40;
      const interval = scenario === "downloading" ? 250 : scenario === "ready" ? 50 : 90;
      let downloadedBytes = 0;
      const timer = window.setInterval(() => {
        downloadedBytes = Math.min(totalBytes, downloadedBytes + Math.round(totalBytes / steps));
        onProgress({ downloadedBytes, totalBytes });
        if (downloadedBytes >= totalBytes) {
          window.clearInterval(timer);
          resolve();
        }
      }, interval);
    });
  },
};

function delay(milliseconds: number): Promise<void> {
  return new Promise((resolve) => window.setTimeout(resolve, milliseconds));
}

/** Browser preview: mirrors the real API so the Settings section can be reviewed offline. */
function createPreviewBackend(): UpdateBackend {
  return {
    async check() {
      await delay(650);
      const scenario = previewScenario();
      if (scenario === "checking") return new Promise<never>(() => undefined);
      if (scenario === "uptodate") return null;
      if (scenario === "error") throw new Error("network error: GitHub could not be reached");
      return PREVIEW_UPDATE;
    },
    async relaunch() {
      console.info("Preview: L8Play would restart to finish the update.");
    },
  };
}

/** The desktop backend inside the WebView, the simulated one in a plain browser. */
export function createUpdateBackend(): UpdateBackend {
  return isTauri() ? createTauriBackend() : createPreviewBackend();
}
