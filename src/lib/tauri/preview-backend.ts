import type { AppSettings } from "@/lib/bindings/AppSettings";
import type { GameSummary } from "@/lib/bindings/GameSummary";
import type { SettingsPatch } from "@/lib/bindings/SettingsPatch";
import type { SnapshotSummary } from "@/lib/bindings/SnapshotSummary";
import type { ArtworkSelection } from "@/lib/bindings/ArtworkSelection";
import type { ClientSource } from "@/lib/bindings/ClientSource";
import type { CloudStatus } from "@/lib/bindings/CloudStatus";
import type { ConflictInfo } from "@/lib/bindings/ConflictInfo";
import { sampleCandidates } from "@/lib/tauri/preview-artwork";
import { sampleLocations, sampleSnapshots } from "@/lib/tauri/preview-saves";
import type { CommandMap, CommandName } from "@/lib/tauri/commands";
import { AppError } from "@/lib/tauri/errors";
import { EVENTS, emitPreviewEvent } from "@/lib/tauri/events";
import { sampleLibrary } from "@/lib/tauri/preview-library";
import { samplePlan } from "@/lib/tauri/preview-import";
import { DEFAULT_SETTINGS } from "@/features/settings/defaults";

const STORAGE_KEY = "l8play.preview.settings";
/** The welcome tour would cover every page of the design preview, so it starts as done. */
const PREVIEW_DEFAULTS: AppSettings = { ...DEFAULT_SETTINGS, onboardingDone: true };

function readStoredSettings(): AppSettings {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    return raw ? { ...PREVIEW_DEFAULTS, ...(JSON.parse(raw) as Partial<AppSettings>) } : PREVIEW_DEFAULTS;
  } catch (error) {
    console.warn("Preview backend: unreadable stored settings, using defaults.", error);
    return PREVIEW_DEFAULTS;
  }
}

function storeSettings(settings: AppSettings): void {
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
  } catch (error) {
    console.warn("Preview backend: settings could not be persisted.", error);
  }
}

function applyPatch(current: AppSettings, patch: SettingsPatch): AppSettings {
  const defined = Object.fromEntries(Object.entries(patch).filter(([, value]) => value !== undefined));
  return { ...current, ...defined } as AppSettings;
}

let library: GameSummary[] = sampleLibrary();
let snapshots: SnapshotSummary[] = sampleSnapshots(null);
let artworkKeySaved = false;
let steamKeySaved = false;
let cloudClientSource: ClientSource = "embedded";
let cloudAccount: CloudStatus["account"] = null;
let conflicts: ConflictInfo[] = [];
let developers = new Map<number, { developer: string | null; releaseYear: number | null }>();
const selections = new Map<number, ArtworkSelection[]>();

const DESKTOP_ONLY = "This needs the desktop app.";

function findGame(gameId: number): GameSummary {
  const game = library.find((candidate) => candidate.id === gameId);
  if (!game) throw new AppError("not_found", `game ${gameId}`);
  return game;
}

type Handlers = { [C in CommandName]: (args: CommandMap[C]["args"]) => CommandMap[C]["result"] };

