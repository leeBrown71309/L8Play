import { isTauri } from "@/lib/tauri/runtime";

const REMOTE = /^(https?:|data:|blob:)/;
/** Game id and file name of a cached image: `...\artwork\5\cover-<hash>.jpg`. */
const CACHED = /[\\/]artwork[\\/](\d+)[\\/]([^\\/]+)$/;

/**
 * Artwork fields hold either a remote URL or an absolute path in the local cache. The WebView
 * reads cached files through the app's own `l8art` protocol (Windows serves it as
 * `http://l8art.localhost`), which only ever serves files the artwork cache wrote.
 */
export function toImageSrc(source: string): string {
  if (REMOTE.test(source) || !isTauri()) return source;
  const match = CACHED.exec(source);
  return match ? `http://l8art.localhost/${match[1] ?? ""}/${encodeURIComponent(match[2] ?? "")}` : source;
}
