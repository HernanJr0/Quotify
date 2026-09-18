import type { ProviderKind } from "./provider";

export type UsageStatus = "ok" | "warning" | "critical" | "unknown" | "unavailable" | "error";

export type UsageUnit = "percentage" | "tokens" | "requests" | "credits" | "messages" | "unknown";

export type UsagePeriod = "hourly" | "rolling" | "daily" | "weekly" | "monthly" | "unknown";

export type UsageSource = "mock" | "officialApi" | "cli" | "localState" | "internalEndpoint";

export interface ProviderUsage {
  provider: ProviderKind;
  installationId: string;
  runtimeId: string;
  status: UsageStatus;
  percentage?: number;
  used?: number;
  remaining?: number;
  limit?: number;
  unit?: UsageUnit;
  period?: UsagePeriod;
  periodDescription?: string;
  resetAt?: string;
  updatedAt: string;
  source: UsageSource;
  error?: string;
}
