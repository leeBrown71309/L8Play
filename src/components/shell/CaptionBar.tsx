import { useLingui } from "@lingui/react/macro";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { ReactNode } from "react";

import { isTauri } from "@/lib/tauri/runtime";

type WindowAction = "minimize" | "toggleMaximize" | "close";

async function runWindowAction(action: WindowAction): Promise<void> {
  if (!isTauri()) return;
  try {
    await getCurrentWindow()[action]();
  } catch (error) {
    console.error(`Window action "${action}" failed.`, error);
  }
}

const GLYPH = { fill: "none", stroke: "currentColor", strokeWidth: 1.25, strokeLinecap: "round" } as const;

function CaptionGlyph({ size, children }: { size: number; children: ReactNode }) {
  return (
    <svg className="l8-icon" width={size} height={size} viewBox="0 0 24 24" aria-hidden="true" {...GLYPH}>
      {children}
    </svg>
  );
}

/** Custom Windows caption strip (`l8-capbar`): drag area + minimize / maximize / close. */
export function CaptionBar() {
  const { t } = useLingui();
  return (
    <div data-tauri-drag-region className="l8-capbar" onDoubleClick={() => void runWindowAction("toggleMaximize")}>
      <div className="l8-caption" onDoubleClick={(event) => event.stopPropagation()}>
        <button
          type="button"
          aria-label={t`Minimize`}
          title={t`Minimize`}
          onClick={() => void runWindowAction("minimize")}
        >
          <CaptionGlyph size={16}>
            <path d="M5 12h14" />
          </CaptionGlyph>
        </button>
        <button
          type="button"
          aria-label={t`Maximize`}
          title={t`Maximize`}
          onClick={() => void runWindowAction("toggleMaximize")}
        >
          <CaptionGlyph size={14}>
            <rect x="5" y="5" width="14" height="14" rx="2" />
          </CaptionGlyph>
        </button>
        <button
          type="button"
          aria-label={t`Close`}
          title={t`Close`}
          className="is-close"
          onClick={() => void runWindowAction("close")}
        >
          <CaptionGlyph size={16}>
            <path d="m6 6 12 12M18 6 6 18" />
          </CaptionGlyph>
        </button>
      </div>
    </div>
  );
}
