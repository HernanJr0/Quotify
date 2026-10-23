// The usage details card lives in its own transparent window so it can extend
// past the overlay's bounds instead of being clipped by them.

export const DETAILS_WINDOW_LABEL = "details";
export const DETAILS_CHANGED_EVENT = "details:changed";

export const DETAILS_CARD_WIDTH = 164;

// Transparent margin around the card so its drop shadow is not clipped by the
// details window itself.
export const DETAILS_SHADOW_MARGIN = 8;

export interface ProviderDetailsWindow {
  label: string;
  // `null` when the provider reports no such window.
  percentage: number | null;
  percentageLabel: string;
  resetLabel: string | null;
  status: string;
}

export interface ProviderDetails {
  accent: string;
  detail: string;
  isMock: boolean;
  isStale: boolean;
  percentageLabel: string;
  providerName: string;
  runtimeLabel: string;
  status: string;
  // Empty while loading or on error; `detail` is shown instead.
  windows: ProviderDetailsWindow[];
}
