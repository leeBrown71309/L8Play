import { useLingui } from "@lingui/react/macro";

import { PLATFORM_GLYPH_PATHS } from "@/components/game/platform-glyphs";
import type { Platform } from "@/lib/bindings/Platform";
import { cn } from "@/lib/cn";

/** Platform names for tooltips and screen readers. */
export function usePlatformNames(): Record<string, string> {
  const { t } = useLingui();
  return {
    steam: "Steam",
    epic: "Epic Games",
    xbox: "Xbox",
    gog: "GOG",
    ea: "EA app",
    ubisoft: "Ubisoft Connect",
    battlenet: "Battle.net",
    standalone: t`Standalone`,
  };
}

interface PlatformIconProps {
  platform: Platform;
  size?: number;
  className?: string;
}

/** The store's official mark (monochrome); a generic computer glyph for standalone games. */
export function PlatformIcon({ platform, size = 20, className }: PlatformIconProps) {
  const names = usePlatformNames();
  const name = names[platform] ?? platform;
  const path = PLATFORM_GLYPH_PATHS[platform];
  return (
    <svg
      viewBox="0 0 24 24"
      width={size}
      height={size}
      role="img"
      aria-label={name}
      className={cn("l8-icon", className)}
    >
      <title>{name}</title>
      {path ? (
        <path d={path} fill="currentColor" />
      ) : (
        <g fill="none" stroke="currentColor" strokeWidth={1.75} strokeLinecap="round" strokeLinejoin="round">
          <rect x="3" y="4.5" width="18" height="12" rx="2.5" />
          <path d="M9 20h6M12 16.5V20" />
        </g>
      )}
    </svg>
  );
}

interface PlatformTagProps {
  platform: Platform;
  /** `stage` sits on artwork. */
  tone?: "plain" | "stage";
  className?: string;
}

/** Platform as a round-ended tag holding the store's mark (the name is its tooltip). */
export function PlatformTag({ platform, tone = "plain", className }: PlatformTagProps) {
  return (
    <span className={cn("l8-ptag", `l8-ptag-${tone}`, "h-[28px] w-[34px] justify-center px-0", className)}>
      <PlatformIcon platform={platform} size={18} />
    </span>
  );
}
