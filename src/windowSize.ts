import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";

// Pins the current window to an exact logical size. GTK ignores resize
// requests on non-resizable windows, so auxiliary windows are declared
// resizable and locked here by matching min and max sizes instead.
export async function pinWindowSize(width: number, height: number): Promise<void> {
  const size = new LogicalSize(Math.ceil(width), Math.ceil(height));
  const appWindow = getCurrentWindow();
  await appWindow.setMinSize(size);
  await appWindow.setMaxSize(size);
  await appWindow.setSize(size);
}
