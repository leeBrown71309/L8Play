import { useLingui } from "@lingui/react/macro";
import { useState } from "react";

import { TextField } from "@/components/ds/form-fields";
import { StatusBadge } from "@/components/ds/status-badge";
import { notify } from "@/components/feedback/toast-store";
import { Button } from "@/components/ui/button";
import {
  useArtworkErrorText,
  useArtworkKeyStatus,
  useClearArtworkKey,
  useFetchMissingArtwork,
  useSaveArtworkKey,
  useVerifyArtworkKey,
} from "@/features/artwork/use-artwork";
import { SettingGroup, SettingRow } from "@/features/settings/setting-blocks";
import { openArtworkKeyPage } from "@/lib/tauri/commands";

/** SteamGridDB key (kept in the Windows Credential Manager) and the "find missing artwork" action. */
export function ArtworkSection() {
  const { t } = useLingui();
  const errorText = useArtworkErrorText();
  const status = useArtworkKeyStatus();
  const save = useSaveArtworkKey();
  const clear = useClearArtworkKey();
  const verify = useVerifyArtworkKey();
  const fetchMissing = useFetchMissingArtwork();
  const [draft, setDraft] = useState("");
  const configured = status.data === true;

  const submit = () =>
    save.mutate(draft, {
      onSuccess: () => {
        // The field is emptied at once: the key now lives only in the Credential Manager.
        setDraft("");
        notify({ tone: "success", title: t`API key saved`, body: t`Stored in the Windows Credential Manager.` });
      },
      onError: (error) => notify({ tone: "danger", title: t`The key was not saved`, body: errorText(error) }),
    });

  return (
    <SettingGroup>
      <div className="st-card">
        <div className="head">
          SteamGridDB
          {configured && <StatusBadge status="created" size="sm">{t`Key saved`}</StatusBadge>}
        </div>
        <p>
          {t`Optional and free. It adds covers, heroes and logos for Epic and standalone games, and lets you replace any image. Without a key, Steam games keep Steam’s artwork and other games get generated placeholders.`}
        </p>

        {configured ? (
          <div className="keyrow">
            <Button
              size="sm"
              icon="scan"
              loading={verify.isPending}
              onClick={() =>
                verify.mutate(undefined, {
                  onSuccess: () => notify({ tone: "success", title: t`The key works` }),
                  onError: (error) => notify({ tone: "danger", title: t`The key was refused`, body: errorText(error) }),
                })
              }
            >
              {t`Test the key`}
            </Button>
            <Button
              size="sm"
              variant="ghost"
              disabled={clear.isPending}
              onClick={() =>
                clear.mutate(undefined, {
                  onSuccess: () => notify({ tone: "neutral", title: t`API key removed` }),
                  onError: (error) =>
                    notify({ tone: "danger", title: t`The key was not removed`, body: errorText(error) }),
                })
              }
            >
              {t`Remove the key`}
            </Button>
          </div>
        ) : (
          <form
            onSubmit={(event) => {
              event.preventDefault();
              if (draft.trim() !== "") submit();
            }}
          >
            <ol>
              <li>{t`Create a free account on steamgriddb.com.`}</li>
              <li>{t`Open Preferences › API and copy your key.`}</li>
              <li>{t`Paste it here. It never leaves this PC except to ask SteamGridDB for images.`}</li>
            </ol>
            <div className="keyrow">
              <TextField
                type="password"
                autoComplete="off"
                spellCheck={false}
                mono
                value={draft}
                onChange={(event) => setDraft(event.target.value)}
                aria-label={t`SteamGridDB API key`}
                placeholder={t`Paste your API key`}
              />
              <Button type="submit" variant="primary" disabled={draft.trim() === "" || save.isPending}>
                {t`Save key`}
              </Button>
              <Button variant="ghost" onClick={() => void openArtworkKeyPage()}>
                {t`Open the key page`}
              </Button>
            </div>
          </form>
        )}
      </div>

      <SettingRow
        title={t`Find missing artwork`}
        description={t`Looks up cover, hero and logo for Epic and standalone games. This also runs when the app starts.`}
        control={
          <Button
            disabled={!configured}
            icon="scan"
            loading={fetchMissing.isPending}
            onClick={() =>
              fetchMissing.mutate(undefined, {
                onSuccess: (report) =>
                  notify({
                    tone: report.fetched > 0 ? "success" : "neutral",
                    title: report.fetched > 0 ? t`Artwork found` : t`Nothing new found`,
                    body: t`${report.fetched} added · ${report.notFound} not found · ${report.failed} failed`,
                  }),
                onError: (error) => notify({ tone: "danger", title: t`The lookup stopped`, body: errorText(error) }),
              })
            }
          >
            {t`Find now`}
          </Button>
        }
      />
    </SettingGroup>
  );
}
