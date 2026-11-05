import { describe, expect, it } from "vitest";

import { dayPeriod } from "@/components/shell/Greeting";

const at = (hour: number) => new Date(2026, 10, 4, hour, 30);

describe("dayPeriod", () => {
  it("greets the morning from 5:00 to 11:59", () => {
    expect(dayPeriod(at(5))).toBe("morning");
    expect(dayPeriod(at(11))).toBe("morning");
  });

  it("greets the afternoon from 12:00 to 17:59", () => {
    expect(dayPeriod(at(12))).toBe("afternoon");
    expect(dayPeriod(at(17))).toBe("afternoon");
  });

  it("greets the evening otherwise, including late night", () => {
    expect(dayPeriod(at(18))).toBe("evening");
    expect(dayPeriod(at(2))).toBe("evening");
  });
});
