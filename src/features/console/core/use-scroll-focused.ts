import { useEffect, type RefObject } from "react";

/** Keeps the focused row (marked `data-focused`) in view inside a scrolling list, with a smooth scroll. */
export function useScrollFocused(container: RefObject<HTMLElement | null>, focusKey: unknown): void {
  useEffect(() => {
    const target = container.current?.querySelector<HTMLElement>("[data-focused='true']");
    target?.scrollIntoView({ block: "nearest", behavior: "smooth" });
  }, [container, focusKey]);
}
