import { useLingui } from "@lingui/react/macro";
import { open as openFilePicker } from "@tauri-apps/plugin-dialog";
import { useRef, useState } from "react";
import { Link } from "react-router";

import { ROUTES } from "@/app/routes";
import { DsIcon } from "@/components/ds/ds-icon";
import { Dropdown, SearchField, Tabs } from "@/components/ds/form-fields";
import { Skeleton } from "@/components/feedback/Skeleton";
import { notify } from "@/components/feedback/toast-store";
import { GameArt } from "@/components/game/GameArt";
import { Button } from "@/components/ui/button";
import { Dialog } from "@/components/ui/dialog";
import {
  useApplyArtwork,
  useArtworkCandidates,
  useArtworkErrorText,
  useArtworkKeyStatus,
  useArtworkSelections,
  useImportArtwork,
  useResetArtwork,
} from "@/features/artwork/use-artwork";
import "@/features/artwork/artwork-dialog.css";
import type { ArtworkCandidate } from "@/lib/bindings/ArtworkCandidate";
import type { ArtworkKind } from "@/lib/bindings/ArtworkKind";
import type { GameSummary } from "@/lib/bindings/GameSummary";
import { cn } from "@/lib/cn";
import { toImageSrc } from "@/lib/tauri/image-src";
import { isTauri } from "@/lib/tauri/runtime";

const ALL_STYLES = "all";

type ChosenIds = Partial<Record<ArtworkKind, string>>;

function without(record: ChosenIds, kind: ArtworkKind): ChosenIds {
  return Object.fromEntries(Object.entries(record).filter(([key]) => key !== kind)) as ChosenIds;
}

