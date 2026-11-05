import { describe, expect, it } from "vitest";

import { cycleOption, stepVolume } from "@/features/console/views/settings-rows";

const OPTIONS = [
  { value: "a", label: "A" },
  { value: "b", label: "B" },
  { value: "c", label: "C" },
];

describe("settings rows helpers", () => {
  it("cycles through the options and wraps at the ends", () => {
    expect(cycleOption(OPTIONS, "a", 1)?.value).toBe("b");
    expect(cycleOption(OPTIONS, "c", 1)?.value).toBe("a");
    expect(cycleOption(OPTIONS, "a", -1)?.value).toBe("c");
    expect(cycleOption([], "a", 1)).toBeUndefined();
  });

  it("falls back to the first option for an unknown current value", () => {
    expect(cycleOption(OPTIONS, "zzz", 1)?.value).toBe("b");
  });

  it("steps the volume by a tenth and clamps it", () => {
    expect(stepVolume(0.6, 1)).toBe(0.7);
    expect(stepVolume(0.3, -1)).toBe(0.2);
    expect(stepVolume(1, 1)).toBe(1);
    expect(stepVolume(0, -1)).toBe(0);
  });
});
