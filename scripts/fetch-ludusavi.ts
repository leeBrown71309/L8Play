/**
 * Downloads the pinned Ludusavi CLI release (MIT) used as a read-only sidecar for save detection,
 * verifies its SHA-256, and places it where Tauri expects external binaries.
 *
 * Usage: bun run fetch:ludusavi   (run once after cloning; the binary is not committed)
 */
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const VERSION = "0.31.0";
const BASE = `https://github.com/mtkennerly/ludusavi/releases/download/v${VERSION}`;
/** SHA-256 of ludusavi.exe inside ludusavi-v0.31.0-win64.zip (verified when pinned). */
const EXPECTED_SHA256 = "5192fd1ef31b5718cdb787d3f41abdcca8682ba4cbad31812d44ca6630ec2c0c";

const root = resolve(import.meta.dirname, "..");
const binaries = join(root, "src-tauri", "binaries");
const target = join(binaries, "ludusavi-x86_64-pc-windows-msvc.exe");
const notices = join(binaries, "ludusavi-third-party-notices.txt");

function sha256(path: string): string {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

async function download(url: string, destination: string): Promise<void> {
  const response = await fetch(url, { headers: { "User-Agent": "L8Play-build" } });
  if (!response.ok) throw new Error(`Download failed (${response.status}): ${url}`);
  writeFileSync(destination, new Uint8Array(await response.arrayBuffer()));
}

function extract(archive: string, destination: string): void {
  // Windows 10+ ships bsdtar (which reads zip files) in System32; a GNU tar earlier on PATH
  // (e.g. from Git Bash) would misread "C:" as a remote host.
  const tar = join(process.env["SystemRoot"] ?? "C:\\Windows", "System32", "tar.exe");
  const result = spawnSync(tar, ["-xf", archive, "-C", destination]);
  if (result.status !== 0) throw new Error(`Could not extract ${archive}: ${String(result.stderr)}`);
}

if (existsSync(target) && sha256(target) === EXPECTED_SHA256 && existsSync(notices)) {
  console.log(`Ludusavi ${VERSION} already present and verified.`);
  process.exit(0);
}

const work = mkdtempSync(join(tmpdir(), "ludusavi-"));
try {
  await download(`${BASE}/ludusavi-v${VERSION}-win64.zip`, join(work, "win64.zip"));
  await download(`${BASE}/ludusavi-v${VERSION}-legal.zip`, join(work, "legal.zip"));
  extract(join(work, "win64.zip"), work);
  extract(join(work, "legal.zip"), work);

  const exe = join(work, "ludusavi.exe");
  const actual = sha256(exe);
  if (actual !== EXPECTED_SHA256) throw new Error(`Checksum mismatch for ludusavi.exe: ${actual}`);

  mkdirSync(binaries, { recursive: true });
  // Copy, not rename: the temporary folder can be on another drive than the repository (CI runners keep
  // temp on C: and the checkout on D:), and a rename cannot cross drives. The folder is removed below.
  copyFileSync(exe, target);
  copyFileSync(join(work, `ludusavi-v${VERSION}-legal.txt`), notices);
  console.log(`Ludusavi ${VERSION} verified and installed to ${target}`);
} finally {
  rmSync(work, { recursive: true, force: true });
}
