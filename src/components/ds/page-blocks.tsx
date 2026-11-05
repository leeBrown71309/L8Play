import type { ReactNode } from "react";

import { DsIcon, type DsIconName } from "@/components/ds/ds-icon";
import { ProgressBar } from "@/components/ds/progress-bar";
import { StatusBadge, type StatusKind } from "@/components/ds/status-badge";
import { GameArt } from "@/components/game/GameArt";
import { IconButton } from "@/components/ui/icon-button";
import type { GameSummary } from "@/lib/bindings/GameSummary";
import { cn } from "@/lib/cn";

interface PageHeaderProps {
  title: string;
  count?: number;
  eyebrow?: string;
  subtitle?: string;
  actions?: ReactNode;
  className?: string;
}

/** Page heading with optional count, eyebrow, subtitle and actions. */
export function PageHeader({ title, count, eyebrow, subtitle, actions, className }: PageHeaderProps) {
  return (
    <div className={cn("l8-pagehead", className)}>
      <div>
        {eyebrow ? <div className="l8-eyebrow">{eyebrow}</div> : null}
        <h1 className="l8-pagehead-title">
          {title}
          {count !== undefined ? <span>{count}</span> : null}
        </h1>
        {subtitle ? <p className="l8-pagehead-sub">{subtitle}</p> : null}
      </div>
      {actions ? <div className="l8-pagehead-actions">{actions}</div> : null}
    </div>
  );
}

interface SectionHeaderProps {
  title: string;
  count?: number;
  icon?: DsIconName;
  action?: ReactNode;
  className?: string;
}

/** Section heading with an optional trailing action ("See all"). */
export function SectionHeader({ title, count, icon, action, className }: SectionHeaderProps) {
  return (
    <div className={cn("l8-sechead", className)}>
      <h2>
        {icon ? <DsIcon name={icon} size={18} /> : null}
        {title}
        {count !== undefined ? <span>{count}</span> : null}
      </h2>
      {action ?? null}
    </div>
  );
}

type BannerTone = "success" | "cloud" | "warning" | "danger" | "neutral";

const BANNER_ICON: Record<BannerTone, DsIconName> = {
  success: "checkCircle",
  cloud: "cloudUp",
  warning: "alert",
  danger: "alertCircle",
  neutral: "info",
};

interface BannerProps {
  tone?: BannerTone;
  icon?: DsIconName;
  title: string;
  action?: ReactNode;
  children?: ReactNode;
  className?: string;
}

/** Inline banner for page-level state. */
export function Banner({ tone = "neutral", icon, title, action, children, className }: BannerProps) {
  return (
    <div className={cn("l8-banner", `tone-${tone}`, className)} role={tone === "danger" ? "alert" : "status"}>
      <span className="l8-banner-icon">
        <DsIcon name={icon ?? BANNER_ICON[tone]} size={18} />
      </span>
      <div className="l8-banner-text">
        <b>{title}</b>
        {children ? <span>{children}</span> : null}
      </div>
      {action ? <div className="l8-banner-actions">{action}</div> : null}
    </div>
  );
}

interface ActionRowProps {
  game?: GameSummary;
  icon?: DsIconName;
  title: string;
  /** Coral aside after the title, shown in parentheses. */
  aside?: string;
  description?: ReactNode;
  status?: StatusKind;
  onClick?: () => void;
  action?: ReactNode;
}

/** Rounded list row with a game icon tile, a title with a coral aside, and a chevron. */
export function ActionRow({ game, icon, title, aside, description, status, onClick, action }: ActionRowProps) {
  return (
    <div
      className="l8-arow"
      role="button"
      tabIndex={0}
      onClick={onClick}
      onKeyDown={(event) => {
        if (onClick && (event.key === "Enter" || event.key === " ")) {
          event.preventDefault();
          onClick();
        }
      }}
    >
      <span className="l8-arow-tile">
        {game ? <GameArt game={game} kind="icon" showTitle={false} /> : <DsIcon name={icon ?? "info"} size={22} />}
      </span>
      <span className="l8-arow-text">
        <span className="l8-arow-title">
          {title}
          {aside ? <em>{` (${aside})`}</em> : null}
        </span>
        {description ? (
          <span className="l8-arow-desc">
            {status ? <StatusBadge status={status} size="sm" /> : null}
            {description}
          </span>
        ) : null}
      </span>
      {action ?? (
        <span className="l8-arow-chev">
          <DsIcon name="chevronRight" size={18} />
        </span>
      )}
    </div>
  );
}

interface OrbBubble {
  icon: DsIconName;
  value: ReactNode;
  label: string;
  tone: "cream" | "cloud" | "flare";
}

interface ProtectionOrbProps {
  value: ReactNode;
  label: string;
  sub?: string;
  bubbles: OrbBubble[];
}

/** Save-protection orb: a glowing blob holding the total, with bubbles below. */
export function ProtectionOrb({ value, label, sub, bubbles }: ProtectionOrbProps) {
  return (
    <div className="l8-orb">
      <div className="l8-orb-stage">
        <div className="l8-orb-blob" />
        <div className="l8-orb-blob is-2" />
        <div className="l8-orb-core">
          <span className="l8-orb-label">{label}</span>
          <b>{value}</b>
          {sub ? <span className="l8-orb-sub">{sub}</span> : null}
        </div>
      </div>
      <div className="l8-orb-bubbles">
        {bubbles.map((bubble) => (
          <div key={bubble.label} className="l8-orb-bubble">
            <span className={`l8-orb-dot tone-${bubble.tone}`} title={bubble.label}>
              <DsIcon name={bubble.icon} size={22} />
            </span>
            <b>{bubble.value}</b>
            <span>{bubble.label}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

interface TransferRowProps {
  game: GameSummary;
  title?: string;
  chip: string;
  status: string;
  meta: string;
  value?: number;
  tone?: "cloud" | "success" | "warning";
  done?: StatusKind;
  pauseLabel: string;
  cancelLabel: string;
  onPause?: () => void;
  onCancel?: () => void;
}

/** A transfer strip: game tile, name, kind chip, progress and round controls. */
export function TransferRow({
  game,
  title,
  chip,
  status,
  meta,
  value,
  tone = "cloud",
  done,
  pauseLabel,
  cancelLabel,
  onPause,
  onCancel,
}: TransferRowProps) {
  return (
    <div className={cn("l8-xfer", `tone-${tone}`)}>
      <span className="l8-xfer-tile">
        <GameArt game={game} kind="icon" showTitle={false} />
      </span>
      <div className="l8-xfer-name">
        <b>{title ?? game.title}</b>
        <span className="l8-xfer-chip">{chip}</span>
      </div>
      <div className="l8-xfer-prog">
        <div className="l8-xfer-meta">
          <b>{status}</b>
          <span>{meta}</span>
        </div>
        {value !== undefined ? <ProgressBar value={value} tone={tone} size="sm" /> : null}
      </div>
      <div className="l8-xfer-btns">
        {done ? (
          <StatusBadge status={done} />
        ) : (
          <>
            {onPause ? (
              <IconButton icon="pause" label={pauseLabel} size="sm" variant="primary" onClick={onPause} />
            ) : null}
            {onCancel ? <IconButton icon="x" label={cancelLabel} size="sm" variant="cream" onClick={onCancel} /> : null}
          </>
        )}
      </div>
    </div>
  );
}
