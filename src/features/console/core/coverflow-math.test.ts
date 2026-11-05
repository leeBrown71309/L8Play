import { describe, expect, it } from "vitest";

import {
  coverStyleVars,
  coverWindow,
  coverTransform,
  RENDER_RADIUS,
  VISIBLE_RADIUS,
  visibleIndexes,
} from "@/features/console/core/coverflow-math";

describe("coverflow math", () => {
  it("keeps the selected cover centred, full size and in front", () => {
    expect(coverTransform(0)).toMatchObject({ x: 0, z: 0, rotateY: 0, scale: 1, opacity: 1, dim: 0, zIndex: 10 });
  });

  it("mirrors left and right neighbours", () => {
    const right = coverTransform(2);
    const left = coverTransform(-2);
    expect(left.x).toBe(-right.x);
    expect(left.rotateY).toBe(-right.rotateY);
    expect(right.rotateY).toBeLessThan(0);
    expect(left.zIndex).toBe(right.zIndex);
  });

  it("scales, fades and dims further covers more, from offsets -3 to 3", () => {
    for (let offset = 1; offset <= VISIBLE_RADIUS; offset += 1) {
      for (const sign of [1, -1]) {
        const nearer = coverTransform(sign * (offset - 1));
        const further = coverTransform(sign * offset);
        expect(further.scale).toBeLessThan(nearer.scale);
        expect(further.opacity).toBeLessThanOrEqual(nearer.opacity);
        expect(further.dim).toBeGreaterThan(nearer.dim);
        expect(further.z).toBeLessThan(nearer.z);
        expect(further.zIndex).toBeLessThan(nearer.zIndex);
        expect(Math.abs(further.x)).toBeGreaterThan(Math.abs(nearer.x));
      }
    }
  });

  it("hides covers beyond the visible radius", () => {
    expect(coverTransform(VISIBLE_RADIUS + 1).opacity).toBe(0);
    expect(coverTransform(-(VISIBLE_RADIUS + 5)).opacity).toBe(0);
  });

  it("renders a window around the selection clamped to the list (no wrap)", () => {
    expect(visibleIndexes(0, 3)).toEqual([0, 1, 2]);
    expect(visibleIndexes(50, 100)).toHaveLength(RENDER_RADIUS * 2 + 1);
    expect(visibleIndexes(99, 100).at(-1)).toBe(99);
    expect(visibleIndexes(0, 0)).toEqual([]);
  });

  it("exposes CSS custom properties", () => {
    expect(coverStyleVars(coverTransform(1))).toMatchObject({ "--s": 0.82, "--ry": "-38deg", zIndex: 9 });
  });

  it("keeps the same mounted window while the selection wobbles near an edge", () => {
    const first = coverWindow(10, 100, null);
    expect(first).toEqual({ first: 3, last: 17 });
    for (const selected of [11, 10, 11, 12, 11, 10, 9, 10, 11, 12, 13]) {
      expect(coverWindow(selected, 100, first)).toBe(first);
    }
  });

  it("recentres the window once the selection leaves the safe zone", () => {
    const first = coverWindow(10, 100, null);
    const next = coverWindow(14, 100, first);
    expect(next).not.toBe(first);
    expect(next).toEqual({ first: 7, last: 21 });
    expect(coverWindow(0, 100, null)).toEqual({ first: 0, last: 7 });
    expect(coverWindow(99, 100, null).last).toBe(99);
  });

  it("drops a stale window when the list shrinks", () => {
    const wide = coverWindow(50, 100, null);
    expect(coverWindow(2, 5, wide).last).toBe(4);
  });
});
