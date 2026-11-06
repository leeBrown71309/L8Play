import type { GameSummary } from "@/lib/bindings/GameSummary";
import type { Platform } from "@/lib/bindings/Platform";

/**
 * Sample library for the browser preview only (no Tauri backend). Real libraries come from the
 * user's Steam, Epic, Xbox app and GOG installations.
 */
const STEAM_CDN = "https://cdn.cloudflare.steamstatic.com/steam/apps";

function steamArt(appId: string): GameSummary["artwork"] {
  return {
    cover: `${STEAM_CDN}/${appId}/library_600x900.jpg`,
    hero: `${STEAM_CDN}/${appId}/library_hero.jpg`,
    logo: `${STEAM_CDN}/${appId}/logo.png`,
    header: `${STEAM_CDN}/${appId}/header.jpg`,
    icon: null,
  };
}

const NO_ART: GameSummary["artwork"] = { cover: null, hero: null, logo: null, header: null, icon: null };
const hoursAgo = (hours: number) => new Date(Date.now() - hours * 3_600_000).toISOString();

interface SampleGame {
  title: string;
  platform: Platform;
  id: string | null;
  playedHoursAgo: number | null;
  playtimeHours: number;
  sizeGb: number;
  installed?: boolean;
  favorite?: boolean;
}

const SAMPLES: SampleGame[] = [
  {
    title: "Sekiro: Shadows Die Twice",
    platform: "steam",
    id: "814380",
    playedHoursAgo: 0.5,
    playtimeHours: 64,
    sizeGb: 25.1,
    favorite: true,
  },
  {
    title: "Monster Hunter: World",
    platform: "steam",
    id: "582010",
    playedHoursAgo: 22,
    playtimeHours: 412,
    sizeGb: 48.6,
    favorite: true,
  },
  {
    title: "Stardew Valley",
    platform: "steam",
    id: "413150",
    playedHoursAgo: 26,
    playtimeHours: 310,
    sizeGb: 0.6,
    favorite: true,
  },
  { title: "Hades II", platform: "steam", id: "1145350", playedHoursAgo: 69, playtimeHours: 52, sizeGb: 11.8 },
  { title: "ELDEN RING", platform: "steam", id: "1245620", playedHoursAgo: 120, playtimeHours: 188, sizeGb: 49.2 },
  { title: "Celeste", platform: "steam", id: "504230", playedHoursAgo: 150, playtimeHours: 38, sizeGb: 1.2 },
  { title: "Baldur's Gate 3", platform: "steam", id: "1086940", playedHoursAgo: 220, playtimeHours: 233, sizeGb: 122 },
  { title: "Cyberpunk 2077", platform: "steam", id: "1091500", playedHoursAgo: 340, playtimeHours: 96, sizeGb: 70.4 },
  {
    title: "Hollow Knight",
    platform: "steam",
    id: "367520",
    playedHoursAgo: 1100,
    playtimeHours: 61,
    sizeGb: 9.1,
    installed: false,
  },
  { title: "Alan Wake 2", platform: "epic", id: "Fennec", playedHoursAgo: 70, playtimeHours: 18, sizeGb: 88.5 },
  {
    title: "Death Stranding Director's Cut",
    platform: "epic",
    id: "Kestrel",
    playedHoursAgo: null,
    playtimeHours: 0,
    sizeGb: 76,
  },
  {
    title: "Minecraft Dungeons II",
    platform: "xbox",
    id: "Microsoft.MinecraftDungeons2_8wekyb3d8bbwe!AppMinecraftDungeonsIIShipping",
    playedHoursAgo: 5,
    playtimeHours: 14,
    sizeGb: 12.4,
  },
  { title: "Moonlighter", platform: "gog", id: "2147483047", playedHoursAgo: 190, playtimeHours: 27, sizeGb: 1.1 },
  {
    title: "Minecraft: Java Edition",
    platform: "standalone",
    id: null,
    playedHoursAgo: 96,
    playtimeHours: 520,
    sizeGb: 1.9,
  },
  // Owned but not installed: never played here, so they stay out of "Recently played".
  {
    title: "Disco Elysium",
    platform: "steam",
    id: "632470",
    playedHoursAgo: null,
    playtimeHours: 0,
    sizeGb: 21,
    installed: false,
  },
  {
    title: "Outer Wilds",
    platform: "steam",
    id: "753640",
    playedHoursAgo: null,
    playtimeHours: 0,
    sizeGb: 5.4,
    installed: false,
  },
  {
    title: "Red Dead Redemption 2",
    platform: "steam",
    id: "1174180",
    playedHoursAgo: null,
    playtimeHours: 0,
    sizeGb: 119,
    installed: false,
  },
  {
    title: "Control",
    platform: "epic",
    id: "Calluna",
    playedHoursAgo: null,
    playtimeHours: 0,
    sizeGb: 42,
    installed: false,
  },
];

/**
 * Browser preview only: `?bulk=150` appends that many filler games so pagination can be reviewed
 * with a large library.
 */
function bulkCount(): number {
  if (typeof window === "undefined") return 0;
  const requested = Number(new URLSearchParams(window.location.search).get("bulk"));
  return Number.isInteger(requested) && requested > 0 ? Math.min(requested, 1000) : 0;
}

export function sampleLibrary(): GameSummary[] {
  const bulk: SampleGame[] = Array.from({ length: bulkCount() }, (_, index) => ({
    title: `Sample game ${index + 1}`,
    platform: (["steam", "epic", "xbox", "gog", "standalone"] as const)[index % 5] ?? "standalone",
    id: null,
    playedHoursAgo: null,
    playtimeHours: index % 40,
    sizeGb: 1 + (index % 30),
    installed: index % 3 !== 0,
  }));
  return [...SAMPLES, ...bulk].map((sample, index) => ({
    id: index + 1,
    title: sample.title,
    platform: sample.platform,
    platformGameId: sample.id,
    installed: sample.installed ?? true,
    archived: false,
    favorite: sample.favorite ?? false,
    lastPlayedAt: sample.playedHoursAgo === null ? null : hoursAgo(sample.playedHoursAgo),
    playtimeSeconds: sample.playtimeHours * 3600,
    sizeBytes: Math.round(sample.sizeGb * 1024 ** 3),
    installPath: sample.platform === "standalone" ? `D:\\Games\\${sample.title}` : `D:\\Library\\${sample.title}`,
    artwork: sample.platform === "steam" && sample.id ? steamArt(sample.id) : NO_ART,
  }));
}