interface ArtworkDialogProps {
  game: GameSummary;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

/**
 * Artwork manager (design system "Artwork"): live preview of hero + logo, cover and icon on the left;
 * kind tabs, search and candidates on the right. Only the candidate list scrolls.
 */
export function ArtworkDialog({ game, open, onOpenChange }: ArtworkDialogProps) {
  const { t } = useLingui();
  const errorText = useArtworkErrorText();
  const [kind, setKind] = useState<ArtworkKind>("cover");
  const [draft, setDraft] = useState("");
  const [term, setTerm] = useState("");
  const [style, setStyle] = useState(ALL_STYLES);
  // Candidates the user picked in this session (the picker highlights them at once) and the one in flight.
  const [chosen, setChosen] = useState<ChosenIds>({});
  const [pendingId, setPendingId] = useState<string | null>(null);
  const searchForm = useRef<HTMLFormElement>(null);

  const key = useArtworkKeyStatus();
  const selections = useArtworkSelections(game.id);
  const candidates = useArtworkCandidates(game.id, kind, term, open && key.data === true);
  const apply = useApplyArtwork();
  const importer = useImportArtwork();
  const reset = useResetArtwork();

  const selection = selections.data?.find((entry) => entry.kind === kind);
  const source = selection
    ? selection.provider === "user"
      ? t`Your image`
      : selection.userSelected
        ? t`SteamGridDB · your choice`
        : t`SteamGridDB · automatic`
    : game.artwork[kind]
      ? t`Steam`
      : t`Generated placeholder`;

  const all = candidates.data ?? [];
  const styles = [
    ...new Set(all.map((candidate) => candidate.style).filter((value): value is string => Boolean(value))),
  ];
  const shown = style === ALL_STYLES ? all : all.filter((candidate) => candidate.style === style);

  const choose = (candidate: ArtworkCandidate) => {
    const previous = chosen[kind];
    setChosen((value) => ({ ...value, [kind]: candidate.id }));
    setPendingId(candidate.id);
    apply.mutate(
      { gameId: game.id, candidate },
      {
        onSuccess: () => notify({ tone: "success", title: t`Artwork updated`, body: game.title }),
        onError: (error) => {
          setChosen((value) => (previous === undefined ? without(value, kind) : { ...value, [kind]: previous }));
          notify({ tone: "danger", title: t`The image was not applied`, body: errorText(error) });
        },
        onSettled: () => setPendingId(null),
      },
    );
  };

  const upload = async () => {
    if (!isTauri()) {
      notify({ tone: "neutral", title: t`Choosing an image needs the desktop app` });
      return;
    }
    let picked: string | string[] | null;
    try {
      picked = await openFilePicker({
        multiple: false,
        directory: false,
        title: t`Choose an image`,
        filters: [{ name: t`Images`, extensions: ["png", "jpg", "jpeg", "webp", "ico"] }],
      });
    } catch (error) {
      console.error("File picker failed.", error);
      notify({ tone: "danger", title: t`The file picker could not be opened` });
      return;
    }
    if (typeof picked !== "string") return;
    importer.mutate(
      { gameId: game.id, kind, path: picked },
      {
        onSuccess: () => notify({ tone: "success", title: t`Artwork updated`, body: game.title }),
        onError: (error) => notify({ tone: "danger", title: t`The image was not used`, body: errorText(error) }),
      },
    );
  };

  const resetKind = () =>
    reset.mutate(
      { gameId: game.id, kind },
      {
        onSuccess: () => {
          setChosen((value) => without(value, kind));
          notify({ tone: "neutral", title: t`Back to automatic artwork`, body: game.title });
        },
        onError: (error) => notify({ tone: "danger", title: t`Reset failed`, body: errorText(error) }),
      },
    );

  const kinds: { id: ArtworkKind; label: string }[] = [
    { id: "cover", label: t`Cover` },
    { id: "hero", label: t`Hero` },
    { id: "logo", label: t`Logo` },
    { id: "icon", label: t`Icon` },
  ];

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      icon={<DsIcon name="image" size={22} />}
      width={860}
      title={t`Artwork — ${game.title}`}
      description={t`Matched automatically on SteamGridDB. Pick another set if this one is wrong.`}
      hint={
        <>
          <DsIcon name="info" size={16} />
          {t`Images are cropped to fit, never stretched.`}
        </>
      }
      actions={
        <>
          <Button variant="ghost" icon="restore" disabled={!selection || reset.isPending} onClick={resetKind}>
            {t`Reset to automatic`}
          </Button>
          <Button variant="primary" size="lg" icon="check" onClick={() => onOpenChange(false)}>
            {t`Done`}
          </Button>
        </>
      }
    >
      <div className="aw">
        <div className="aw-prev">
          <div className="aw-lbl">{t`Preview`}</div>
          <div className="aw-heroprev">
            <GameArt game={game} kind="hero" showTitle={false} className="object-top" />
            {game.artwork.logo ? <img className="aw-logo" src={toImageSrc(game.artwork.logo)} alt="" /> : null}
          </div>
          <div className="aw-row2">
            <div className="aw-cov">
              <GameArt game={game} kind="cover" showTitle={false} />
            </div>
            <div>
              <div className="aw-lbl" style={{ marginBottom: 8 }}>
                {t`Icon`}
              </div>
              <div className="aw-icn">
                <GameArt game={game} kind="icon" showTitle={false} />
              </div>
            </div>
          </div>
          <div className="text-[12.5px] text-ink-muted">
            <span className="text-ink-faint">{t`Source`}</span> · <b className="font-semibold text-ink">{source}</b>
          </div>
          <Button
            variant="ghost"
            size="sm"
            icon="search"
            onClick={() => searchForm.current?.querySelector("input")?.focus()}
            disabled={key.data !== true}
          >
            {t`Wrong game? Search another title`}
          </Button>
        </div>

        <div className="aw-pick">
          <div className="aw-bar">
            <Tabs
              value={kind}
              onChange={(next) => {
                setKind(next);
                setStyle(ALL_STYLES);
              }}
              items={kinds.map((entry) => ({
                id: entry.id,
                label: entry.label,
                ...(entry.id === kind && candidates.data ? { count: candidates.data.length } : {}),
              }))}
            />
            {styles.length > 1 ? (
              <span style={{ marginLeft: "auto" }}>
                <Dropdown
                  prefix={t`Style`}
                  value={style}
                  onChange={setStyle}
                  options={[
                    { id: ALL_STYLES, label: t`All styles` },
                    ...styles.map((entry) => ({ id: entry, label: entry })),
                  ]}
                />
              </span>
            ) : null}
          </div>

          {key.data !== true ? (
            <div className="aw-note">
              <b className="font-semibold text-ink">{t`Find more images on SteamGridDB`}</b>
              <p className="mt-1 mb-3">
                {t`It’s free: add your own SteamGridDB API key once and thousands of community covers, heroes and logos become available.`}
              </p>
              <Button asChild size="sm" variant="primary">
                <Link to={`${ROUTES.settings}?section=artwork`}>{t`Set up SteamGridDB`}</Link>
              </Button>
            </div>
          ) : (
            <>
              <form
                ref={searchForm}
                className="aw-search"
                onSubmit={(event) => {
                  event.preventDefault();
                  setTerm(draft);
                }}
              >
                <SearchField
                  placeholder={t`Game name (default: ${game.title})`}
                  value={draft}
                  onChange={(event) => setDraft(event.target.value)}
                />
                <Button type="submit" disabled={candidates.isFetching}>
                  {t`Search`}
                </Button>
              </form>

              {candidates.isError ? (
                <div className="aw-note">{errorText(candidates.error)}</div>
              ) : candidates.isFetching && !candidates.data ? (
                <div className={cn("aw-grid", `is-${kind}`)}>
                  {Array.from({ length: kind === "hero" ? 4 : 10 }, (_, index) => (
                    <Skeleton key={index} className={cn("aw-cand", `is-${kind}`)} />
                  ))}
                </div>
              ) : (
                <div className={cn("aw-grid", `is-${kind}`)} aria-label={t`Candidates`}>
                  <button
                    type="button"
                    className={cn("aw-upload", `is-${kind}`)}
                    aria-label={t`Use your own image`}
                    title={t`Use your own image`}
                    onClick={() => void upload()}
                    disabled={importer.isPending}
                  >
                    <span className="aw-upload-in">
                      <DsIcon name="upload" size={22} />
                      <span className="aw-upload-label">{t`Use your own image`}</span>
                      <span className="aw-upload-hint text-ink-faint">{t`PNG · JPG · WebP`}</span>
                    </span>
                  </button>
                  {candidates.data?.length === 0 ? (
                    <p className="aw-note" style={{ gridColumn: "1 / -1" }}>
                      {t`No image found. Try another spelling of the game’s name.`}
                    </p>
                  ) : null}
                  {shown.map((candidate) => {
                    const selected = chosen[kind] === candidate.id;
                    return (
                      <button
                        key={candidate.id}
                        type="button"
                        aria-pressed={selected}
                        aria-busy={pendingId === candidate.id}
                        onClick={() => choose(candidate)}
                        disabled={apply.isPending}
                        className={cn("aw-cand", `is-${kind}`, selected && "on")}
                      >
                        <img src={candidate.thumbUrl} alt="" loading="lazy" />
                        {selected ? (
                          <span className="aw-chk">
                            <DsIcon
                              name={pendingId === candidate.id ? "sync" : "check"}
                              size={14}
                              className={pendingId === candidate.id ? "animate-spin" : undefined}
                            />
                          </span>
                        ) : null}
                        {candidate.author || candidate.style ? (
                          <span className="aw-src">{candidate.author ?? candidate.style}</span>
                        ) : null}
                      </button>
                    );
                  })}
                </div>
              )}
            </>
          )}
        </div>
      </div>
    </Dialog>
  );
}
