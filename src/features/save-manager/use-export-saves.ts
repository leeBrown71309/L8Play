import { useLingui } from "@lingui/react/macro";
import { useMutation } from "@tanstack/react-query";

import { notify } from "@/components/feedback/toast-store";
import { useFolderPicker } from "@/features/saves/use-folder-picker";
import { formatBytes } from "@/lib/format";
import { exportAllSaves } from "@/lib/tauri/commands";

/** "Export all saves": asks for a folder, then writes every snapshot this PC holds into one zip there. */
export function useExportAllSaves() {
  const { t, i18n } = useLingui();
  const pickFolder = useFolderPicker();
  const mutation = useMutation({
    mutationFn: (folder: string) => exportAllSaves(folder),
    onSuccess: (report) => {
      const size = formatBytes(report.bytes, i18n.locale);
      const summary = t`${report.snapshots} snapshots from ${report.games} games · ${size}`;
      notify({
        tone: report.skipped > 0 ? "warning" : "success",
        title: t`Saves exported`,
        body:
          report.skipped > 0
            ? t`${summary}. ${report.skipped} snapshots only exist in Google Drive and were left out.`
            : `${summary}\n${report.path}`,
      });
    },
    onError: (error) => {
      console.error("Export of all saves failed.", error);
      notify({ tone: "danger", title: t`The saves could not be exported` });
    },
  });

  return {
    exporting: mutation.isPending,
    run: async () => {
      const folder = await pickFolder(t`Choose where to save the export`);
      if (folder) mutation.mutate(folder);
    },
  };
}
