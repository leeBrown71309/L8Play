/**
 * Builds the `latest.json` manifest read by tauri-plugin-updater for a GitHub release.
 *
 * Usage:
 *   bun scripts/make-update-manifest.ts --version 0.1.3 --notes-file notes.md
 *   bun scripts/make-update-manifest.ts --version 0.1.3 --notes "First updater release"
 *
 * By default the signature is read from
 * `target/release/bundle/nsis/L8Play_<version>_x64-setup.exe.sig` and the installer URL points at
 * the matching GitHub release asset. Every option can be overridden with `--sig`, `--url`,
 * `--out` and `--pub-date`.
 */
import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

export const GITHUB_REPOSITORY = "leeBrown71309/L8Play";
export const DEFAULT_OUTPUT = "latest.json";
export const WINDOWS_PLATFORM = "windows-x86_64";

const VERSION_PATTERN = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/;
const OPTION_NAMES = new Set(["version", "notes", "notes-file", "sig", "url", "out", "pub-date"]);

export interface UpdateManifest {
  version: string;
  notes: string;
  pub_date: string;
  platforms: {
    [WINDOWS_PLATFORM]: {
      signature: string;
      url: string;
    };
  };
}

export interface ManifestInput {
  version: string;
  notes: string;
  pubDate: string;
  signature: string;
  url: string;
}

const USAGE = `Usage: bun scripts/make-update-manifest.ts --version <x.y.z> [options]

Options:
  --version <x.y.z>     Release version (required)
  --notes <text>        Release notes as plain text
  --notes-file <path>   Read the release notes from a file
  --sig <path>          Signature file (default: target/release/bundle/nsis/...)
  --url <url>           Installer URL (default: the matching GitHub release asset)
  --out <path>          Output file (default: latest.json)
  --pub-date <iso>      Publication date (default: now, RFC 3339)
`;

/** URL of the NSIS installer attached to the GitHub release for this version. */
export function defaultInstallerUrl(version: string): string {
  return `https://github.com/${GITHUB_REPOSITORY}/releases/download/v${version}/L8Play_${version}_x64-setup.exe`;
}

/** Where `bun tauri build` writes the updater signature for this version. */
export function defaultSignaturePath(version: string, workingDirectory = process.cwd()): string {
  return resolve(workingDirectory, "target", "release", "bundle", "nsis", `L8Play_${version}_x64-setup.exe.sig`);
}

function rfc3339(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) throw new Error(`Invalid date: ${value}`);
  return date.toISOString();
}

/** Builds the manifest object. The signature is trimmed because the `.sig` file ends with a newline. */
export function buildManifest({ version, notes, pubDate, signature, url }: ManifestInput): UpdateManifest {
  if (!VERSION_PATTERN.test(version)) throw new Error(`Invalid version: ${version}`);
  const cleanedSignature = signature.trim();
  if (cleanedSignature === "") throw new Error("The update signature is empty");
  if (notes.trim() === "") throw new Error("Release notes are empty");
  if (!/^https:\/\//.test(url)) throw new Error(`The installer URL must use HTTPS: ${url}`);
  return {
    version,
    notes,
    pub_date: rfc3339(pubDate),
    platforms: {
      [WINDOWS_PLATFORM]: {
        signature: cleanedSignature,
        url,
      },
    },
  };
}

/** Reads a Tauri `.sig` file, rejecting an empty one before a broken manifest is published. */
export function readSignature(path: string): string {
  const signature = readFileSync(path, "utf8").trim();
  if (signature === "") throw new Error(`The signature file is empty: ${path}`);
  return signature;
}

/** Parses `--name value` pairs; unknown or incomplete options fail instead of being ignored. */
export function parseArgs(argv: readonly string[]): Map<string, string> {
  const parsed = new Map<string, string>();
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (!argument || !argument.startsWith("--")) throw new Error(`Unexpected argument: ${String(argument)}`);
    const name = argument.slice(2);
    if (!OPTION_NAMES.has(name)) throw new Error(`Unknown option: --${name}`);
    const value = argv[index + 1];
    if (value === undefined || value.startsWith("--")) throw new Error(`Missing value for --${name}`);
    parsed.set(name, value);
    index += 1;
  }
  return parsed;
}

export interface ManifestFromArgs {
  manifest: UpdateManifest;
  outputPath: string;
}

/** Turns parsed options into the manifest and the path it must be written to. */
export function manifestFromArgs(
  args: Map<string, string>,
  workingDirectory = process.cwd(),
  now = new Date(),
): ManifestFromArgs {
  const version = args.get("version");
  if (!version) throw new Error("--version is required");
  const notesFile = args.get("notes-file");
  const notes =
    args.get("notes") ?? (notesFile ? readFileSync(resolve(workingDirectory, notesFile), "utf8").trim() : "");
  if (notes.trim() === "") throw new Error("--notes or --notes-file is required");
  const signaturePath = args.get("sig") ?? defaultSignaturePath(version, workingDirectory);
  const signature = readSignature(signaturePath);
  const url = args.get("url") ?? defaultInstallerUrl(version);
  const pubDate = args.get("pub-date") ?? now.toISOString();
  const manifest = buildManifest({ version, notes, pubDate, signature, url });
  const outputPath = resolve(workingDirectory, args.get("out") ?? DEFAULT_OUTPUT);
  return { manifest, outputPath };
}

/** CLI entry point, kept separate so tests can exercise every function above. */
export function main(argv: readonly string[], workingDirectory = process.cwd()): void {
  if (argv.includes("--help") || argv.includes("-h")) {
    console.log(USAGE);
    return;
  }
  const { manifest, outputPath } = manifestFromArgs(parseArgs(argv), workingDirectory);
  writeFileSync(outputPath, `${JSON.stringify(manifest, null, 2)}\n`, "utf8");
  console.log(`Wrote ${outputPath} for L8Play ${manifest.version}.`);
}

const invokedPath = process.argv[1];
if (invokedPath && import.meta.url === pathToFileURL(resolve(invokedPath)).href) {
  try {
    main(process.argv.slice(2));
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
