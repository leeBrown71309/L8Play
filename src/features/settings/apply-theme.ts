import type { Theme } from "@/lib/bindings/Theme";

/** Switches the design-system theme; tokens.css keys every colour on `data-theme`. */
export function applyTheme(theme: Theme): void {
  document.documentElement.dataset.theme = theme;
}
