import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { useConsoleInput, useInputSource } from "@/features/console/input";

function pressKey(key: string) {
  act(() => {
    window.dispatchEvent(new KeyboardEvent("keydown", { key, cancelable: true }));
  });
}

describe("useConsoleInput", () => {
  it("delivers actions with the latest handler and stops after unmount", () => {
    const first = vi.fn();
    const second = vi.fn();
    const { rerender, unmount } = renderHook(({ handler }) => useConsoleInput(handler), {
      initialProps: { handler: first },
    });
    pressKey("ArrowDown");
    expect(first).toHaveBeenCalledWith("down", "keyboard");
    rerender({ handler: second });
    pressKey("Enter");
    expect(second).toHaveBeenCalledWith("confirm", "keyboard");
    expect(first).toHaveBeenCalledTimes(1);
    unmount();
    pressKey("Enter");
    expect(second).toHaveBeenCalledTimes(1);
  });

  it("lets a dialog capture the input while it is enabled", () => {
    const page = vi.fn();
    const dialog = vi.fn();
    renderHook(() => useConsoleInput(page));
    const { rerender } = renderHook(({ open }) => useConsoleInput(dialog, { enabled: open }), {
      initialProps: { open: true },
    });
    pressKey("e");
    rerender({ open: false });
    pressKey("q");
    expect(dialog).toHaveBeenCalledTimes(1);
    expect(page).toHaveBeenCalledTimes(1);
    expect(page).toHaveBeenCalledWith("prevTab", "keyboard");
  });

  it("reports the device used last", () => {
    const { result } = renderHook(() => useInputSource());
    act(() => {
      window.dispatchEvent(new Event("pointermove"));
    });
    expect(result.current).toBe("pointer");
  });
});
