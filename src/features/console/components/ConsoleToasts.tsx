import { DsIcon, type DsIconName } from "@/components/ds/ds-icon";
import { useConsoleToasts, type ConsoleToastTone } from "@/features/console/core/toast-store";
import { cn } from "@/lib/cn";

const ICONS: Record<ConsoleToastTone, DsIconName> = {
  neutral: "info",
  success: "checkCircle",
  warning: "alert",
  danger: "alertCircle",
};

/** Notifications at the top right of console mode; announced politely to screen readers. */
export function ConsoleToasts() {
  const toasts = useConsoleToasts((state) => state.toasts);
  return (
    <div className="cs-toasts" aria-live="polite" role="status">
      {toasts.map((toast) => (
        <div key={toast.id} className={cn("cs-toast", `tone-${toast.tone}`)}>
          <DsIcon name={ICONS[toast.tone]} size={26} />
          <div>
            <b>{toast.title}</b>
            {toast.body ? <span>{toast.body}</span> : null}
          </div>
        </div>
      ))}
    </div>
  );
}
