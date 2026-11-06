import { useConsoleNav } from "@/features/console/core/nav-store";

/** Asks before leaving console mode: a stray button press must not drop the user back to the desktop. */
export function requestConsoleExit(): void {
  const nav = useConsoleNav.getState();
  if (nav.modals.some((modal) => modal.kind === "confirm-exit")) return;
  nav.dispatch({ type: "pushModal", modal: { kind: "confirm-exit" } });
}
