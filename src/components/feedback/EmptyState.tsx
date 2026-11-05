import type { ReactNode } from "react";

import { cn } from "@/lib/cn";

interface EmptyStateProps {
  icon: ReactNode;
  title: string;
  body?: string | undefined;
  action?: ReactNode;
  className?: string;
}

/** What is missing, why, and the one thing to do next — with the dashed-ring icon (`l8-empty`). */
export function EmptyState({ icon, title, body, action, className }: EmptyStateProps) {
  return (
    <div className={cn("l8-empty", className)}>
      <span className="l8-empty-icon">{icon}</span>
      <h3>{title}</h3>
      {body ? <p>{body}</p> : null}
      {action ? <div className="l8-empty-actions">{action}</div> : null}
    </div>
  );
}
