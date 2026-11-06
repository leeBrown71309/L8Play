import { create } from "zustand";

import type { ImportPlan } from "@/lib/bindings/ImportPlan";
import type { ImportProgress } from "@/lib/bindings/ImportProgress";
import type { ImportReport } from "@/lib/bindings/ImportReport";

/** Which step of the "Import saves" flow is on screen. */
export type ImportStage =
  | { kind: "closed" }
  | { kind: "choose" }
  | { kind: "analyzing"; folder: string }
  | { kind: "review"; plan: ImportPlan }
  | { kind: "importing"; progress: ImportProgress | null }
  | { kind: "done"; report: ImportReport };

interface ImportState {
  stage: ImportStage;
  /** The dialog is hidden but the work goes on: an indicator in the app shows it. */
  minimized: boolean;
  minimize: () => void;
  restore: () => void;
  open: () => void;
  analyzing: (folder: string) => void;
  review: (plan: ImportPlan) => void;
  importing: () => void;
  progress: (progress: ImportProgress) => void;
  done: (report: ImportReport) => void;
  close: () => void;
}

export const useImportStore = create<ImportState>((set) => ({
  stage: { kind: "closed" },
  minimized: false,
  minimize: () => set({ minimized: true }),
  restore: () => set({ minimized: false }),
  open: () => set({ stage: { kind: "choose" }, minimized: false }),
  analyzing: (folder) => set({ stage: { kind: "analyzing", folder }, minimized: false }),
  // A stage reached while the dialog is hidden stays hidden: the indicator announces it instead.
  review: (plan) => set({ stage: { kind: "review", plan } }),
  importing: () => set({ stage: { kind: "importing", progress: null } }),
  progress: (progress) =>
    set((state) => (state.stage.kind === "importing" ? { stage: { kind: "importing", progress } } : state)),
  done: (report) => set({ stage: { kind: "done", report } }),
  close: () => set({ stage: { kind: "closed" }, minimized: false }),
}));
