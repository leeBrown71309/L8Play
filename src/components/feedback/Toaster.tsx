import { useLingui } from "@lingui/react/macro";

import { DsIcon, type DsIconName } from "@/components/ds/ds-icon";
import { useToasts, type ToastTone } from "@/components/feedback/toast-store";
import { IconButton } from "@/components/ui/icon-button";

const TONE: Record<ToastTone, { icon: DsIconName; tone: string }> = {
  success: { icon: "checkCircle", tone: "success" },
  neutral: { icon: "info", tone: "neutral" },
  warning: { icon: "alert", tone: "warning" },
  danger: { icon: "alertCircle", tone: "danger" },
};

/** Bottom-right toast stack (`l8-toast`), above the quick-launch rail. */
export function Toaster() {
  const { t } = useLingui();
  const { toasts, dismiss } = useToasts();
  return (
    <div aria-live="polite" className="pointer-events-none fixed right-[104px] bottom-5 z-50 flex flex-col gap-2.5">
      {toasts.map((toast) => {
        const { icon, tone } = TONE[toast.tone];
        return (
          <div
            key={toast.id}
            role={toast.tone === "danger" ? "alert" : "status"}
            className={`l8-toast tone-${tone} pointer-events-auto`}
          >
            <span className="l8-toast-icon">
              <DsIcon name={icon} size={20} />
            </span>
            <div className="l8-toast-body">
              <div className="l8-toast-title">{toast.title}</div>
              {toast.body ? <div className="l8-toast-text">{toast.body}</div> : null}
            </div>
            <IconButton
              icon="x"
              label={t`Dismiss`}
              size="sm"
              className="l8-toast-close"
              onClick={() => dismiss(toast.id)}
            />
          </div>
        );
      })}
    </div>
  );
}
