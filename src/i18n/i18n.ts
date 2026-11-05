import { i18n } from "@lingui/core";

import type { Locale } from "@/lib/bindings/Locale";

export const SUPPORTED_LOCALES: readonly Locale[] = ["en", "fr"];

/** Loads a catalogue on demand (the Lingui Vite plugin compiles .po files at import) and activates it. */
export async function activateLocale(locale: Locale): Promise<void> {
  const { messages } = await import(`../locales/${locale}/messages.po`);
  i18n.loadAndActivate({ locale, messages });
  document.documentElement.lang = locale;
}
