import { invoke } from "@tauri-apps/api/core";

import type { AddGamesReport } from "@/lib/bindings/AddGamesReport";
import type { AppInfo } from "@/lib/bindings/AppInfo";
import type { ArtworkCandidate } from "@/lib/bindings/ArtworkCandidate";
import type { ArtworkFetchReport } from "@/lib/bindings/ArtworkFetchReport";
import type { ArtworkKind } from "@/lib/bindings/ArtworkKind";
import type { ArtworkSelection } from "@/lib/bindings/ArtworkSelection";
import type { AppSettings } from "@/lib/bindings/AppSettings";
import type { CloudQuota } from "@/lib/bindings/CloudQuota";
import type { CloudStatus } from "@/lib/bindings/CloudStatus";
import type { ConflictInfo } from "@/lib/bindings/ConflictInfo";
import type { SnapshotStorage } from "@/lib/bindings/SnapshotStorage";
import type { ConflictResolution } from "@/lib/bindings/ConflictResolution";
import type { CreateSnapshotResult } from "@/lib/bindings/CreateSnapshotResult";
import type { CurrentSaveState } from "@/lib/bindings/CurrentSaveState";
import type { DetectionOutcome } from "@/lib/bindings/DetectionOutcome";
import type { FindSavesOutcome } from "@/lib/bindings/FindSavesOutcome";
import type { FolderScanReport } from "@/lib/bindings/FolderScanReport";
import type { GameDetail } from "@/lib/bindings/GameDetail";
import type { GameSaveStatus } from "@/lib/bindings/GameSaveStatus";
import type { GameSummary } from "@/lib/bindings/GameSummary";
import type { ImportGameRequest } from "@/lib/bindings/ImportGameRequest";
import type { ExportReport } from "@/lib/bindings/ExportReport";
import type { ImportPlan } from "@/lib/bindings/ImportPlan";
import type { ImportReport } from "@/lib/bindings/ImportReport";
import type { PlannedGame } from "@/lib/bindings/PlannedGame";
import type { PlannedLayout } from "@/lib/bindings/PlannedLayout";
import type { ScanMode } from "@/lib/bindings/ScanMode";
import type { NewGameInput } from "@/lib/bindings/NewGameInput";
import type { RestoreReport } from "@/lib/bindings/RestoreReport";
import type { RestoreTarget } from "@/lib/bindings/RestoreTarget";
import type { SaveLocation } from "@/lib/bindings/SaveLocation";
import type { OwnedLibraryStatus } from "@/lib/bindings/OwnedLibraryStatus";
import type { ScanReport } from "@/lib/bindings/ScanReport";
import type { SettingsPatch } from "@/lib/bindings/SettingsPatch";
import type { SnapshotSummary } from "@/lib/bindings/SnapshotSummary";
import type { SyncOperationInfo } from "@/lib/bindings/SyncOperationInfo";
import type { SyncReport } from "@/lib/bindings/SyncReport";
import { toAppError } from "@/lib/tauri/errors";
import { previewBackend } from "@/lib/tauri/preview-backend";
import { isTauri } from "@/lib/tauri/runtime";

