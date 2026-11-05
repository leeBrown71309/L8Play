import { beforeEach, describe, expect, it } from "vitest";

import { DEFAULT_SETTINGS } from "@/features/settings/defaults";
import { previewBackend } from "@/lib/tauri/preview-backend";

/** The design preview starts with the welcome tour already done, so it never covers a page. */
const PREVIEW_SETTINGS = { ...DEFAULT_SETTINGS, onboardingDone: true };

describe("previewBackend settings", () => {
  beforeEach(() => window.localStorage.clear());

  it("returns defaults when nothing is stored", async () => {
    expect(await previewBackend.handle("get_settings")).toEqual(PREVIEW_SETTINGS);
  });

  it("applies a partial patch and persists it", async () => {
    const updated = await previewBackend.handle("update_settings", { patch: { locale: "fr" } });
    expect(updated).toEqual({ ...PREVIEW_SETTINGS, locale: "fr" });
    expect(await previewBackend.handle("get_settings")).toEqual(updated);
  });

  it("ignores corrupt stored data", async () => {
    window.localStorage.setItem("l8play.preview.settings", "{not json");
    expect(await previewBackend.handle("get_settings")).toEqual(PREVIEW_SETTINGS);
  });
});
