import { useLingui } from "@lingui/react/macro";

import { ActionMenu, type ActionMenuItem } from "@/components/ui/action-menu";
import { IconButton } from "@/components/ui/icon-button";

export type SnapshotMenuItem = ActionMenuItem;

/** "More snapshot actions" button with a floating menu; Esc or an outside click closes it. */
export function SnapshotMenu({ items }: { items: SnapshotMenuItem[] }) {
  const { t } = useLingui();
  return (
    <ActionMenu
      items={items}
      trigger={({ toggle }) => (
        <IconButton
          icon="more"
          label={t`More snapshot actions`}
          size="sm"
          onClick={(event) => {
            event.stopPropagation();
            toggle();
          }}
        />
      )}
    />
  );
}