/** Typed signatures of every backend command (names must match `src-tauri/src/lib.rs`). */
export interface CommandMap {
  get_settings: { args: undefined; result: AppSettings };
  update_settings: { args: { patch: SettingsPatch }; result: AppSettings };
  get_app_info: { args: undefined; result: AppInfo };
  scan_libraries_now: { args: undefined; result: ScanReport };
  get_owned_library_status: { args: undefined; result: OwnedLibraryStatus };
  get_steam_key_status: { args: undefined; result: boolean };
  save_steam_key: { args: { key: string }; result: null };
  clear_steam_key: { args: undefined; result: null };
  open_steam_api_key_page: { args: undefined; result: null };
  list_games: { args: undefined; result: GameSummary[] };
  get_game: { args: { gameId: number }; result: GameDetail };
  get_link_candidates: { args: { gameId: number }; result: GameSummary[] };
  link_archived_game: { args: { archivedId: number; targetId: number }; result: GameSummary };
  set_favorite: { args: { gameId: number; favorite: boolean }; result: GameSummary };
  add_standalone_game: { args: { executablePath: string; title: string | null }; result: GameSummary };
  scan_game_folder: { args: { folder: string }; result: FolderScanReport };
  add_games: { args: { games: NewGameInput[] }; result: AddGamesReport };
  get_running_games: { args: undefined; result: number[] };
  launch_game: { args: { gameId: number }; result: null };
  open_install_folder: { args: { gameId: number }; result: null };
  list_save_locations: { args: { gameId: number }; result: SaveLocation[] };
  detect_saves: { args: { gameId: number }; result: DetectionOutcome };
  add_save_folder: { args: { gameId: number; folder: string }; result: SaveLocation[] };
  remove_save_location: { args: { gameId: number; locationId: number }; result: SaveLocation[] };
  list_snapshots: { args: { gameId: number | null }; result: SnapshotSummary[] };
  get_save_statuses: { args: undefined; result: GameSaveStatus[] };
  get_current_save: { args: { gameId: number }; result: CurrentSaveState };
  create_snapshot: {
    args: { gameId: number; name: string; description: string | null; pinned: boolean; storage: SnapshotStorage };
    result: CreateSnapshotResult;
  };
  restore_snapshot: { args: { snapshotId: string }; result: RestoreReport };
  delete_snapshot: { args: { snapshotId: string; alsoFromDrive?: boolean }; result: null };
  verify_snapshot: { args: { snapshotId: string }; result: SnapshotSummary };
  set_snapshot_pinned: { args: { snapshotId: string; pinned: boolean }; result: SnapshotSummary };
  reveal_snapshot: { args: { snapshotId: string | null }; result: null };
  get_snapshot_folder: { args: undefined; result: string };
  find_saves: { args: { gameId: number }; result: FindSavesOutcome };
  open_wiki_page: { args: { url: string }; result: null };
  scan_backup_folder: { args: { folder: string; mode: ScanMode | null }; result: ImportPlan };
  scan_pc_for_leftover_saves: { args: undefined; result: ImportPlan };
  export_all_saves: { args: { folder: string }; result: ExportReport };
  replan_backup_game: {
    args: { folder: string; key: string; fromCollection: string | null; layout: PlannedLayout };
    result: PlannedGame | null;
  };
  import_backup_games: {
    args: { games: ImportGameRequest[]; storage: SnapshotStorage | null };
    result: ImportReport;
  };
  get_restore_targets: { args: { snapshotId: string }; result: RestoreTarget[] };
  restore_snapshot_to: { args: { snapshotId: string; target: string }; result: RestoreReport };
  rename_snapshot: { args: { snapshotId: string; label: string | null }; result: SnapshotSummary };
  get_artwork_key_status: { args: undefined; result: boolean };
  get_artwork_pending: { args: undefined; result: number[] };
  save_artwork_key: { args: { key: string }; result: null };
  clear_artwork_key: { args: undefined; result: null };
  verify_artwork_key: { args: undefined; result: null };
  get_artwork_selections: { args: { gameId: number }; result: ArtworkSelection[] };
  find_artwork: {
    args: { gameId: number; kind: ArtworkKind; query: string | null };
    result: ArtworkCandidate[];
  };
  apply_artwork: { args: { gameId: number; candidate: ArtworkCandidate }; result: null };
  import_artwork: { args: { gameId: number; kind: ArtworkKind; path: string }; result: null };
  reset_artwork: { args: { gameId: number; kind: ArtworkKind }; result: null };
  fetch_missing_artwork: { args: undefined; result: ArtworkFetchReport };
  update_game_metadata: {
    args: { gameId: number; title: string; developer: string | null; releaseYear: number | null };
    result: GameSummary;
  };
  open_artwork_key_page: { args: undefined; result: null };
  get_cloud_status: { args: undefined; result: CloudStatus };
  get_cloud_quota: { args: undefined; result: CloudQuota };
  save_google_client: { args: { clientId: string; clientSecret: string }; result: null };
  clear_google_client: { args: undefined; result: null };
  connect_google_drive: { args: undefined; result: CloudStatus };
  disconnect_google_drive: { args: undefined; result: CloudStatus };
  sync_cloud_now: { args: undefined; result: SyncReport };
  list_sync_operations: { args: undefined; result: SyncOperationInfo[] };
  retry_sync: { args: undefined; result: number };
  list_conflicts: { args: undefined; result: ConflictInfo[] };
  resolve_conflict: { args: { conflictId: number; resolution: ConflictResolution }; result: null };
  download_snapshot: { args: { snapshotId: string }; result: null };
}

