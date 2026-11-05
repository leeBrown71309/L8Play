import { createHashRouter } from "react-router";

import { ROUTES } from "@/app/routes";
import { AppShell } from "@/components/shell/AppShell";
import { DashboardPage } from "@/features/dashboard/DashboardPage";
import { LibraryPage } from "@/features/library/LibraryPage";

// Hash routing: the Tauri custom protocol serves a single index.html, so deep links must not
// depend on server-side rewrites.
export const router = createHashRouter([
  {
    path: ROUTES.dashboard,
    element: <AppShell />,
    children: [
      { index: true, element: <DashboardPage /> },
      { path: ROUTES.games, element: <LibraryPage /> },
      {
        path: `${ROUTES.games}/:gameId`,
        lazy: async () => ({ Component: (await import("@/features/game-details/GameDetailsPage")).GameDetailsPage }),
      },
      {
        path: ROUTES.saves,
        lazy: async () => ({ Component: (await import("@/features/save-manager/SaveManagerPage")).SaveManagerPage }),
      },
      { path: ROUTES.cloud, lazy: async () => ({ Component: (await import("@/features/cloud/CloudPage")).CloudPage }) },
      {
        path: ROUTES.settings,
        lazy: async () => ({ Component: (await import("@/features/settings/SettingsPage")).SettingsPage }),
      },
    ],
  },
]);

// Every navigation (links, rail buttons, programmatic) cross-fades the page body through the View Transitions
// API; webviews without it simply navigate instantly. The transition group is named in motion.css.
const navigate = router.navigate.bind(router);
router.navigate = ((to: Parameters<typeof navigate>[0], options?: Parameters<typeof navigate>[1]) =>
  navigate(to as never, { viewTransition: true, ...options })) as typeof router.navigate;
