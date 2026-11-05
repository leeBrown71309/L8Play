import { defineConfig } from "@lingui/cli";

/** English is the source language; every user-facing string must also exist in French. */
export default defineConfig({
  sourceLocale: "en",
  locales: ["en", "fr"],
  catalogs: [
    {
      path: "<rootDir>/src/locales/{locale}/messages",
      include: ["src"],
      exclude: ["src/lib/bindings/**", "**/*.test.tsx", "**/*.test.ts"],
    },
  ],
});