export type CommandName = keyof CommandMap;

async function call<K extends CommandName>(command: K, args?: CommandMap[K]["args"]): Promise<CommandMap[K]["result"]> {
  if (!isTauri()) return previewBackend.handle(command, args);
  try {
    return await invoke<CommandMap[K]["result"]>(command, args ?? {});
  } catch (error) {
    throw toAppError(error);
  }
}

export const getSettings = () => call("get_settings");
export const updateSettings = (patch: SettingsPatch) => call("update_settings", { patch });
export const getAppInfo = () => call("get_app_info");

export const scanLibraries = () => call("scan_libraries_now");
export const getOwnedLibraryStatus = () => call("get_owned_library_status");
export const getSteamKeyStatus = () => call("get_steam_key_status");
export const saveSteamKey = (key: string) => call("save_steam_key", { key });
export const clearSteamKey = () => call("clear_steam_key");
export const openSteamApiKeyPage = () => call("open_steam_api_key_page");
export const listGames = () => call("list_games");
export const getGame = (gameId: number) => call("get_game", { gameId });
export const getLinkCandidates = (gameId: number) => call("get_link_candidates", { gameId });
export const linkArchivedGame = (archivedId: number, targetId: number) =>
  call("link_archived_game", { archivedId, targetId });
export const setFavorite = (gameId: number, favorite: boolean) => call("set_favorite", { gameId, favorite });
export const addStandaloneGame = (executablePath: string, title: string | null = null) =>
  call("add_standalone_game", { executablePath, title });
export const scanGameFolder = (folder: string) => call("scan_game_folder", { folder });
export const addGames = (games: NewGameInput[]) => call("add_games", { games });
export const getRunningGames = () => call("get_running_games");
export const launchGame = (gameId: number) => call("launch_game", { gameId });
export const openInstallFolder = (gameId: number) => call("open_install_folder", { gameId });

export const listSaveLocations = (gameId: number) => call("list_save_locations", { gameId });
export const detectSaves = (gameId: number) => call("detect_saves", { gameId });
export const addSaveFolder = (gameId: number, folder: string) => call("add_save_folder", { gameId, folder });
export const removeSaveLocation = (gameId: number, locationId: number) =>
  call("remove_save_location", { gameId, locationId });
export const listSnapshots = (gameId: number | null) => call("list_snapshots", { gameId });
export const getSaveStatuses = () => call("get_save_statuses");
export const getCurrentSave = (gameId: number) => call("get_current_save", { gameId });
export const createSnapshot = (
  gameId: number,
  name: string,
  description: string | null,
  pinned: boolean,
  storage: SnapshotStorage = "both",
) => call("create_snapshot", { gameId, name, description, pinned, storage });
export const restoreSnapshot = (snapshotId: string) => call("restore_snapshot", { snapshotId });
export const deleteSnapshot = (snapshotId: string, alsoFromDrive = false) =>
  call("delete_snapshot", { snapshotId, alsoFromDrive });
