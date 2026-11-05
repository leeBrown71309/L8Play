import { create, type StoreApi, type UseBoundStore } from "zustand";

import { classifyUpdateError, type UpdateErrorCode } from "@/features/updates/update-format";
import {
  createUpdateBackend,
  type AvailableUpdate,
  type DownloadProgress,
  type UpdateBackend,
} from "@/features/updates/update-service";
import { getAppInfo } from "@/lib/tauri/commands";

/** What the Settings section and the navigation rail need to show. */
export type UpdatePhase = "idle" | "checking" | "available" | "downloading" | "ready" | "error" | "upToDate";

/** Plain update data; the install handle stays inside the store closure. */
export interface UpdateInfo {
  version: string;
  date?: string | undefined;
  body?: string | undefined;
}

export interface UpdateStoreState {
  phase: UpdatePhase;
  /** Version of the running app, filled on the first check. */
  currentVersion: string;
  update: UpdateInfo | null;
  /** 0–100, or `null` while the total size is unknown. */
  progress: number | null;
  downloadedBytes: number;
  totalBytes: number | null;
  error: UpdateErrorCode | null;
  lastCheckedAt: string | null;
  check: () => Promise<void>;
  install: () => Promise<void>;
  relaunch: () => Promise<void>;
  dismissError: () => void;
}

export type UpdateStore = UseBoundStore<StoreApi<UpdateStoreState>>;

interface CreateUpdateStoreOptions {
  backend: UpdateBackend;
  /** Reads the running version (the AppInfo command in the app, a stub in tests). */
  currentVersion: () => Promise<string>;
  /** Injectable clock so tests can assert the last-check timestamp. */
  now?: () => Date;
}

function toInfo(update: AvailableUpdate): UpdateInfo {
  return { version: update.version, date: update.date, body: update.body };
}

/**
 * Update state machine: `idle → checking → available|upToDate|error`,
 * `available → downloading → ready|error`. Installing never starts on its own.
 */
export function createUpdateStore({
  backend,
  currentVersion,
  now = () => new Date(),
}: CreateUpdateStoreOptions): UpdateStore {
  let release: AvailableUpdate | null = null;

  return create<UpdateStoreState>((set, get) => ({
    phase: "idle",
    currentVersion: "",
    update: null,
    progress: null,
    downloadedBytes: 0,
    totalBytes: null,
    error: null,
    lastCheckedAt: null,

    async check() {
      if (get().phase === "checking" || get().phase === "downloading") return;
      set({ phase: "checking", error: null });
      const version = await currentVersion().catch((error: unknown) => {
        console.warn("Update check: the running version could not be read.", error);
        return get().currentVersion;
      });
      try {
        const found = await backend.check();
        if (!found) {
          release = null;
          set({
            phase: "upToDate",
            currentVersion: version,
            update: null,
            progress: null,
            downloadedBytes: 0,
            totalBytes: null,
            lastCheckedAt: now().toISOString(),
          });
          return;
        }
        release = found;
        set({
          phase: "available",
          currentVersion: version,
          update: toInfo(found),
          progress: null,
          downloadedBytes: 0,
          totalBytes: null,
          lastCheckedAt: now().toISOString(),
        });
      } catch (error) {
        release = null;
        console.warn("Update check failed.", error);
        set({ phase: "error", error: classifyUpdateError(error), update: null, progress: null });
      }
    },

    async install() {
      if (get().phase !== "available" || !release) return;
      const target = release;
      const onProgress = (progress: DownloadProgress): void => {
        const percent =
          progress.totalBytes !== null && progress.totalBytes > 0
            ? Math.min(100, Math.round((progress.downloadedBytes / progress.totalBytes) * 100))
            : null;
        set({
          progress: percent,
          downloadedBytes: progress.downloadedBytes,
          totalBytes: progress.totalBytes,
        });
      };
      set({ phase: "downloading", progress: 0, downloadedBytes: 0, totalBytes: null, error: null });
      try {
        await target.install(onProgress);
        set({ phase: "ready", progress: 100 });
        await get().relaunch();
      } catch (error) {
        console.warn("Update installation failed.", error);
        set({ phase: "error", error: classifyUpdateError(error), progress: null });
      }
    },

    async relaunch() {
      await backend.relaunch();
    },

    dismissError() {
      set({ phase: release ? "available" : "idle", error: null });
    },
  }));
}

/** The app's single store; tests build isolated stores with {@link createUpdateStore}. */
export const updateStore = createUpdateStore({
  backend: createUpdateBackend(),
  currentVersion: async () => (await getAppInfo()).version,
});
