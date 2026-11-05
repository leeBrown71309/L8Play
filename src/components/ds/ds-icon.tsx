import {
  Archive,
  ArrowRight,
  Bell,
  Check,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  CircleAlert,
  CircleCheck,
  Clock,
  Cloud,
  CloudCheck,
  CloudDownload,
  CloudOff,
  CloudUpload,
  Copy,
  Download,
  Ellipsis,
  ExternalLink,
  File,
  Folder,
  FolderOpen,
  Gamepad2,
  GitMerge,
  HardDrive,
  House,
  Image,
  Info,
  Layers,
  List,
  LayoutGrid,
  Lock,
  Palette,
  Pause,
  Pencil,
  Play,
  Plus,
  RefreshCw,
  RotateCcwClock,
  ScanSearch,
  Search,
  Settings2,
  Shield,
  ShieldCheck,
  Sparkles,
  Star,
  Trash,
  TriangleAlert,
  Upload,
  X,
  type LucideIcon,
} from "lucide-react";

import { cn } from "@/lib/cn";

/** Icon names used by the design system screens, each mapped to its Lucide counterpart. */
const ICONS = {
  alert: TriangleAlert,
  alertCircle: CircleAlert,
  arrowRight: ArrowRight,
  bell: Bell,
  check: Check,
  checkCircle: CircleCheck,
  chevronDown: ChevronDown,
  chevronLeft: ChevronLeft,
  chevronRight: ChevronRight,
  clock: Clock,
  cloud: Cloud,
  cloudCheck: CloudCheck,
  cloudDown: CloudDownload,
  cloudOff: CloudOff,
  cloudUp: CloudUpload,
  copy: Copy,
  download: Download,
  drive: HardDrive,
  edit: Pencil,
  external: ExternalLink,
  file: File,
  folder: Folder,
  folderOpen: FolderOpen,
  gamepad: Gamepad2,
  grid: LayoutGrid,
  home: House,
  image: Image,
  info: Info,
  layers: Layers,
  list: List,
  lock: Lock,
  merge: GitMerge,
  more: Ellipsis,
  palette: Palette,
  pause: Pause,
  play: Play,
  plus: Plus,
  restore: RotateCcwClock,
  scan: ScanSearch,
  search: Search,
  settings: Settings2,
  shield: Shield,
  shieldCheck: ShieldCheck,
  snapshot: Archive,
  sparkle: Sparkles,
  star: Star,
  starFill: Star,
  sync: RefreshCw,
  trash: Trash,
  upload: Upload,
  x: X,
} satisfies Record<string, LucideIcon>;

export type DsIconName = keyof typeof ICONS;

interface DsIconProps {
  name: DsIconName;
  size?: number;
  className?: string | undefined;
  /** Accessible name; icons without one are decorative. */
  label?: string | undefined;
}

/** Design-system glyph. Always `currentColor`, 1.75 stroke, decorative unless `label` is given. */
export function DsIcon({ name, size = 20, className, label }: DsIconProps) {
  const Glyph = ICONS[name];
  return (
    <Glyph
      className={cn("l8-icon", className)}
      size={size}
      strokeWidth={1.75}
      fill={name === "starFill" ? "currentColor" : "none"}
      {...(label ? { role: "img", "aria-label": label } : { "aria-hidden": true })}
    />
  );
}
