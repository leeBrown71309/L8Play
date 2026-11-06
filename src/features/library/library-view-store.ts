import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";

import { DEFAULT_PAGE_SIZE, isPageSize, type PageSize } from "@/features/library/pagination";
import type { Platform } from "@/lib/bindings/Platform";

export type LibraryView = "grid" | "list";
export type LibrarySort = "recent" | "title" | "playtime" | "size" | "added";
/**
 * Which games to list: everything, only those installed on this PC, those that are not (but could be),
 * or those that exist only as saves imported from an older backup.
 */
export type InstallFilter = "all" | "installed" | "not-installed" | "saves-only";

export interface LibraryFilters {
  search: string;
  platform: Platform | "all";
  install: InstallFilter;
  favoritesOnly: boolean;
  sort: LibrarySort;
}

interface LibraryViewState extends LibraryFilters {
  view: LibraryView;
  /** 1-based page of the filtered list. */
  page: number;
  pageSize: PageSize;
  setView: (view: LibraryView) => void;
  /** Any filter or sort change goes back to the first page. */
  setFilters: (patch: Partial<LibraryFilters>) => void;
  setPage: (page: number) => void;
  setPageSize: (pageSize: PageSize) => void;
}

/** Storage that never throws (private windows, blocked site data): the view then just is not remembered. */
const safeStorage = createJSONStorage<Pick<LibraryViewState, "view" | "pageSize" | "install">>(() => ({
  getItem: (key) => {
    try {
      return window.localStorage.getItem(key);
    } catch {
      return null;
    }
  },
  setItem: (key, value) => {
    try {
      window.localStorage.setItem(key, value);
    } catch {
      /* not remembered */
    }
  },
  removeItem: (key) => {
    try {
      window.localStorage.removeItem(key);
    } catch {
      /* nothing to remove */
    }
  },
}));

/**
 * State of the library page. The layout, the page size and the install filter are remembered
 * between launches; search, platform, favourites and sort start fresh (server data lives in TanStack Query).
 */
export const useLibraryView = create<LibraryViewState>()(
  persist(
    (set) => ({
      view: "grid",
      search: "",
      platform: "all",
      install: "all",
      favoritesOnly: false,
      sort: "recent",
      page: 1,
      pageSize: DEFAULT_PAGE_SIZE,
      setView: (view) => set({ view }),
      setFilters: (patch) => set({ ...patch, page: 1 }),
      setPage: (page) => set({ page }),
      setPageSize: (pageSize) => set({ pageSize, page: 1 }),
    }),
    {
      name: "l8play.library-view",
      version: 1,
      storage: safeStorage,
      partialize: (state) => ({ view: state.view, pageSize: state.pageSize, install: state.install }),
      // Stored values are untrusted input: keep only what is still valid.
      merge: (stored, current) => {
        const saved = (stored ?? {}) as Partial<Pick<LibraryViewState, "view" | "pageSize" | "install">>;
        return {
          ...current,
          view: saved.view === "list" ? "list" : "grid",
          pageSize: isPageSize(saved.pageSize) ? saved.pageSize : current.pageSize,
          install: saved.install === "installed" || saved.install === "not-installed" || saved.install === "saves-only"
              ? saved.install
              : current.install,
        };
      },
    },
  ),
);
