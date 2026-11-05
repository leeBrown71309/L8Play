import { QueryClient } from "@tanstack/react-query";

/**
 * Commands answer from local SQLite, so data is only stale when the backend says so (through
 * events, from phase 2 on). No refetch on focus: a desktop window regains focus constantly.
 */
export const queryClient = new QueryClient({
  defaultOptions: {
    queries: { staleTime: Infinity, refetchOnWindowFocus: false, retry: 1 },
  },
});
