/** Page sizes offered by the library pager. */
export const PAGE_SIZES = [24, 48, 96, 200] as const;
export type PageSize = (typeof PAGE_SIZES)[number];
export const DEFAULT_PAGE_SIZE: PageSize = 48;

export function isPageSize(value: unknown): value is PageSize {
  return PAGE_SIZES.some((size) => size === value);
}

/** Number of pages for `total` items; an empty list still has one (empty) page. */
export function pageCount(total: number, pageSize: number): number {
  return Math.max(1, Math.ceil(total / Math.max(1, pageSize)));
}

/** Brings a 1-based page number back inside 1…pageCount. */
export function clampPage(page: number, total: number, pageSize: number): number {
  return Math.min(Math.max(1, Math.trunc(page) || 1), pageCount(total, pageSize));
}

/** The items of a 1-based page. */
export function pageSlice<T>(items: readonly T[], page: number, pageSize: number): T[] {
  const current = clampPage(page, items.length, pageSize);
  return items.slice((current - 1) * pageSize, current * pageSize);
}

/** 1-based inclusive range of the items on a page, for "Showing 1–48 of 312"; both 0 when empty. */
export function pageRange(total: number, page: number, pageSize: number): { from: number; to: number } {
  if (total === 0) return { from: 0, to: 0 };
  const current = clampPage(page, total, pageSize);
  return { from: (current - 1) * pageSize + 1, to: Math.min(total, current * pageSize) };
}

export type PagerItem = number | "gap";

/**
 * Page buttons to show: the first and last page, the current one with `siblings` on each side, and
 * a gap marker where pages are skipped (a gap that would hide a single page shows that page).
 */
export function pagerItems(page: number, pages: number, siblings = 1): PagerItem[] {
  const current = Math.min(Math.max(1, page), pages);
  const wanted = new Set<number>([1, pages]);
  for (let offset = -siblings; offset <= siblings; offset += 1) {
    const candidate = current + offset;
    if (candidate >= 1 && candidate <= pages) wanted.add(candidate);
  }
  const sorted = [...wanted].sort((a, b) => a - b);
  const items: PagerItem[] = [];
  sorted.forEach((value, index) => {
    const previous = sorted[index - 1];
    if (previous !== undefined && value - previous > 1) {
      items.push(value - previous === 2 ? previous + 1 : "gap");
    }
    items.push(value);
  });
  return items;
}
