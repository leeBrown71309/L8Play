/** Coverflow geometry: where a cover sits relative to the selected one (offset 0). Pure, so it is easy to test. */

/** Covers further than this from the selection are not drawn. */
export const VISIBLE_RADIUS = 3;
/** Covers are rendered one step beyond the visible radius so they can glide in from nothing. */
export const RENDER_RADIUS = VISIBLE_RADIUS + 1;

export interface CoverTransform {
  /** Horizontal shift in card widths. */
  x: number;
  /** Depth in pixels (negative = further away). */
  z: number;
  /** Y rotation in degrees (negative turns the right edge towards the viewer's left, i.e. faces the centre). */
  rotateY: number;
  scale: number;
  opacity: number;
  /** 0 (full brightness) to 1 (black): drawn as an overlay, which is cheaper than a filter. */
  dim: number;
  zIndex: number;
}

const STEPS = [
  { x: 0, z: 0, rotateY: 0, scale: 1, opacity: 1, dim: 0 },
  { x: 0.78, z: -120, rotateY: 38, scale: 0.82, opacity: 1, dim: 0.28 },
  { x: 1.3, z: -240, rotateY: 42, scale: 0.68, opacity: 0.86, dim: 0.5 },
  { x: 1.7, z: -360, rotateY: 46, scale: 0.56, opacity: 0.55, dim: 0.68 },
] as const;

/** Transform for a cover `offset` positions away from the selection (negative = to the left). */
export function coverTransform(offset: number): CoverTransform {
  const distance = Math.abs(offset);
  const sign = offset < 0 ? -1 : 1;
  if (distance > VISIBLE_RADIUS) {
    const last = STEPS[VISIBLE_RADIUS];
    return {
      x: sign * (last.x + 0.35 * (distance - VISIBLE_RADIUS)),
      z: last.z - 120,
      rotateY: -sign * last.rotateY,
      scale: last.scale - 0.08,
      opacity: 0,
      dim: 0.8,
      zIndex: 0,
    };
  }
  const step = STEPS[distance] ?? STEPS[0];
  return {
    x: sign * step.x,
    z: step.z,
    // Covers on the right turn so their left edge comes forward (negative angle), and the other way round.
    rotateY: step.rotateY === 0 ? 0 : -sign * step.rotateY,
    scale: step.scale,
    opacity: step.opacity,
    dim: step.dim,
    zIndex: 10 - distance,
  };
}

/** Indexes worth rendering around the selection, clamped to the list (no wrap-around at the ends). */
export function visibleIndexes(selected: number, total: number): number[] {
  if (total <= 0) return [];
  const first = Math.max(0, selected - RENDER_RADIUS);
  const last = Math.min(total - 1, selected + RENDER_RADIUS);
  return Array.from({ length: last - first + 1 }, (_, step) => first + step);
}

/** Extra covers kept mounted on each side so moving back and forth near the edge never mounts or unmounts any. */
export const WINDOW_SLACK = 3;

export interface CoverWindow {
  first: number;
  last: number;
}

/**
 * The covers to keep mounted. The previous window is kept for as long as it still covers what must be drawn,
 * so a selection that wobbles between two covers never creates or destroys a DOM node; when it finally leaves
 * the safe zone, a fresh window is centred on it.
 */
export function coverWindow(selected: number, total: number, previous: CoverWindow | null): CoverWindow {
  const need = { first: Math.max(0, selected - RENDER_RADIUS), last: Math.min(total - 1, selected + RENDER_RADIUS) };
  if (previous && previous.first <= need.first && previous.last >= need.last && previous.last < total) return previous;
  return {
    first: Math.max(0, selected - RENDER_RADIUS - WINDOW_SLACK),
    last: Math.min(total - 1, selected + RENDER_RADIUS + WINDOW_SLACK),
  };
}

/** Inline CSS custom properties for one cover, consumed by `console-library.css`. */
export function coverStyleVars(transform: CoverTransform): Record<string, string | number> {
  return {
    "--x": transform.x,
    "--z": `${transform.z}px`,
    "--ry": `${transform.rotateY}deg`,
    "--s": transform.scale,
    "--op": transform.opacity,
    "--dim": transform.dim,
    zIndex: transform.zIndex,
  };
}
