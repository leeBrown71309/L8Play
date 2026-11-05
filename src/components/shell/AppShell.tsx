import { Outlet, useLocation } from "react-router";

import { ROUTES } from "@/app/routes";
import { useBackendEvents } from "@/app/use-backend-events";
import { Toaster } from "@/components/feedback/Toaster";
import { AmbientBackdrop } from "@/components/shell/AmbientBackdrop";
import { CaptionBar } from "@/components/shell/CaptionBar";
import { Greeting } from "@/components/shell/Greeting";
import { NavRail } from "@/components/shell/NavRail";
import { QuickRail } from "@/components/shell/QuickRail";
import { TopBar } from "@/components/shell/TopBar";
import { useSettings, useUpdateSettings } from "@/features/settings/use-settings";
import { AddGameDialogs } from "@/features/library/AddGameDialogs";
import { OnboardingDialog } from "@/features/onboarding/OnboardingDialog";
import { cn } from "@/lib/cn";

import "@/components/shell/shell-motion.css";

/**
 * The application window: navigation rail, caption strip, top bar, page, quick-launch rail.
 * The grid and every part come from the design system's window (`l8-window`), filling the OS window.
 */
export function AppShell() {
  const { navExpanded } = useSettings();
  const updateSettings = useUpdateSettings();
  useBackendEvents();
  const { pathname } = useLocation();
  return (
    <div className={cn("l8-window h-full w-full rounded-none", navExpanded && "is-nav-expanded")}>
      <NavRail expanded={navExpanded} onToggle={() => updateSettings.mutate({ navExpanded: !navExpanded })} />
      <CaptionBar />
      <main className="l8-main">
        <AmbientBackdrop />
        <TopBar heading={pathname === ROUTES.dashboard ? <Greeting /> : undefined} />
        <div className="l8-main-body">
          <div className="l8-page min-h-full">
            <Outlet />
          </div>
        </div>
      </main>
      <QuickRail />
      <AddGameDialogs />
      <OnboardingDialog />
      <Toaster />
    </div>
  );
}
