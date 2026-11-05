import { cn } from "@/lib/cn";

/** Shimmering placeholder (`l8-skel`) used while data or artwork loads. */
export function Skeleton({ className }: { className?: string }) {
  return <div aria-hidden="true" className={cn("l8-skel", className)} />;
}
