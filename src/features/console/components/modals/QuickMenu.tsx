import { useLingui } from "@lingui/react/macro";

import { ListRow } from "@/features/console/components/ListRow";
import { ModalFrame } from "@/features/console/components/modals/ModalFrame";
import { useConsoleMode } from "@/features/console/console-mode-store";
import { useConsoleNav } from "@/features/console/core/nav-store";
import { useModalList } from "@/features/console/core/use-modal-list";
import type { DsIconName } from "@/components/ds/ds-icon";

/** The Start button's panel: carry on, jump to the settings, or go back to the desktop. */
export function QuickMenu() {
  const { t } = useLingui();
  const dispatch = useConsoleNav((state) => state.dispatch);
  const exit = useConsoleMode((state) => state.exit);

  const items: { label: string; icon: DsIconName; run: () => void }[] = [
    { label: t`Resume`, icon: "play", run: () => dispatch({ type: "popModal" }) },
    { label: t`Settings`, icon: "settings", run: () => dispatch({ type: "gotoSection", section: "settings" }) },
    { label: t`Exit console mode`, icon: "arrowRight", run: exit },
  ];
  const list = useModalList({
    count: items.length,
    onConfirm: (index) => items[index]?.run(),
    onBack: () => dispatch({ type: "popModal" }),
  });

  return (
    <ModalFrame
      title={t`Menu`}
      icon="gamepad"
      footer={
        <>
          {items.map((item, index) => (
            <ListRow
              key={item.label}
              label={item.label}
              icon={item.icon}
              focused={list.index === index}
              onFocus={() => list.setIndex(index)}
              onActivate={item.run}
            />
          ))}
        </>
      }
    />
  );
}
