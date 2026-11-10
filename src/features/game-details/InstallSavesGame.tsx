import { useLingui } from "@lingui/react/macro";
import { open as openFilePicker } from "@tauri-apps/plugin-dialog";
import { useState } from "react";
import { useNavigate } from "react-router";

import { DsIcon } from "@/components/ds/ds-icon";
import { OptionCard } from "@/components/ds/form-fields";
import { notify } from "@/components/feedback/toast-store";
import { usePlatformNames } from "@/components/game/PlatformTag";
import { Button } from "@/components/ui/button";
import { Dialog } from "@/components/ui/dialog";
import { useInstallSavesFromExecutable } from "@/features/library/use-library";
import type { GameSummary } from "@/lib/bindings/GameSummary";
import { openStoreSearch, type Store } from "@/lib/tauri/commands";
import { AppError } from "@/lib/tauri/errors";
import { isTauri } from "@/lib/tauri/runtime";

const STORES: readonly Store[] = ["steam", "epic", "gog", "xbox"];

/** What a saves-only game can become: installed from a store, or joined to a game already on this PC. */
function InstallSavesDialog({ game, onClose }: { game: GameSummary; onClose: () => void }) {
  const { t } = useLingui();
  const names = usePlatformNames();
  const navigate = useNavigate();
  const link = useInstallSavesFromExecutable();

  const openStore = (store: Store) => {
    openStoreSearch(store, game.title)
      .then(() => {
        notify({
          tone: "neutral",
          title: t`Install ${game.title} from the store`,
          body: t`Once it is installed, scan your libraries: L8Play will offer to join these saves to it.`,
        });
        onClose();
      })
      .catch((error: unknown) => {
        console.error("The store could not be opened.", error);
        notify({ tone: "danger", title: t`The store could not be opened` });
      });
  };

  const chooseExecutable = async () => {
    if (!isTauri()) {
      notify({ tone: "neutral", title: t`Linking a game needs the desktop app` });
      return;
    }
    let picked: string | null;
    try {
      picked = await openFilePicker({
        multiple: false,
        directory: false,
        title: t`Choose the game's executable`,
        filters: [{ name: t`Programs`, extensions: ["exe"] }],
      });
    } catch (error) {
      console.error("File picker failed.", error);
      notify({ tone: "danger", title: t`The file picker could not be opened` });
      return;
    }
    if (!picked) return;
    link.mutate(
      { archivedId: game.id, executablePath: picked, title: game.title },
      {
        onSuccess: (joined) => {
          notify({ tone: "success", title: t`Saves joined to ${joined.title}` });
          onClose();
          void navigate(`/games/${joined.id}`, { replace: true });
        },
        onError: (error) =>
          notify({
            tone: "danger",
            title: t`The game could not be linked`,
            body: error instanceof AppError && error.code === "validation" ? error.message : undefined,
          }),
      },
    );
  };

  return (
    <Dialog
      open
      onOpenChange={(isOpen) => !isOpen && onClose()}
      width={560}
      icon={<DsIcon name="download" size={22} />}
      title={t`Install or link ${game.title}`}
      description={t`These saves are kept, but the game itself is not on this PC. Install it from a store you own it on, or link it to a copy you already have.`}
      actions={
        <Button variant="ghost" onClick={onClose}>
          {t`Cancel`}
        </Button>
      }
    >
      <div className="grid gap-2" role="group">
        {STORES.map((store) => (
          <OptionCard
            key={store}
            selected={false}
            icon="cloudDown"
            tone="cloud"
            title={t`Install from ${names[store] ?? store}`}
            description={t`Opens ${names[store] ?? store} on a search for “${game.title}”. You must own the game there.`}
            onClick={() => openStore(store)}
          />
        ))}
        <OptionCard
          selected={false}
          icon="folderOpen"
          tone="local"
          title={t`I already have the game: choose its executable`}
          description={t`For a game installed outside the stores. L8Play adds it to your library and joins these saves to it.`}
          disabled={link.isPending}
          onClick={() => void chooseExecutable()}
        />
      </div>
    </Dialog>
  );
}

/** "Install or link" button for a saves-only game, with its chooser dialog. */
export function InstallSavesButton({ game }: { game: GameSummary }) {
  const { t } = useLingui();
  const [open, setOpen] = useState(false);
  return (
    <>
      <Button variant="primary" size="lg" icon="download" onClick={() => setOpen(true)}>
        {t`Install or link`}
      </Button>
      {open ? <InstallSavesDialog game={game} onClose={() => setOpen(false)} /> : null}
    </>
  );
}
