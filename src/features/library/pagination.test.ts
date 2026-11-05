import { describe, expect, it } from "vitest";

import { clampPage, isPageSize, pageCount, pageRange, pageSlice, pagerItems } from "@/features/library/pagination";

describe("pageCount", () => {
  it("rounds up and never drops below one page", () => {
    expect(pageCount(0, 48)).toBe(1);
    expect(pageCount(48, 48)).toBe(1);
    expect(pageCount(49, 48)).toBe(2);
    expect(pageCount(312, 48)).toBe(7);
  });
});

describe("clampPage", () => {
  it("keeps the page inside the available pages", () => {
    expect(clampPage(0, 100, 24)).toBe(1);
    expect(clampPage(9, 100, 24)).toBe(5);
    expect(clampPage(Number.NaN, 100, 24)).toBe(1);
    expect(clampPage(3, 0, 24)).toBe(1);
  });
});

describe("pageSlice and pageRange", () => {
  const items = Array.from({ length: 55 }, (_, index) => index + 1);

  it("slices a page and clamps an out-of-range one", () => {
    expect(pageSlice(items, 1, 24)).toHaveLength(24);
    expect(pageSlice(items, 3, 24)).toEqual([49, 50, 51, 52, 53, 54, 55]);
    expect(pageSlice(items, 99, 24)).toEqual([49, 50, 51, 52, 53, 54, 55]);
  });

  it("describes the visible range", () => {
    expect(pageRange(55, 1, 24)).toEqual({ from: 1, to: 24 });
    expect(pageRange(55, 3, 24)).toEqual({ from: 49, to: 55 });
    expect(pageRange(0, 1, 24)).toEqual({ from: 0, to: 0 });
  });
});

describe("pagerItems", () => {
  it("lists every page when there are few", () => {
    expect(pagerItems(1, 1)).toEqual([1]);
    expect(pagerItems(2, 4)).toEqual([1, 2, 3, 4]);
  });

  it("collapses long runs into gaps around the current page", () => {
    expect(pagerItems(1, 20)).toEqual([1, 2, "gap", 20]);
    expect(pagerItems(10, 20)).toEqual([1, "gap", 9, 10, 11, "gap", 20]);
    expect(pagerItems(20, 20)).toEqual([1, "gap", 19, 20]);
  });

  it("shows a lone skipped page instead of a gap", () => {
    expect(pagerItems(4, 8)).toEqual([1, 2, 3, 4, 5, "gap", 8]);
    expect(pagerItems(5, 8)).toEqual([1, "gap", 4, 5, 6, 7, 8]);
  });
});

describe("isPageSize", () => {
  it("accepts only the offered sizes", () => {
    expect(isPageSize(96)).toBe(true);
    expect(isPageSize(50)).toBe(false);
    expect(isPageSize("48")).toBe(false);
  });
});
