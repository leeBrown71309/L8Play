import { useLingui } from "@lingui/react/macro";
import { useState } from "react";

import { TextField } from "@/components/ds/form-fields";
import { notify } from "@/components/feedback/toast-store";
import { Button } from "@/components/ui/button";
import { useClearGoogleClient, useCloudErrorText, useSaveGoogleClient } from "@/features/cloud/use-cloud";
import type { ClientSource } from "@/lib/bindings/ClientSource";

interface AdvancedClientProps {
  source: ClientSource;
  connected: boolean;
}

/** Advanced: use your own Google Cloud project instead of the client built into the app. */
export function AdvancedClient({ source, connected }: AdvancedClientProps) {
  const { t } = useLingui();
  const errorText = useCloudErrorText();
  const save = useSaveGoogleClient();
  const clear = useClearGoogleClient();
  const [clientId, setClientId] = useState("");
  const [clientSecret, setClientSecret] = useState("");
  const ready = clientId.trim() !== "" && clientSecret.trim() !== "";

  const submit = () =>
    save.mutate(
      { clientId, clientSecret },
      {
        onSuccess: () => {
          // Emptied at once: from here on the secret only lives in the Credential Manager.
          setClientId("");
          setClientSecret("");
          notify({
            tone: "success",
            title: t`Google client saved`,
            body: t`Stored in the Windows Credential Manager.`,
          });
        },
        onError: (error) => notify({ tone: "danger", title: t`The client was not saved`, body: errorText(error) }),
      },
    );

  return (
    <details open={source === "none"} className="cloud-advanced">
      <summary>{t`Advanced: use my own Google project`}</summary>
      <div className="body">
        <p className="help">
          {source === "custom"
            ? t`Sign-in currently uses your own Google client.`
            : source === "embedded"
              ? t`Sign-in currently uses the Google client built into L8Play. You only need your own if you prefer to keep everything in a Google Cloud project you control.`
              : t`This build of L8Play has no built-in Google client: create your own free Google Cloud project and add its client below.`}{" "}
          {t`See docs/google-drive-setup.md for the steps. No billing account is ever needed.`}
        </p>
        {connected ? (
          <Banner>{t`Disconnect Google Drive first to change the client. Your snapshots and Drive files are not touched.`}</Banner>
        ) : (
          <form
            className="body"
            style={{ marginTop: 0 }}
            onSubmit={(event) => {
              event.preventDefault();
              if (ready) submit();
            }}
          >
            <TextField
              label={t`OAuth client id`}
              value={clientId}
              onChange={(event) => setClientId(event.target.value)}
              autoComplete="off"
              spellCheck={false}
              mono
              placeholder={t`Client id (….apps.googleusercontent.com)`}
            />
            <TextField
              label={t`OAuth client secret`}
              type="password"
              value={clientSecret}
              onChange={(event) => setClientSecret(event.target.value)}
              autoComplete="off"
              mono
              placeholder={t`Client secret`}
            />
            <div className="actions">
              <Button type="submit" variant="primary" disabled={!ready || save.isPending}>
                {t`Save my client`}
              </Button>
              {source === "custom" && (
                <Button
                  variant="ghost"
                  disabled={clear.isPending}
                  onClick={() =>
                    clear.mutate(undefined, {
                      onSuccess: () => notify({ tone: "neutral", title: t`Back to the default Google client` }),
                      onError: (error) =>
                        notify({ tone: "danger", title: t`The client was not removed`, body: errorText(error) }),
                    })
                  }
                >
                  {t`Use the default client`}
                </Button>
              )}
            </div>
          </form>
        )}
      </div>
    </details>
  );
}

function Banner({ children }: { children: string }) {
  return <p className="cloud-note">{children}</p>;
}
