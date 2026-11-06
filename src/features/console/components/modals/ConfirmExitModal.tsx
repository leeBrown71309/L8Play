import { useLingui } from "@lingui/react/macro";

import { ListRow } from "@/features/console/components/ListRow";
import { ModalFrame } from "@/features/console/components/modals/ModalFrame";
import { useConsoleMode } from "@/features/console/console-mode-store";
import { useConsoleNav } from "@/features/console/core/nav-store";
import { useModalList } from "@/features/console/core/use-modal-list";
import { consoleSound } from "@/features/console/sound";

/** "Leave console mode?": the focus starts on Stay, so a stray confirm keeps the user where they are. */
export function ConfirmExitModal() {
  const { t } = useLingui();
  const dispatch = useConsoleNav((state) => state.dispatch);
  const exit = useConsoleMode((state) => state.exit);
  const stay = () => dispatch({ type: "popModal" });

  const items = [
    {
      label: t`Leave console mode`,
      icon: "arrowRight" as const,
      run: () => {
        consoleSound.play("close");
        exit();
      },
    },
    { label: t`Stay in console mode`, icon: "gamepad" as const, run: stay },
  ];
  const list = useModalList({
    count: items.length,
    initialIndex: 1,
    onConfirm: (index) => items[index]?.run(),
    onBack: stay,
  });

  return (
    <ModalFrame
      title={t`Leave console mode?`}
      icon="arrowRight"
      tone="warning"
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
    >
      <p className="cs-note">{t`You will go back to the desktop interface. Your games and saves are not affected.`}</p>
    </ModalFrame>
  );
}
