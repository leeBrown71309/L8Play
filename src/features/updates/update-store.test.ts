import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { AvailableUpdate, UpdateBackend } from "@/features/updates/update-service";
import { createUpdateStore } from "@/features/updates/update-store";

const NOW = new Date("2026-11-05T12:00:00Z");

function updateWith(install: AvailableUpdate["install"]): AvailableUpdate {
  return {
    version: "0.1.4",
    date: "2026-11-05T10:00:00Z",
    body: "Release notes",
    install,
  };
}

function backendWith(overrides: Partial<UpdateBackend> = {}): UpdateBackend {
  return {
    check: async () => null,
    relaunch: async () => undefined,
    ...overrides,
  };
}

function createStore(backend: UpdateBackend, version: () => Promise<string> = async () => "0.1.3") {
  return createUpdateStore({ backend, currentVersion: version, now: () => NOW });
}

beforeEach(() => {
  vi.stubGlobal("navigator", { onLine: true });
  vi.spyOn(console, "warn").mockImplementation(() => undefined);
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("update store", () => {
  it("starts idle", () => {
    const store = createStore(backendWith());
    expect(store.getState().phase).toBe("idle");
    expect(store.getState().update).toBeNull();
  });

  it("moves to available and records the manifest", async () => {
    const store = createStore(backendWith({ check: async () => updateWith(async () => undefined) }));
    await store.getState().check();
    const state = store.getState();
    expect(state.phase).toBe("available");
    expect(state.currentVersion).toBe("0.1.3");
    expect(state.update).toEqual({
      version: "0.1.4",
      date: "2026-11-05T10:00:00Z",
      body: "Release notes",
    });
    expect(state.lastCheckedAt).toBe(NOW.toISOString());
  });

  it("moves to upToDate when the endpoint returns nothing", async () => {
    const store = createStore(backendWith());
    await store.getState().check();
    expect(store.getState().phase).toBe("upToDate");
    expect(store.getState().update).toBeNull();
  });

  it("reports an unreachable endpoint", async () => {
    const store = createStore(
      backendWith({
        check: async () => {
          throw new Error("network error: connection refused");
        },
      }),
    );
    await store.getState().check();
    expect(store.getState().phase).toBe("error");
    expect(store.getState().error).toBe("unreachable");
  });

  it("recognizes a signature failure during installation", async () => {
    const store = createStore(
      backendWith({
        check: async () =>
          updateWith(async () => {
            throw new Error("signature verification failed");
          }),
      }),
    );
    await store.getState().check();
    await store.getState().install();
    expect(store.getState().phase).toBe("error");
    expect(store.getState().error).toBe("signature");
  });

  it("streams download progress and restarts after install", async () => {
    let release: (() => void) | undefined;
    const relaunch = vi.fn(async () => undefined);
    const store = createStore(
      backendWith({
        check: async () =>
          updateWith(
            (onProgress) =>
              new Promise<void>((resolve) => {
                onProgress({ downloadedBytes: 40, totalBytes: 100 });
                release = () => {
                  onProgress({ downloadedBytes: 100, totalBytes: 100 });
                  resolve();
                };
              }),
          ),
        relaunch,
      }),
    );
    await store.getState().check();
    const installing = store.getState().install();
    await Promise.resolve();
    expect(store.getState().phase).toBe("downloading");
    expect(store.getState().progress).toBe(40);
    release?.();
    await installing;
    expect(store.getState().phase).toBe("ready");
    expect(store.getState().progress).toBe(100);
    expect(relaunch).toHaveBeenCalledOnce();
  });

  it("keeps the update available when an installation error is dismissed", async () => {
    const store = createStore(
      backendWith({
        check: async () =>
          updateWith(async () => {
            throw new Error("signature verification failed");
          }),
      }),
    );
    await store.getState().check();
    await store.getState().install();
    store.getState().dismissError();
    expect(store.getState().phase).toBe("available");
    expect(store.getState().error).toBeNull();
  });

  it("still checks when the running version cannot be read", async () => {
    const store = createStore(backendWith({ check: async () => updateWith(async () => undefined) }), async () => {
      throw new Error("app info failed");
    });
    await store.getState().check();
    expect(store.getState().phase).toBe("available");
    expect(store.getState().currentVersion).toBe("");
  });
});
