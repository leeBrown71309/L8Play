import { useRef, useState } from "react";

import { ConsoleBackdrop } from "@/features/console/components/ConsoleBackdrop";
import { ConsoleToasts } from "@/features/console/components/ConsoleToasts";
import { ConsoleTopBar } from "@/features/console/components/ConsoleTopBar";
import { HintBar } from "@/features/console/components/HintBar";
import { ModalLayer } from "@/features/console/components/modals/ModalLayer";
import { useConsoleNav } from "@/features/console/core/nav-store";
import { requestConsoleExit } from "@/features/console/core/use-request-exit";
import { usePresence } from "@/features/console/core/use-presence";
import { dispatchToViews, ViewInputProvider, type ViewRegistry } from "@/features/console/core/view-input";
import { useConsoleInput } from "@/features/console/input";
import { consoleSound } from "@/features/console/sound";
import { CloudView } from "@/features/console/views/CloudView";
import { GamePanel } from "@/features/console/views/GamePanel";
import { LibraryView } from "@/features/console/views/LibraryView";
import { SavesView } from "@/features/console/views/SavesView";
import { SettingsView } from "@/features/console/views/SettingsView";
import { useGames } from "@/features/library/use-library";
import { cn } from "@/lib/cn";

const PANEL_EXIT_MS = 360;

/** What the buttons do on every screen: switch sections, open the menu or the letter picker, go back. */
function useRootInput(registry: ViewRegistry) {
  useConsoleInput((action, source) => {
    const nav = useConsoleNav.getState();
    switch (action) {
      case "prevTab":
      case "nextTab": {
        if (nav.gameId !== null) return;
        const before = nav.section;
        nav.dispatch({ type: "switchSection", delta: action === "nextTab" ? 1 : -1 });
        if (useConsoleNav.getState().section !== before) consoleSound.play("tab");
        return;
      }
      case "menu":
        consoleSound.play("open");
        return nav.dispatch({ type: "pushModal", modal: { kind: "quick-menu" } });
      case "search":
        if (nav.section === "library" && nav.gameId === null) {
          consoleSound.play("open");
          nav.dispatch({ type: "pushModal", modal: { kind: "letters" } });
        }
        return;
      case "back":
        if (nav.gameId !== null) {
          consoleSound.play("back");
          nav.dispatch({ type: "closeGame" });
        } else if (nav.section === "library" && (nav.focus["library.zone"] ?? 0) === 1) {
          consoleSound.play("back");
          nav.dispatch({ type: "setFocus", zone: "library.zone", index: 0, count: 2 });
        } else if (nav.section !== "library") {
          consoleSound.play("back");
          nav.dispatch({ type: "gotoSection", section: "library" });
        } else {
          consoleSound.play("open");
          requestConsoleExit();
        }
        return;
      default:
        dispatchToViews(registry, action, source);
    }
  });
}

/** The console interface: backdrop, top bar, the current section, the game panel, dialogs, hints and toasts. */
export function ConsoleApp() {
  const registry = useRef<ViewRegistry["current"]>([]) as ViewRegistry;
  const section = useConsoleNav((state) => state.section);
  const gameId = useConsoleNav((state) => state.gameId);
  const games = useGames().data;
  // The panel keeps showing the last game while it slides out, after the open game has been cleared.
  const [shownGameId, setShownGameId] = useState(gameId);
  if (gameId !== null && gameId !== shownGameId) setShownGameId(gameId);
  const panel = usePresence(gameId !== null, PANEL_EXIT_MS);
  const panelGame = games?.find((game) => game.id === shownGameId) ?? null;
  useRootInput(registry);

  return (
    <ViewInputProvider value={registry}>
      <ConsoleBackdrop />
      <ConsoleTopBar />
      <main className={cn("cs-main", panel.mounted && "has-panel", panel.closing && "is-panel-closing")}>
        <div key={section} className="cs-view">
          {section === "library" ? <LibraryView /> : null}
          {section === "saves" ? <SavesView /> : null}
          {section === "cloud" ? <CloudView /> : null}
          {section === "settings" ? <SettingsView /> : null}
        </div>
        {panel.mounted && panelGame ? <GamePanel game={panelGame} closing={panel.closing} /> : null}
      </main>
      <HintBar />
      <ModalLayer />
      <ConsoleToasts />
    </ViewInputProvider>
  );
}
