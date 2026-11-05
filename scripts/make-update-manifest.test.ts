import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";

import {
  buildManifest,
  defaultInstallerUrl,
  defaultSignaturePath,
  main,
  manifestFromArgs,
  parseArgs,
  readSignature,
} from "./make-update-manifest";

const temporaryDirectories: string[] = [];

function temporaryDirectory(): string {
  const directory = mkdtempSync(join(tmpdir(), "l8play-manifest-"));
  temporaryDirectories.push(directory);
  return directory;
}

afterEach(() => {
  for (const directory of temporaryDirectories.splice(0)) rmSync(directory, { recursive: true, force: true });
});

describe("defaultInstallerUrl", () => {
  it("points at the GitHub release asset", () => {
    expect(defaultInstallerUrl("0.1.3")).toBe(
      "https://github.com/leeBrown71309/L8Play/releases/download/v0.1.3/L8Play_0.1.3_x64-setup.exe",
    );
  });
});

describe("defaultSignaturePath", () => {
  it("uses the NSIS bundle folder", () => {
    expect(defaultSignaturePath("0.1.3", "C:/work")).toMatch(
      /target[\\/]release[\\/]bundle[\\/]nsis[\\/]L8Play_0.1.3_x64-setup\.exe\.sig$/,
    );
  });
});

describe("buildManifest", () => {
  const input = {
    version: "0.1.3",
    notes: "First updater release\n\n- Fixes",
    pubDate: "2026-11-05T12:00:00Z",
    signature: "  trusted comment: minisign signature  \n",
    url: defaultInstallerUrl("0.1.3"),
  };

  it("builds the v2 manifest with a trimmed signature", () => {
    const manifest = buildManifest(input);
    expect(manifest).toEqual({
      version: "0.1.3",
      notes: input.notes,
      pub_date: "2026-11-05T12:00:00.000Z",
      platforms: {
        "windows-x86_64": {
          signature: "trusted comment: minisign signature",
          url: input.url,
        },
      },
    });
  });

  it("rejects invalid versions, empty notes, empty signatures and insecure URLs", () => {
    expect(() => buildManifest({ ...input, version: "1.0" })).toThrow(/Invalid version/);
    expect(() => buildManifest({ ...input, notes: "   " })).toThrow(/notes/);
    expect(() => buildManifest({ ...input, signature: "  " })).toThrow(/signature/);
    expect(() => buildManifest({ ...input, url: "http://example.com/setup.exe" })).toThrow(/HTTPS/);
  });

  it("rejects an invalid publication date", () => {
    expect(() => buildManifest({ ...input, pubDate: "not a date" })).toThrow(/Invalid date/);
  });
});

describe("readSignature", () => {
  it("reads and trims the .sig file", () => {
    const directory = temporaryDirectory();
    const path = join(directory, "installer.exe.sig");
    writeFileSync(path, "signature-value\n", "utf8");
    expect(readSignature(path)).toBe("signature-value");
  });

  it("rejects an empty .sig file", () => {
    const directory = temporaryDirectory();
    const path = join(directory, "empty.sig");
    writeFileSync(path, "  \n", "utf8");
    expect(() => readSignature(path)).toThrow(/empty/);
  });
});

describe("parseArgs", () => {
  it("parses values and ignores nothing silently", () => {
    expect([...parseArgs(["--version", "0.1.3", "--notes", "Hello"])]).toEqual([
      ["version", "0.1.3"],
      ["notes", "Hello"],
    ]);
    expect(() => parseArgs(["--unknown", "x"])).toThrow(/Unknown option/);
    expect(() => parseArgs(["--version"])).toThrow(/Missing value/);
    expect(() => parseArgs(["stray"])).toThrow(/Unexpected argument/);
  });
});

describe("manifestFromArgs", () => {
  it("defaults the signature path, URL, output and publication date", () => {
    const directory = temporaryDirectory();
    const signatureDirectory = join(directory, "target", "release", "bundle", "nsis");
    const signaturePath = join(signatureDirectory, "L8Play_0.1.3_x64-setup.exe.sig");
    writeFileSync(join(directory, "notes.md"), "Notes from a file\n", "utf8");
    mkdirSync(signatureDirectory, { recursive: true });
    writeFileSync(signaturePath, "signature\n", "utf8");

    const { manifest, outputPath } = manifestFromArgs(
      new Map([
        ["version", "0.1.3"],
        ["notes-file", "notes.md"],
      ]),
      directory,
      new Date("2026-11-05T12:00:00Z"),
    );

    expect(manifest.notes).toBe("Notes from a file");
    expect(manifest.pub_date).toBe("2026-11-05T12:00:00.000Z");
    expect(manifest.platforms["windows-x86_64"].url).toBe(defaultInstallerUrl("0.1.3"));
    expect(outputPath).toBe(join(directory, "latest.json"));
  });

  it("writes latest.json through the CLI entry point", () => {
    const directory = temporaryDirectory();
    const signaturePath = join(directory, "custom.sig");
    writeFileSync(signaturePath, "signature\n", "utf8");
    main(
      [
        "--version",
        "0.1.3",
        "--notes",
        "Release notes",
        "--sig",
        signaturePath,
        "--out",
        "out.json",
        "--pub-date",
        "2026-11-05T12:00:00Z",
      ],
      directory,
    );
    const manifest = JSON.parse(readFileSync(join(directory, "out.json"), "utf8")) as {
      version: string;
      platforms: Record<string, { signature: string }>;
    };
    expect(manifest.version).toBe("0.1.3");
    expect(manifest.platforms["windows-x86_64"]?.signature).toBe("signature");
  });
});
