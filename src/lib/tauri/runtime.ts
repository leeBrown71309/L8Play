/**
 * True inside the Tauri WebView. In a plain browser (vite dev server, tests) the app runs
 * against the in-memory preview backend instead.
 */
export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}
