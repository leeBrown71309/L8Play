import { describe, expect, it } from "vitest";

import { cloudRetentionChoices, pcRetentionChoices } from "@/features/settings/retention-options";

describe("retention options", () => {
  it("offers the standard counts for this PC and keeps an unusual stored value selectable", () => {
    expect(pcRetentionChoices(10)).toEqual([5, 10, 15, 20, 30, 50]);
    expect(pcRetentionChoices(7)).toEqual([5, 7, 10, 15, 20, 30, 50]);
  });

  it("starts the Drive list with 'same as this PC' and never offers fewer than this PC keeps", () => {
    expect(cloudRetentionChoices(10, 0)).toEqual([0, 15, 20, 30, 50]);
    expect(cloudRetentionChoices(20, 0)).toEqual([0, 30, 50]);
    expect(cloudRetentionChoices(50, 0)).toEqual([0]);
  });

  it("keeps a stored Drive value that is no longer in the list", () => {
    expect(cloudRetentionChoices(10, 25)).toEqual([0, 15, 20, 25, 30, 50]);
  });
});
