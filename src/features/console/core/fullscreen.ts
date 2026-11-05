import { isTauri } from "@/lib/tauri/runtime";

/** Native fullscreen through the window API; every call is a no-op in the browser preview. */
async function currentWindow() {
  if (!isTauri()) return null;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  return getCurrentWindow();
}

/** Goes fullscreen and reports whether the window already was (so leaving can restore it). */
export async function enterFullscreen(): Promise<boolean> {
  try {
    const window = await currentWindow();
    if (!window) return false;
    const already = await window.isFullscreen();
    if (!already) await window.setFullscreen(true);
    return already;
  } catch (error) {
    console.warn("Fullscreen could not be entered.", error);
    return false;
  }
}

export async function setFullscreen(fullscreen: boolean): Promise<void> {
  try {
    await (await currentWindow())?.setFullscreen(fullscreen);
  } catch (error) {
    console.warn("Fullscreen could not be changed.", error);
  }
}

export async function toggleFullscreen(): Promise<void> {
  try {
    const window = await currentWindow();
    if (window) await window.setFullscreen(!(await window.isFullscreen()));
  } catch (error) {
    console.warn("Fullscreen could not be toggled.", error);
  }
}
