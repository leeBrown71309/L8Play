import { useLingui } from "@lingui/react/macro";
import { useState } from "react";

import { notify } from "@/components/feedback/toast-store";
import { TextField } from "@/components/ds/form-fields";
import { PageHeader } from "@/components/ds/page-blocks";
import { Button } from "@/components/ui/button";
import { useArtworkErrorText, useUpdateGameMetadata } from "@/features/artwork/use-artwork";
import type { GameDetail } from "@/lib/bindings/GameDetail";
import { formatDateTime } from "@/lib/format";

const MAX_TEXT = 120;

/** Title, developer and release year. Saved edits are never overwritten by a library scan. */
export function PropertiesPanel({ detail }: { detail: GameDetail }) {
  const { t, i18n } = useLingui();
  const errorText = useArtworkErrorText();
  const update = useUpdateGameMetadata();
  const { game } = detail;
  const [title, setTitle] = useState(game.title);
  const [developer, setDeveloper] = useState(detail.developer ?? "");
  const [year, setYear] = useState(detail.releaseYear === null ? "" : String(detail.releaseYear));

  const parsedYear = year.trim() === "" ? null : Number(year);
  const yearValid = parsedYear === null || (Number.isInteger(parsedYear) && parsedYear >= 1970 && parsedYear <= 2100);
  const valid = title.trim() !== "" && yearValid;
  const dirty =
    title.trim() !== game.title || developer.trim() !== (detail.developer ?? "") || parsedYear !== detail.releaseYear;

  const save = () =>
    update.mutate(
      { gameId: game.id, title, developer: developer.trim() === "" ? null : developer.trim(), releaseYear: parsedYear },
      {
        onSuccess: () => notify({ tone: "success", title: t`Properties saved`, body: title.trim() }),
        onError: (error) => notify({ tone: "danger", title: t`Properties were not saved`, body: errorText(error) }),
      },
    );

  const facts: [string, string][] = [
    [t`Sessions recorded`, String(detail.sessionCount)],
    [t`Added`, formatDateTime(detail.addedAt, i18n.locale)],
    ...(game.installPath ? ([[t`Install folder`, game.installPath]] as [string, string][]) : []),
    ...(detail.executablePath ? ([[t`Executable`, detail.executablePath]] as [string, string][]) : []),
  ];

  return (
    <div className="gd-props">
      <PageHeader
        title={t`Properties`}
        subtitle={t`Correct a name that was read wrongly. Your edits are kept when libraries are scanned again.`}
      />
      <form
        onSubmit={(event) => {
          event.preventDefault();
          if (valid && dirty) save();
        }}
      >
        <TextField
          label={t`Title`}
          value={title}
          maxLength={MAX_TEXT}
          onChange={(event) => setTitle(event.target.value)}
        />
        <TextField
          label={t`Developer`}
          value={developer}
          maxLength={MAX_TEXT}
          onChange={(event) => setDeveloper(event.target.value)}
        />
        <TextField
          label={t`Release year`}
          inputMode="numeric"
          value={year}
          className="max-w-[180px]"
          error={yearValid ? undefined : t`Enter a year between 1970 and 2100.`}
          onChange={(event) => setYear(event.target.value.replace(/\D/g, "").slice(0, 4))}
        />
        <div className="flex gap-2">
          <Button type="submit" variant="primary" disabled={!valid || !dirty || update.isPending}>
            {update.isPending ? t`Saving…` : t`Save`}
          </Button>
          <Button
            variant="ghost"
            disabled={!dirty}
            onClick={() => {
              setTitle(game.title);
              setDeveloper(detail.developer ?? "");
              setYear(detail.releaseYear === null ? "" : String(detail.releaseYear));
            }}
          >
            {t`Discard changes`}
          </Button>
        </div>
      </form>
      <dl className="gd-dl mt-8">
        {facts.map(([label, value]) => (
          <div key={label}>
            <dt>{label}</dt>
            <dd className="break-all">{value}</dd>
          </div>
        ))}
      </dl>
    </div>
  );
}
