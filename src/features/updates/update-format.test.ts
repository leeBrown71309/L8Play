import { afterEach, describe, expect, it, vi } from "vitest";

import { classifyUpdateError, cleanReleaseNotes, formatUpdateDate } from "@/features/updates/update-format";

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("classifyUpdateError", () => {
  it("reports offline first when the browser says so", () => {
    vi.stubGlobal("navigator", { onLine: false });
    expect(classifyUpdateError(new Error("network error"))).toBe("offline");
  });

  it("recognizes a signature failure", () => {
    vi.stubGlobal("navigator", { onLine: true });
    expect(classifyUpdateError(new Error("signature verification failed"))).toBe("signature");
  });

  it("recognizes an unreachable endpoint", () => {
    vi.stubGlobal("navigator", { onLine: true });
    expect(classifyUpdateError(new Error("error sending request: connection refused"))).toBe("unreachable");
  });

  it("falls back to unknown", () => {
    vi.stubGlobal("navigator", { onLine: true });
    expect(classifyUpdateError(new Error("unexpected state"))).toBe("unknown");
  });
});

describe("formatUpdateDate", () => {
  it("formats a valid RFC 3339 date for the locale", () => {
    expect(formatUpdateDate("2026-11-05T10:00:00Z", "en")).toContain("2026");
    expect(formatUpdateDate("2026-11-05T10:00:00Z", "fr")).toContain("2026");
  });

  it("returns null when the manifest has no usable date", () => {
    expect(formatUpdateDate(undefined, "en")).toBeNull();
    expect(formatUpdateDate("not a date", "en")).toBeNull();
  });
});

describe("cleanReleaseNotes", () => {
  it("returns an empty string without notes", () => {
    expect(cleanReleaseNotes(undefined)).toBe("");
  });

  it("keeps plain text and strips tags and control characters", () => {
    expect(cleanReleaseNotes("<p>Hello</p>\n\n\n\n- Fix</p>")).toBe("Hello\n\n- Fix");
    expect(cleanReleaseNotes("bad\u0000text")).toBe("badtext");
  });

  it("normalizes line endings and trims", () => {
    expect(cleanReleaseNotes("  a\r\n\r\n\r\nb  ")).toBe("a\n\nb");
  });
});