export const verifySnapshot = (snapshotId: string) => call("verify_snapshot", { snapshotId });
export const setSnapshotPinned = (snapshotId: string, pinned: boolean) =>
  call("set_snapshot_pinned", { snapshotId, pinned });
export const revealSnapshot = (snapshotId: string | null) => call("reveal_snapshot", { snapshotId });
export const getSnapshotFolder = () => call("get_snapshot_folder");
export const findSaves = (gameId: number) => call("find_saves", { gameId });
export const openWikiPage = (url: string) => call("open_wiki_page", { url });

export const scanBackupFolder = (folder: string, mode: ScanMode | null = null) =>
  call("scan_backup_folder", { folder, mode });
export const scanPcForLeftoverSaves = () => call("scan_pc_for_leftover_saves");
export const exportAllSaves = (folder: string) => call("export_all_saves", { folder });
export const replanBackupGame = (
  folder: string,
  key: string,
  fromCollection: string | null,
  layout: PlannedLayout,
) => call("replan_backup_game", { folder, key, fromCollection, layout });
export const importBackupGames = (games: ImportGameRequest[], storage: SnapshotStorage | null = null) =>
  call("import_backup_games", { games, storage });
export const getRestoreTargets = (snapshotId: string) => call("get_restore_targets", { snapshotId });
export const restoreSnapshotTo = (snapshotId: string, target: string) =>
  call("restore_snapshot_to", { snapshotId, target });
export const renameSnapshot = (snapshotId: string, label: string | null) =>
  call("rename_snapshot", { snapshotId, label });

export const getArtworkKeyStatus = () => call("get_artwork_key_status");
/** Ids of the games whose automatic artwork lookup is still queued or running. */
export const getArtworkPending = () => call("get_artwork_pending");
export const saveArtworkKey = (key: string) => call("save_artwork_key", { key });
export const clearArtworkKey = () => call("clear_artwork_key");
export const verifyArtworkKey = () => call("verify_artwork_key");
export const getArtworkSelections = (gameId: number) => call("get_artwork_selections", { gameId });
export const findArtwork = (gameId: number, kind: ArtworkKind, query: string | null) =>
  call("find_artwork", { gameId, kind, query });
export const applyArtwork = (gameId: number, candidate: ArtworkCandidate) =>
  call("apply_artwork", { gameId, candidate });
export const importArtwork = (gameId: number, kind: ArtworkKind, path: string) =>
  call("import_artwork", { gameId, kind, path });
export const resetArtwork = (gameId: number, kind: ArtworkKind) => call("reset_artwork", { gameId, kind });
export const fetchMissingArtwork = () => call("fetch_missing_artwork");
export const updateGameMetadata = (
  gameId: number,
  title: string,
  developer: string | null,
  releaseYear: number | null,
) => call("update_game_metadata", { gameId, title, developer, releaseYear });
export const openArtworkKeyPage = () => call("open_artwork_key_page");

export const getCloudStatus = () => call("get_cloud_status");
export const getCloudQuota = () => call("get_cloud_quota");
export const saveGoogleClient = (clientId: string, clientSecret: string) =>
  call("save_google_client", { clientId, clientSecret });
export const clearGoogleClient = () => call("clear_google_client");
export const connectGoogleDrive = () => call("connect_google_drive");
export const disconnectGoogleDrive = () => call("disconnect_google_drive");
export const syncCloudNow = () => call("sync_cloud_now");
export const listSyncOperations = () => call("list_sync_operations");
export const retrySync = () => call("retry_sync");
export const listConflicts = () => call("list_conflicts");
export const resolveConflict = (conflictId: number, resolution: ConflictResolution) =>
  call("resolve_conflict", { conflictId, resolution });
export const downloadSnapshot = (snapshotId: string) => call("download_snapshot", { snapshotId });
