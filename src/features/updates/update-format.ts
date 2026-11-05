/** Stable failure kinds shown to the user; the raw plugin error never reaches the screen. */
export type UpdateErrorCode = "offline" | "signature" | "unreachable" | "unknown";

/**
 * Classifies an updater failure by its message. The plugin reports network, TLS and minisign
 * failures as strings, so this is the narrowest reliable signal available to the WebView.
 */
export function classifyUpdateError(error: unknown): UpdateErrorCode {
  if (typeof navigator !== "undefined" && navigator.onLine === false) return "offline";
  const message = (error instanceof Error ? error.message : String(error)).toLowerCase();
  if (/signature|minisign|public key|verif|integrity/.test(message)) return "signature";
  if (/network|connect|dns|offline|timed? ?out|timeout|failed to fetch|error sending request/.test(message)) {
    return "unreachable";
  }
  return "unknown";
}

/** Publication date in the user's language, or `null` when the manifest has none. */
export function formatUpdateDate(iso: string | undefined, locale: string): string | null {
  if (!iso) return null;
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return null;
  return new Intl.DateTimeFormat(locale, { dateStyle: "medium", timeStyle: "short" }).format(date);
}

/**
 * Release notes are plain text. Tags are stripped so a manifest written with Markdown/HTML is
 * still readable, and React escapes the result instead of rendering it as HTML.
 */
export function cleanReleaseNotes(body: string | undefined): string {
  if (!body) return "";
  const normalized = body.replace(/\r\n?/g, "\n");
  const withoutControls = Array.from(normalized)
    .filter((character) => {
      const code = character.codePointAt(0) ?? 0;
      return code === 9 || code === 10 || (code >= 32 && code !== 127);
    })
    .join("");
  return withoutControls
    .replace(/<[^>]*>/g, "")
    .replace(/[ \t]+\n/g, "\n")
    .replace(/\n{3,}/g, "\n\n")
    .trim();
}
