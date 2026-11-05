import { describe, expect, it } from "vitest";

import { formatBytes, formatPlaytime, formatRelative } from "@/lib/format";

describe("formatPlaytime", () => {
  it("uses minutes under an hour and hours above", () => {
    expect(formatPlaytime(0)).toBe("—");
    expect(formatPlaytime(45 * 60)).toBe("45 min");
    expect(formatPlaytime(64 * 3600)).toBe("64 h");
  });
});

describe("formatBytes", () => {
  it("picks a readable unit", () => {
    expect(formatBytes(null, "en")).toBe("—");
    expect(formatBytes(25.1 * 1024 ** 3, "en")).toBe("25.1 GB");
    expect(formatBytes(122 * 1024 ** 3, "en")).toBe("122 GB");
    expect(formatBytes(25.1 * 1024 ** 3, "fr")).toBe("25,1 GB");
  });
});

describe("formatRelative", () => {
  const now = new Date("2026-11-04T21:40:00Z");
  it("formats recent and older dates", () => {
    expect(formatRelative("2026-11-04T21:12:00Z", "en", now)).toBe("28 minutes ago");
    expect(formatRelative("2026-11-03T20:00:00Z", "en", now)).toBe("yesterday");
    expect(formatRelative("2026-11-01T21:40:00Z", "fr", now)).toBe("il y a 3 jours");
  });
});
