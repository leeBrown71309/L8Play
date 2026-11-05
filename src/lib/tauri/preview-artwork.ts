import type { ArtworkCandidate } from "@/lib/bindings/ArtworkCandidate";
import type { ArtworkKind } from "@/lib/bindings/ArtworkKind";

const SIZES: Record<ArtworkKind, [number, number]> = {
  cover: [600, 900],
  hero: [1920, 620],
  logo: [800, 310],
  icon: [256, 256],
};

/** Colored placeholder drawn as an inline SVG, so the preview never needs the network. */
function placeholder(label: string, width: number, height: number, hue: number): string {
  const svg =
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${width} ${height}">` +
    `<rect width="100%" height="100%" fill="hsl(${hue} 50% 32%)"/>` +
    `<text x="50%" y="50%" fill="white" font-family="sans-serif" font-size="${Math.round(height / 9)}" ` +
    `text-anchor="middle" dominant-baseline="middle">${label}</text></svg>`;
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

/** Sample SteamGridDB results for the browser preview. */
export function sampleCandidates(kind: ArtworkKind, query: string): ArtworkCandidate[] {
  const [width, height] = SIZES[kind];
  // Enough results to make the grid scroll, like a real SteamGridDB search.
  return Array.from({ length: kind === "hero" ? 14 : 24 }, (_, index) => {
    const image = placeholder(`${query} #${index + 1}`, width, height, 340 + index * 14);
    return {
      id: `preview-${kind}-${index}`,
      kind,
      url: image,
      thumbUrl: image,
      width,
      height,
      style: index % 2 === 0 ? "alternate" : "blurred",
      author: ["Ann", "Bob", "Cleo"][index % 3] ?? null,
      mime: "image/png",
    };
  });
}
