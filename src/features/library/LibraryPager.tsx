import { useLingui } from "@lingui/react/macro";

import { Dropdown } from "@/components/ds/form-fields";
import { IconButton } from "@/components/ui/icon-button";
import { PAGE_SIZES, pageCount, pageRange, pagerItems, type PageSize } from "@/features/library/pagination";
import { cn } from "@/lib/cn";

interface LibraryPagerProps {
  total: number;
  page: number;
  pageSize: PageSize;
  onPage: (page: number) => void;
  onPageSize: (pageSize: PageSize) => void;
}

/** Pager under the library: range text, previous / numbered / next pages and the page size. */
export function LibraryPager({ total, page, pageSize, onPage, onPageSize }: LibraryPagerProps) {
  const { t } = useLingui();
  const pages = pageCount(total, pageSize);
  const { from, to } = pageRange(total, page, pageSize);

  return (
    <nav aria-label={t`Pages`} className="mt-7 grid grid-cols-[1fr_auto_1fr] items-center gap-4 pb-2">
      <p className="m-0 text-[13px] text-ink-muted" aria-live="polite">
        {t`Showing ${from}–${to} of ${total}`}
      </p>
      <div className="flex items-center gap-1.5">
        <IconButton
          icon="chevronLeft"
          label={t`Previous page`}
          variant="secondary"
          size="sm"
          disabled={page <= 1}
          tooltipSide="top"
          onClick={() => onPage(page - 1)}
        />
        {pagerItems(page, pages).map((item, index) =>
          item === "gap" ? (
            <span key={`gap-${index}`} aria-hidden="true" className="px-1 text-ink-faint">
              …
            </span>
          ) : (
            <button
              key={item}
              type="button"
              aria-label={t`Page ${item}`}
              aria-current={item === page ? "page" : undefined}
              className={cn("l8-chip justify-center px-0", item === page && "is-on")}
              style={{ minWidth: 34 }}
              onClick={() => onPage(item)}
            >
              {item}
            </button>
          ),
        )}
        <IconButton
          icon="chevronRight"
          label={t`Next page`}
          variant="secondary"
          size="sm"
          disabled={page >= pages}
          tooltipSide="top"
          onClick={() => onPage(page + 1)}
        />
      </div>
      {/* The pager sits at the bottom of the page: the menu opens upwards and ends at the right edge. */}
      <div className="flex justify-end [&_.l8-dd-menu]:top-auto [&_.l8-dd-menu]:right-0 [&_.l8-dd-menu]:bottom-[calc(100%+8px)] [&_.l8-dd-menu]:left-auto [&_.l8-dd-menu]:min-w-[120px]">
        <Dropdown<PageSize>
          prefix={t`Per page`}
          label={t`Games per page`}
          value={pageSize}
          options={PAGE_SIZES.map((size) => ({ id: size, label: String(size) }))}
          onChange={onPageSize}
        />
      </div>
    </nav>
  );
}
