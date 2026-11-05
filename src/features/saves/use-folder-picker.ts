import { useLingui } from "@lingui/react/macro";
import { open } from "@tauri-apps/plugin-dialog";

import { notify } from "@/components/feedback/toast-store";
import { isTauri } from "@/lib/tauri/runtime";

const PREVIEW_FOLDER = "D:/Workspace/Games";

/** Native folder picker; resolves to `null` when cancelled or unavailable (with a toast saying why). */
export function useFolderPicker() {
  const { t } = useLingui();
  return async (title: string, defaultPath?: string): Promise<string | null> => {
    if (!isTauri()) {
      // Design preview (vite dev server only): a sample folder lets the folder flows be reviewed in a browser.
      if (import.meta.env.DEV) return PREVIEW_FOLDER;
      notify({ tone: "neutral", title: t`Choosing a folder needs the desktop app` });
      return null;
    }
    try {
      const picked = await open({ multiple: false, directory: true, title, ...(defaultPath ? { defaultPath } : {}) });
      return typeof picked === "string" ? picked : null;
    } catch (error) {
      console.error("Folder picker failed.", error);
      notify({ tone: "danger", title: t`The folder picker could not be opened` });
      return null;
    }
  };
}
