import { useState } from "react";

import { useConsoleInput, type ConsoleAction } from "@/features/console/input";
import { consoleSound } from "@/features/console/sound";

interface ModalListOptions {
  count: number;
  /** Cells per row; up and down jump by this many. */
  columns?: number;
  onConfirm: (index: number) => void;
  onBack: () => void;
  /** The X button (clear, delete…); optional. */
  onSecondary?: () => void;
  /** Where the focus starts; dangerous dialogs start on their safe choice. */
  initialIndex?: number;
}

/**
 * Input for a dialog's list: this hook mounts after the console root's, so while the dialog is open it is the one
 * that receives the buttons, and everything behind it stays still.
 */
export function useModalList({
  count,
  columns = 1,
  onConfirm,
  onBack,
  onSecondary,
  initialIndex = 0,
}: ModalListOptions) {
  const [index, setIndex] = useState(initialIndex);

  const move = (delta: number) => {
    const next = Math.min(count - 1, Math.max(0, index + delta));
    if (next === index) return;
    consoleSound.play("move");
    setIndex(next);
  };

  useConsoleInput((action: ConsoleAction) => {
    switch (action) {
      case "up":
        return move(-columns);
      case "down":
        return move(columns);
      case "left":
        return columns > 1 ? move(-1) : undefined;
      case "right":
        return columns > 1 ? move(1) : undefined;
      case "confirm":
        if (count > 0) {
          consoleSound.play("select");
          onConfirm(index);
        }
        return;
      case "back":
        consoleSound.play("back");
        return onBack();
      case "secondary":
        return onSecondary?.();
      default:
    }
  });

  return { index, setIndex: (next: number) => setIndex(Math.min(Math.max(0, next), Math.max(0, count - 1))) };
}
