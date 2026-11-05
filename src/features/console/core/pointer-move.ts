let last = { x: -1000, y: -1000 };

/**
 * True only when the mouse really moved since the last time this returned true. Rows scrolling under a resting
 * cursor fire `pointerenter`, which must not steal the focus from the pad; real movement can.
 */
export function pointerMoved(event: { clientX: number; clientY: number }): boolean {
  const moved = Math.abs(event.clientX - last.x) + Math.abs(event.clientY - last.y) > 3;
  if (moved) last = { x: event.clientX, y: event.clientY };
  return moved;
}
