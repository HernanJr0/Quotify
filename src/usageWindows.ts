import type { ProviderUsage, UsagePeriod } from "./types/usage";

export interface UsageWindowView {
  label: string;
  percentage: number;
  period?: UsagePeriod;
  resetAt?: string;
}

export interface UsageWindows {
  // Short (5h) window. `null` while the provider reports none, which happens
  // for Claude until a session is started.
  session: UsageWindowView | null;
  // Long (weekly) window.
  weekly: UsageWindowView | null;
  // Providers with one unrelated window (e.g. a 30-day quota) keep a single ring.
  single: UsageWindowView | null;
}

export function splitUsageWindows(usage: ProviderUsage | null | undefined): UsageWindows {
  const none: UsageWindows = { session: null, weekly: null, single: null };
  if (!usage || usage.percentage == null) return none;

  const primary: UsageWindowView = {
    label: usage.periodDescription ?? "Current window",
    percentage: usage.percentage,
    period: usage.period,
    resetAt: usage.resetAt,
  };

  if (usage.weekly) {
    return {
      session: { ...primary, label: "5h" },
      weekly: { ...usage.weekly, label: "Weekly" },
      single: null,
    };
  }
  if (usage.period === "weekly") {
    return { session: null, weekly: { ...primary, label: "Weekly" }, single: null };
  }
  return { ...none, single: primary };
}