const handlers: Handlers = {
  get_settings: () => readStoredSettings(),
  update_settings: ({ patch }) => {
    const next = applyPatch(readStoredSettings(), patch);
    storeSettings(next);
    return next;
  },
  get_app_info: () => ({
    version: "0.1.0-preview",
    databasePath: "(browser preview — no database)",
    schemaVersion: 1,
    ludusaviAvailable: true,
  }),
  scan_libraries_now: () => ({
    added: 0,
    updated: library.length,
    uninstalled: 0,
    total: library.length,
    platforms: [
      { platform: "steam", found: library.filter((game) => game.platform === "steam").length, error: null },
      { platform: "epic", found: library.filter((game) => game.platform === "epic").length, error: null },
      { platform: "xbox", found: library.filter((game) => game.platform === "xbox").length, error: null },
      { platform: "gog", found: library.filter((game) => game.platform === "gog").length, error: null },
    ],
    warnings: [],
    ownedLibrary: "listed",
    ownedAdded: 0,
  }),
  get_owned_library_status: () => (steamKeySaved ? "listed" : "needs_key"),
  get_steam_key_status: () => steamKeySaved,
  save_steam_key: ({ key }) => {
    if (!/^[0-9a-fA-F]{32}$/.test(key.trim()))
      throw new AppError("validation", "A Steam Web API key is 32 characters: digits 0-9 and letters A-F");
    steamKeySaved = true;
    return null;
  },
  clear_steam_key: () => {
    steamKeySaved = false;
    return null;
  },
  open_steam_api_key_page: () => null,
  list_games: () => library,
  get_game: ({ gameId }) => ({
    game: findGame(gameId),
    developer: developers.get(gameId)?.developer ?? "FromSoftware",
    releaseYear: developers.get(gameId)?.releaseYear ?? 2019,
    executablePath: null,
    addedAt: new Date().toISOString(),
    sessionCount: 3,
  }),
  get_link_candidates: () => [],
  link_archived_game: ({ targetId }) => findGame(targetId),
  set_favorite: ({ gameId, favorite }) => {
    library = library.map((game) => (game.id === gameId ? { ...game, favorite } : game));
    return findGame(gameId);
  },
  add_standalone_game: () => {
    throw new AppError("validation", DESKTOP_ONLY);
  },
  scan_game_folder: () => ({
    alreadyAdded: 1,
    candidates: [
      {
        folder: "D:\\Workspace\\Games\\The Last of Us",
        suggestedTitle: "The Last of Us",
        executables: [
          {
            path: "D:\\Workspace\\Games\\The Last of Us\\tlou-i.exe",
            relativePath: "tlou-i.exe",
            sizeBytes: 60_519_048,
            suggested: true,
          },
          {
            path: "D:\\Workspace\\Games\\The Last of Us\\tlou-i-l.exe",
            relativePath: "tlou-i-l.exe",
            sizeBytes: 60_494_464,
            suggested: false,
          },
          {
            path: "D:\\Workspace\\Games\\The Last of Us\\launcher.exe",
            relativePath: "launcher.exe",
            sizeBytes: 1_310_344,
            suggested: false,
          },
        ],
      },
      {
        folder: "D:\\Workspace\\Games\\Moonlighter",
        suggestedTitle: "Moonlighter",
        executables: [
          {
            path: "D:\\Workspace\\Games\\Moonlighter\\Moonlighter.exe",
            relativePath: "Moonlighter.exe",
            sizeBytes: 650_752,
            suggested: true,
          },
        ],
      },
    ],
  }),
  add_games: ({ games }) => {
    const added = games.map((game, index) => ({
      ...sampleLibrary()[0],
      id: 1000 + index + library.length,
      title: game.title,
      platform: "standalone" as const,
      platformGameId: null,
      installPath: game.folder,
      artwork: { cover: null, hero: null, logo: null, header: null, icon: null },
    })) as GameSummary[];
    library = [...library, ...added];
    // Like the real backend: artwork is looked up in the background and reported when it ends.
    for (const [index, game] of added.entries()) {
      setTimeout(() => emitPreviewEvent(EVENTS.artworkFetched, game.id), 4000 + index * 1500);
    }
    return { added, skipped: 0, failed: 0 };
  },
  get_running_games: () => [],
  launch_game: ({ gameId }) => {
    findGame(gameId);
    return null;
  },
  open_install_folder: () => null,
  list_save_locations: ({ gameId }) => sampleLocations(gameId),
  detect_saves: ({ gameId }) =>
    gameId === 1
      ? { status: "found", manifestTitle: "Sekiro: Shadows Die Twice", locations: sampleLocations(gameId) }
      : { status: "unknown", locations: [] },
  add_save_folder: () => {
    throw new AppError("validation", DESKTOP_ONLY);
  },
  remove_save_location: ({ gameId }) => sampleLocations(gameId),
  list_snapshots: ({ gameId }) => snapshots.filter((snapshot) => gameId === null || snapshot.gameId === gameId),
  get_save_statuses: () =>
    library.map((game) => ({
      gameId: game.id,
      locationCount: sampleLocations(game.id).length,
      snapshotCount: snapshots.filter((snapshot) => snapshot.gameId === game.id).length,
      lastSnapshotAt: snapshots.find((snapshot) => snapshot.gameId === game.id)?.createdAt ?? null,
      unknownToLudusavi: false,
    })),
  get_current_save: ({ gameId }) => ({
    totalFiles: gameId === 1 ? 1 : 0,
    totalBytes: gameId === 1 ? 10_276_000 : 0,
    lastModifiedAt: snapshots[0]?.createdAt ?? null,
    latestSnapshotId: snapshots[0]?.id ?? null,
    matchesLatest: gameId === 1,
  }),
  create_snapshot: () => {
    throw new AppError("validation", DESKTOP_ONLY);
  },
  restore_snapshot: () => {
    throw new AppError("validation", DESKTOP_ONLY);
  },
  delete_snapshot: ({ snapshotId }) => {
    snapshots = snapshots.filter((snapshot) => snapshot.id !== snapshotId);
    return null;
  },
  verify_snapshot: ({ snapshotId }) => {
    const found = snapshots.find((snapshot) => snapshot.id === snapshotId);
    if (!found) throw new AppError("not_found", snapshotId);
    return found;
  },
  set_snapshot_pinned: ({ snapshotId, pinned }) => {
    snapshots = snapshots.map((snapshot) => (snapshot.id === snapshotId ? { ...snapshot, pinned } : snapshot));
    const found = snapshots.find((snapshot) => snapshot.id === snapshotId);
    if (!found) throw new AppError("not_found", snapshotId);
    return found;
  },
  reveal_snapshot: () => null,
  get_cloud_status: () => ({
    clientSource: cloudClientSource,
    account: cloudAccount,
    quota: null,
    queued: cloudAccount ? 2 : 0,
    running: 0,
    failed: 0,
    openConflicts: conflicts.length,
    lastSyncAt: cloudAccount ? new Date(Date.now() - 600_000).toISOString() : null,
  }),
  get_cloud_quota: () => ({ usedBytes: 3_221_225_472, limitBytes: 16_106_127_360 }),
  save_google_client: ({ clientId }) => {
    if (!clientId.endsWith(".apps.googleusercontent.com")) {
      throw new AppError("validation", "the client id ends with .apps.googleusercontent.com");
    }
    cloudClientSource = "custom";
    return null;
  },
  clear_google_client: () => {
    cloudClientSource = "embedded";
    return null;
  },
  connect_google_drive: () => {
    cloudAccount = { email: "ryu@example.com", status: "connected", connectedAt: new Date().toISOString() };
    const local = snapshots[0];
    const other = snapshots[1];
    if (local && other && conflicts.length === 0) {
      const side = (snapshot: SnapshotSummary, device: string, localCopy: boolean) => ({
        snapshotId: snapshot.id,
        name: snapshot.name || "Game closed",
        createdAt: snapshot.createdAt,
        deviceName: device,
        totalFiles: snapshot.totalFiles,
        totalBytes: snapshot.totalBytes,
        playtimeSeconds: snapshot.playtimeSeconds,
        localCopy,
      });
      conflicts = [
        {
          id: 1,
          gameId: local.gameId,
          gameTitle: local.gameTitle,
          detectedAt: new Date().toISOString(),
          local: side(local, "DESKTOP-RYU", true),
          remote: side(other, "LAPTOP-RYU", false),
        },
      ];
    }
    return {
      clientSource: cloudClientSource,
      account: cloudAccount,
      quota: null,
      queued: 2,
      running: 0,
      failed: 0,
      openConflicts: conflicts.length,
      lastSyncAt: null,
    };
  },
  disconnect_google_drive: () => {
    cloudAccount = null;
    conflicts = [];
    return {
      clientSource: cloudClientSource,
      account: null,
      quota: null,
      queued: 0,
      running: 0,
      failed: 0,
      openConflicts: 0,
      lastSyncAt: null,
    };
  },
  sync_cloud_now: () => ({
    uploaded: 2,
    downloaded: 0,
    failed: 0,
    imported: 1,
    newConflicts: 0,
    skippedUnknownGames: 0,
  }),
  list_sync_operations: () =>
    cloudAccount
      ? [
          {
            id: 1,
            snapshotId: snapshots[0]?.id ?? "x",
            gameTitle: "Sekiro: Shadows Die Twice",
            snapshotName: "Game closed",
            direction: "upload",
            status: "running",
            attempts: 0,
            bytesDone: 4_194_304,
            bytesTotal: 9_800_000,
            lastError: null,
            nextAttemptAt: null,
          },
          {
            id: 2,
            snapshotId: snapshots[1]?.id ?? "y",
            gameTitle: "Sekiro: Shadows Die Twice",
            snapshotName: "Before Isshin",
            direction: "upload",
            status: "failed",
            attempts: 3,
            bytesDone: 0,
            bytesTotal: 9_800_000,
            lastError: "network error: connection reset",
            nextAttemptAt: new Date(Date.now() + 120_000).toISOString(),
          },
        ]
      : [],
  retry_sync: () => 1,
  list_conflicts: () => conflicts,
  resolve_conflict: () => {
    conflicts = [];
    return null;
  },
  download_snapshot: () => null,
  find_saves: ({ gameId }) => ({
    wiki: gameId === 1 ? "found" : "not_found",
    wikiPageUrl: gameId === 1 ? "https://www.pcgamingwiki.com/wiki/Sekiro:_Shadows_Die_Twice" : null,
    candidates:
      gameId === 1
        ? [
            {
              path: "C:\\Users\\Ryu\\AppData\\Roaming\\Sekiro",
              reason: "wiki_template",
              evidence: "{{p|appdata}}\\Sekiro",
              evidenceUrl: "https://www.pcgamingwiki.com/wiki/Sekiro:_Shadows_Die_Twice",
              fileCount: 4,
              totalBytes: 10_276_000,
              newestModifiedAt: new Date(Date.now() - 3_600_000).toISOString(),
            },
            {
              path: "C:\\Users\\Ryu\\Documents\\Sekiro Backup",
              reason: "folder_name_matches_title",
              evidence: "Sekiro Backup",
              evidenceUrl: null,
              fileCount: 2,
              totalBytes: 2048,
              newestModifiedAt: null,
            },
          ]
        : [],
  }),
  open_wiki_page: () => null,
  scan_backup_folder: () => samplePlan(),
  scan_pc_for_leftover_saves: () => samplePlan(),
  export_all_saves: () => ({ path: "D:/Exports/L8Play-saves.zip", snapshots: 12, games: 5, bytes: 48_000_000, skipped: 0 }),
  replan_backup_game: () => samplePlan().games[0] ?? null,
  import_backup_games: ({ games }) => ({
    games: games.map((game) => ({
      gameId: null,
      title: game.title,
      created: true,
      imported: game.snapshots.length,
      snapshotIds: [],
      duplicates: 0,
      errors: [],
    })),
  }),
  get_restore_targets: () => [
    { path: "C:\\Users\\Ryu\\AppData\\Local\\SB\\Saved", source: "detected", exists: true },
  ],
  restore_snapshot_to: ({ snapshotId }) => ({
    restoredSnapshotId: snapshotId,
    safetySnapshotId: null,
    filesRestored: 3,
  }),
  rename_snapshot: ({ snapshotId, label }) => {
    const found = snapshots.find((snapshot) => snapshot.id === snapshotId);
    if (!found) throw new AppError("not_found", `snapshot ${snapshotId}`);
    found.label = label;
    return found;
  },
  get_artwork_key_status: () => artworkKeySaved,
  get_artwork_pending: () => [],
  save_artwork_key: ({ key }) => {
    if (key.trim().length < 16)
      throw new AppError("validation", "A SteamGridDB API key is 16 to 128 letters and digits");
    artworkKeySaved = true;
    return null;
  },
  clear_artwork_key: () => {
    artworkKeySaved = false;
    return null;
  },
  verify_artwork_key: () => {
    if (!artworkKeySaved) throw new AppError("auth", "no key");
    return null;
  },
  get_artwork_selections: ({ gameId }) => selections.get(gameId) ?? [],
  find_artwork: ({ kind, query, gameId }) => {
    if (!artworkKeySaved) throw new AppError("auth", "no key");
    return sampleCandidates(kind, query ?? findGame(gameId).title);
  },
  apply_artwork: ({ gameId, candidate }) => {
    const others = (selections.get(gameId) ?? []).filter((entry) => entry.kind !== candidate.kind);
    selections.set(gameId, [...others, { kind: candidate.kind, provider: "steamgriddb", userSelected: true }]);
    library = library.map((game) =>
      game.id === gameId ? { ...game, artwork: { ...game.artwork, [candidate.kind]: candidate.url } } : game,
    );
    return null;
  },
  import_artwork: () => {
    throw new AppError("validation", DESKTOP_ONLY);
  },
  reset_artwork: ({ gameId, kind }) => {
    selections.set(
      gameId,
      (selections.get(gameId) ?? []).filter((entry) => entry.kind !== kind),
    );
    return null;
  },
  fetch_missing_artwork: () => ({ fetched: 0, notFound: 0, failed: 0 }),
  update_game_metadata: ({ gameId, title, developer, releaseYear }) => {
    if (title.trim() === "") throw new AppError("validation", "The title must be 1 to 120 characters");
    developers = new Map(developers).set(gameId, { developer, releaseYear });
    library = library.map((game) => (game.id === gameId ? { ...game, title: title.trim() } : game));
    return findGame(gameId);
  },
  open_artwork_key_page: () => null,
  get_snapshot_folder: () => "C:\\Users\\Ryu\\Documents\\L8Play\\Snapshots",
};

/**
 * Stand-in for the Rust backend when the UI runs in a plain browser (design review, tests).
 * It never touches the file system; it only mirrors the command contract with sample data.
 */
export const previewBackend = {
  async handle<K extends CommandName>(command: K, args?: CommandMap[K]["args"]): Promise<CommandMap[K]["result"]> {
    const handler = handlers[command] as ((args: CommandMap[K]["args"]) => CommandMap[K]["result"]) | undefined;
    if (!handler) throw new AppError("internal", `Unknown command: ${command}`);
    return handler(args as CommandMap[K]["args"]);
  },
};
