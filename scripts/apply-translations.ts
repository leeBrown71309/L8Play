/**
 * Fills empty `msgstr` entries of a Lingui .po catalogue from scripts/translations/<locale>.json
 * (English source text → translation). Existing translations are never overwritten.
 * Usage: bun scripts/apply-translations.ts fr
 */
import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

const locale = process.argv[2];
if (!locale) throw new Error("Usage: bun scripts/apply-translations.ts <locale>");

const root = resolve(import.meta.dirname, "..");
// fr.json plus any partial fr-<area>.json next to it (later files win), so areas can be translated independently.
const translationsDir = resolve(root, "scripts/translations");
const dictionary: Record<string, string> = {};
for (const file of readdirSync(translationsDir)
  .filter((name) => name === `${locale}.json` || (name.startsWith(`${locale}-`) && name.endsWith(".json")))
  .sort()) {
  Object.assign(dictionary, JSON.parse(readFileSync(resolve(translationsDir, file), "utf8")) as Record<string, string>);
}
const poPath = resolve(root, `src/locales/${locale}/messages.po`);
const lines = readFileSync(poPath, "utf8").split(/\r?\n/);

const escape = (text: string) => text.replaceAll("\\", "\\\\").replaceAll('"', '\\"');
const unescape = (text: string) => text.replaceAll('\\"', '"').replaceAll("\\\\", "\\");

let filled = 0;
const missing: string[] = [];
for (let index = 0; index < lines.length - 1; index += 1) {
  const idMatch = /^msgid "(.*)"$/.exec(lines[index] ?? "");
  if (!idMatch || lines[index + 1] !== 'msgstr ""') continue;
  const source = unescape(idMatch[1] ?? "");
  if (source === "") continue;
  const translation = dictionary[source];
  if (translation) {
    lines[index + 1] = `msgstr "${escape(translation)}"`;
    filled += 1;
  } else {
    missing.push(source);
  }
}

writeFileSync(poPath, lines.join("\n"));
console.log(`${locale}: ${filled} translations applied.`);
if (missing.length > 0) {
  console.error(`${locale}: ${missing.length} strings still untranslated:\n  ${missing.join("\n  ")}`);
  process.exitCode = 1;
}
