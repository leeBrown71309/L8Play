import { useLingui } from "@lingui/react/macro";
import { useEffect, useRef } from "react";

import { DsIcon, type DsIconName } from "@/components/ds/ds-icon";
import { ProgressBar } from "@/components/ds/progress-bar";
import { notify } from "@/components/feedback/toast-store";
import { useImportStore } from "@/features/saves-import/import-store";

/**
 * While the import dialog is hidden, a small card shows what the import is doing and brings the dialog
 * back on click. It also tells the user when the review is ready or the import is over.
 */
export function ImportActivity() {
  const { t } = useLingui();
  const stage = useImportStore((state) => state.stage);
  const minimized = useImportStore((state) => state.minimized);
  const restore = useImportStore((state) => state.restore);
  const previous = useRef(stage.kind);

  useEffect(() => {
    const was = previous.current;
    previous.current = stage.kind;
    if (!minimized || was === stage.kind) return;
    if (stage.kind === "review") notify({ tone: "success", title: t`Your saves are ready to review` });
    if (stage.kind === "done") notify({ tone: "success", title: t`Your saves were imported` });
  }, [stage.kind, minimized, t]);

  if (!minimized || stage.kind === "closed" || stage.kind === "choose") return null;

  let icon: DsIconName = "scan";
  let title = t`Reading your saves…`;
  let hint = t`Click to see the details`;
  let progress: { value: number; meta: string } | null = null;
  switch (stage.kind) {
    case "review":
      icon = "checkCircle";
      title = t`Ready to review`;
      hint = t`Click to choose what to import`;
      break;
    case "importing":
      icon = "snapshot";
      title = t`Importing your saves…`;
      if (stage.progress && stage.progress.total > 0) {
        progress = {
          value: (stage.progress.done / stage.progress.total) * 100,
          meta: `${stage.progress.done} / ${stage.progress.total}`,
        };
      }
      break;
    case "done":
      icon = "check";
      title = t`Import finished`;
      hint = t`Click to see the result`;
      break;
    default:
  }
  const working = stage.kind === "analyzing" || stage.kind === "importing";

  return (
    <button
      type="button"
      onClick={restore}
      className="fixed bottom-5 left-1/2 z-40 flex w-[320px] -translate-x-1/2 cursor-pointer flex-col gap-2 rounded-lg border-0 bg-surface-300 p-3 text-left text-ink shadow-lg"
      aria-label={`${title}. ${hint}`}
    >
      <span className="flex items-center gap-2.5">
        <span className={working ? "animate-pulse" : undefined}>
          <DsIcon name={icon} size={18} />
        </span>
        <span className="grid min-w-0 flex-1">
          <b className="truncate text-[13px]">{title}</b>
          <span className="truncate text-[12px] text-ink-muted">{hint}</span>
        </span>
      </span>
      {working ? (
        <ProgressBar size="sm" indeterminate={progress === null} value={progress?.value ?? 0} meta={progress?.meta} />
      ) : null}
    </button>
  );
}
