/** How many automatic snapshots can be kept per game (manual, pinned and safety copies never count). */
export const KEEP_CHOICES = [5, 10, 15, 20, 30, 50] as const;

/** The standard choices plus the stored value when it is not one of them (older or hand-edited settings). */
export function withCurrent(choices: readonly number[], current: number): number[] {
  return choices.includes(current) ? [...choices] : [...choices, current].sort((a, b) => a - b);
}

/** Options for "Keep on this PC". */
export function pcRetentionChoices(keepAutomatic: number): number[] {
  return withCurrent(KEEP_CHOICES, keepAutomatic);
}

/**
 * Options for "Keep in Google Drive": `0` (same as this PC) first, then only counts above the PC limit,
 * because Drive never keeps fewer snapshots than this PC.
 */
export function cloudRetentionChoices(keepAutomatic: number, keepCloud: number): number[] {
  const above = KEEP_CHOICES.filter((choice) => choice > keepAutomatic);
  return [0, ...withCurrent(above, keepCloud).filter((choice) => choice > 0)];
}
