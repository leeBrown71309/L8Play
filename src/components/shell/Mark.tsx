import { useId } from "react";

/** The L8Play mark: two stacked save slots forming an 8, a play triangle and a cream "saved" dot on a coral tile. */
export function Mark({ size = 32 }: { size?: number }) {
  const id = useId();
  const tile = `${id}-tile`;
  const glow = `${id}-glow`;
  return (
    <svg width={size} height={size} viewBox="0 0 512 512" aria-hidden="true" className="l8-icon shrink-0">
      <defs>
        <linearGradient id={tile} x1="0.15" y1="0" x2="0.85" y2="1">
          <stop offset="0" stopColor="#f0695c" />
          <stop offset="0.55" stopColor="#cf4239" />
          <stop offset="1" stopColor="#a02d26" />
        </linearGradient>
        <radialGradient id={glow} cx="0.28" cy="0.18" r="0.8">
          <stop offset="0" stopColor="#ffffff" stopOpacity="0.28" />
          <stop offset="1" stopColor="#ffffff" stopOpacity="0" />
        </radialGradient>
      </defs>
      <rect width="512" height="512" rx="116" fill={`url(#${tile})`} />
      <rect width="512" height="512" rx="116" fill={`url(#${glow})`} />
      <rect x="150" y="92" width="212" height="158" rx="52" fill="none" stroke="#fff" strokeWidth="30" />
      <rect x="124" y="262" width="264" height="158" rx="56" fill="none" stroke="#fff" strokeWidth="30" />
      <path d="M236 142v58l50-29z" fill="#fff" stroke="#fff" strokeWidth="10" strokeLinejoin="round" />
      <circle cx="256" cy="341" r="20" fill="#f6e3a6" />
    </svg>
  );
}
