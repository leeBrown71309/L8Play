/** TanStack Query keys, in one place so backend events can invalidate them precisely. */
export const queryKeys = {
  settings: ["settings"] as const,
  appInfo: ["app-info"] as const,
  games: ["games"] as const,
  game: (gameId: number) => ["games", gameId] as const,
  runningGames: ["running-games"] as const,
  /** Installed games a saves-only game could be joined to. */
  linkCandidates: (gameId: number) => ["games", gameId, "link-candidates"] as const,
  /** What the last owned-but-not-installed Steam lookup found (settings hint). */
  ownedLibraryStatus: ["owned-library-status"] as const,
  /** Whether a Steam Web API key is saved (never the key itself). */
  steamKey: ["steam-key"] as const,
  /** Everything about saves; invalidating it refreshes locations, snapshots and statuses. */
  saves: ["saves"] as const,
  saveLocations: (gameId: number) => ["saves", "locations", gameId] as const,
  snapshots: (gameId: number | null) => ["saves", "snapshots", gameId ?? "all"] as const,
  saveStatuses: ["saves", "statuses"] as const,
  currentSave: (gameId: number) => ["saves", "current", gameId] as const,
  snapshotFolder: ["saves", "folder"] as const,
  /** Everything about Google Drive; invalidated by cloud events. */
  cloud: ["cloud"] as const,
  cloudStatus: ["cloud", "status"] as const,
  cloudQuota: ["cloud", "quota"] as const,
  cloudOperations: ["cloud", "operations"] as const,
  cloudConflicts: ["cloud", "conflicts"] as const,
  /** Artwork key status, selections and provider candidates. */
  artwork: ["artwork"] as const,
  artworkKey: ["artwork", "key"] as const,
  artworkSelections: (gameId: number) => ["artwork", "selections", gameId] as const,
  artworkCandidates: (gameId: number, kind: string, term: string) =>
    ["artwork", "candidates", gameId, kind, term] as const,
};
