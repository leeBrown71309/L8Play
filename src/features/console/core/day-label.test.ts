import { describe, expect, it } from "vitest";

import { dayKey, dayLabel } from "@/features/console/core/day-label";

const NOW = new Date(2026, 10, 5, 14, 30);

describe("day labels", () => {
  it("says today and yesterday in words", () => {
    expect(dayLabel(new Date(2026, 10, 5, 1, 0).toISOString(), "en", NOW)).toBe("today");
    expect(dayLabel(new Date(2026, 10, 4, 23, 0).toISOString(), "en", NOW)).toBe("yesterday");
  });

  it("uses a weekday and date for older days", () => {
    expect(dayLabel(new Date(2026, 10, 1, 12, 0).toISOString(), "en", NOW)).toMatch(/Sunday/);
  });

  it("keys days in local time", () => {
    const morning = new Date(2026, 10, 5, 8, 0).toISOString();
    const evening = new Date(2026, 10, 5, 22, 0).toISOString();
    expect(dayKey(morning)).toBe(dayKey(evening));
    expect(dayKey(morning)).not.toBe(dayKey(new Date(2026, 10, 4, 22, 0).toISOString()));
  });
});
