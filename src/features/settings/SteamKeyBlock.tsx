import { useLingui } from "@lingui/react/macro";
import { useState } from "react";

import { TextField } from "@/components/ds/form-fields";
import { StatusBadge } from "@/components/ds/status-badge";
import { notify } from "@/components/feedback/toast-store";
import { Button } from "@/components/ui/button";
import { useClearSteamKey, useSaveSteamKey, useSteamKeyStatus } from "@/features/settings/use-owned-library";
import type { OwnedLibraryStatus } from "@/lib/bindings/OwnedLibraryStatus";
import { openSteamApiKeyPage } from "@/lib/tauri/commands";
import { toAppError } from "@/lib/tauri/errors";

interface SteamKeyBlockProps {
  status: OwnedLibraryStatus;
  /** Called after the key was saved or removed, to list the owned games again. */
  onChanged: () => void;
}

/**
 * The user's own free Steam Web API key (kept in the Windows Credential Manager). Shown when Steam asks
 * for one or refused it, or when the user opens it. The key is typed here once and never shown again.
 */
export function SteamKeyBlock({ status, onChanged }: SteamKeyBlockProps) {
  const { t } = useLingui();
  const keySaved = useSteamKeyStatus().data === true;
  const save = useSaveSteamKey();
  const clear = useClearSteamKey();
  const [opened, setOpened] = useState(false);
  const [draft, setDraft] = useState("");
  const asked = status === "needs_key" || status === "invalid_key";

  if (!opened && !asked) {
    return (
      <div className="st-row">
        <div className="grow">
          <div className="t">{t`Steam Web API key`}</div>
          <div className="d">
            {keySaved ? t`A key is saved.` : t`Optional: lets Steam list your games to this app.`}
          </div>
        </div>
        <div className="controls">
          <Button size="sm" onClick={() => setOpened(true)}>
            {keySaved ? t`Manage the key` : t`Use my Steam Web API key`}
          </Button>
        </div>
      </div>
    );
  }

  const submit = () =>
    save.mutate(draft, {
      onSuccess: () => {
        // Emptied at once: the key now lives only in the Credential Manager.
        setDraft("");
        notify({ tone: "success", title: t`Steam key saved`, body: t`Stored in the Windows Credential Manager.` });
        onChanged();
      },
      onError: (error) => notify({ tone: "danger", title: t`The key was not saved`, body: toAppError(error).message }),
    });

  return (
    <div className="st-card">
      <div className="head">
        {t`Steam Web API key`}
        {keySaved && (
          <StatusBadge status="created" size="sm">
            {t`Key saved`}
          </StatusBadge>
        )}
        {status === "invalid_key" && <StatusBadge status="error" size="sm">{t`Steam refused the key`}</StatusBadge>}
        {status === "needs_key" && <StatusBadge status="unprotected" size="sm">{t`A key is needed`}</StatusBadge>}
        {status === "listed" && keySaved && <StatusBadge status="verified" size="sm">{t`Games listed`}</StatusBadge>}
        {status === "private" && (
          <StatusBadge status="unprotected" size="sm">{t`Game details are private`}</StatusBadge>
        )}
      </div>
      <p>
        {t`Steam no longer lists your games to apps without a key. Create a free key at steamcommunity.com/dev/apikey (any domain name works, no payment) and paste it here. It is stored in the Windows Credential Manager.`}
      </p>
      <form
        onSubmit={(event) => {
          event.preventDefault();
          if (draft.trim() !== "") submit();
        }}
      >
        <div className="keyrow">
          <TextField
            type="password"
            autoComplete="off"
            spellCheck={false}
            mono
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            aria-label={t`Steam Web API key`}
            placeholder={t`Paste your Steam Web API key`}
          />
          <Button type="submit" variant="primary" disabled={draft.trim() === "" || save.isPending}>
            {t`Save key`}
          </Button>
          {keySaved && (
            <Button
              variant="ghost"
              disabled={clear.isPending}
              onClick={() =>
                clear.mutate(undefined, {
                  onSuccess: () => {
                    notify({ tone: "neutral", title: t`Steam key removed` });
                    onChanged();
                  },
                  onError: (error) =>
                    notify({ tone: "danger", title: t`The key was not removed`, body: toAppError(error).message }),
                })
              }
            >
              {t`Remove the key`}
            </Button>
          )}
          <Button variant="ghost" onClick={() => void openSteamApiKeyPage()}>
            {t`Open the key page`}
          </Button>
        </div>
      </form>
    </div>
  );
}
