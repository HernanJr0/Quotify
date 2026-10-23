// Shared contract between the overlay and the appearance window. Both
// windows share the same origin, so localStorage holds the persisted value
// while Tauri events carry live changes across windows.

export const APPEARANCE_WINDOW_LABEL = "appearance";
// Logical width of the appearance window; its height follows the content.
export const APPEARANCE_WINDOW_WIDTH = 220;

export const OPACITY_CHANGED_EVENT = "appearance:opacity-changed";
export const RESET_SIZE_EVENT = "appearance:reset-size";
export const APPEARANCE_VISIBILITY_EVENT = "appearance:visibility";

const OPACITY_STORAGE_KEY = "quotify:opacity";
export const MIN_OPACITY = 35;
export const MAX_OPACITY = 100;
const DEFAULT_OPACITY = 50;

export function readOpacity(): number {
  const stored = Number(localStorage.getItem(OPACITY_STORAGE_KEY));
  return Number.isFinite(stored) && stored >= MIN_OPACITY && stored <= MAX_OPACITY
    ? stored
    : DEFAULT_OPACITY;
}

export function storeOpacity(opacity: number): void {
  localStorage.setItem(OPACITY_STORAGE_KEY, String(opacity));
}
