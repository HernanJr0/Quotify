import { useEffect, useState, type CSSProperties } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ProviderInstallation } from "../types/provider";
import type { ProviderUsage } from "../types/usage";
import { ProviderIcon } from "./ProviderIcon";
import { UsageRing } from "./UsageRing";

interface ProviderButtonProps {
  displayUsage?: ProviderUsage | null;
  displayRuntimeName?: string;
  forceRefresh: boolean;
  hidden?: boolean;
  installation: ProviderInstallation;
  onHoverChange: (isHovered: boolean) => void;
  onUsageLoaded: (installationId: string, usage: ProviderUsage | null) => void;
  refreshToken: number;
  tooltipAlignment?: "start" | "center" | "end";
}

const PROVIDER_ACCENTS: Record<string, string> = {
  claude: "#dc9563",
  codex: "#82d45b",
  gemini: "#6fa4ff",
  grok: "#b28af3",
};

export function ProviderButton({
  displayUsage,
  displayRuntimeName,
  forceRefresh,
  hidden = false,
  installation,
  onHoverChange,
  onUsageLoaded,
  refreshToken,
  tooltipAlignment = "center",
}: ProviderButtonProps) {
  const [usage, setUsage] = useState<ProviderUsage | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    onUsageLoaded(installation.id, null);
    invoke<ProviderUsage>("fetch_provider_usage", {
      installationId: installation.id,
      forceRefresh,
    })
      .then((nextUsage) => {
        setUsage(nextUsage);
        setError(null);
        onUsageLoaded(installation.id, nextUsage);
      })
      .catch((reason: unknown) => {
        setUsage(null);
        setError(String(reason));
        onUsageLoaded(installation.id, null);
      });
  }, [forceRefresh, installation.id, onUsageLoaded, refreshToken]);

  const visibleUsage = displayUsage ?? usage;
  const visibleError = visibleUsage?.error ?? (visibleUsage ? null : error);
  const percentage = visibleUsage?.percentage ?? 0;
  const status = visibleUsage
    ? usageStatus(visibleUsage)
    : visibleError
      ? "is-error"
      : "is-loading";
  const isMock = visibleUsage?.source === "mock";
  const isStale = Boolean(visibleUsage?.error);
  const reset = visibleUsage?.resetAt ? formatReset(visibleUsage.resetAt) : null;
  const accent = PROVIDER_ACCENTS[installation.provider] ?? "#a3a3a3";
  const percentageLabel =
    visibleUsage?.percentage == null ? "—" : formatPercentage(visibleUsage.percentage);
  const detail = visibleError
    ? friendlyError(visibleError)
    : visibleUsage
      ? (visibleUsage.periodDescription ?? "Current window") + (reset ? " · " + reset : "")
      : "Checking…";
  const weekly = visibleUsage?.weekly;
  const weeklyLabel = weekly ? `${formatPercentage(weekly.percentage)} weekly` : null;
  const runtimeLabel = displayRuntimeName ?? installation.runtimeName;
  const buttonStyle = {
    "--provider-accent": accent,
  } as CSSProperties;

  return (
    <button
      className={
        "provider-orb " +
        status +
        (visibleError ? " has-error" : "") +
        (hidden ? " is-hidden" : "") +
        ` tooltip-${tooltipAlignment}`
      }
      style={buttonStyle}
      type="button"
      aria-label={`${installation.providerName}: ${percentageLabel} · ${runtimeLabel}`}
      aria-hidden={hidden}
      tabIndex={hidden ? -1 : undefined}
      title={`${installation.providerName} · ${runtimeLabel}`}
      onBlur={() => onHoverChange(false)}
      onFocus={() => onHoverChange(true)}
      onMouseEnter={() => onHoverChange(true)}
      onMouseLeave={() => onHoverChange(false)}
    >
      <span className="provider-orb-visual">
        <UsageRing percentage={percentage} status={status} />
        <span className="provider-orb-icon">
          <ProviderIcon provider={installation.provider} />
        </span>
        {weekly && (
          <span className="provider-orb-weekly" aria-label={weeklyLabel ?? undefined}>
            <UsageRing
              className="usage-ring-weekly"
              percentage={weekly.percentage}
              status={usageStatusForPercentage(weekly.percentage)}
            />
            <span>W</span>
          </span>
        )}
      </span>
      <span className="provider-orb-label">{installation.providerName}</span>

      <span className="provider-orb-details" role="tooltip">
        <span className="provider-orb-details-heading">
          <strong>{installation.providerName}</strong>
          <span>{runtimeLabel}</span>
        </span>
        <span className="provider-orb-details-usage">
          <strong>{percentageLabel}</strong>
          <span>{detail}</span>
        </span>
        {weekly && (
          <span className="provider-orb-details-usage">
            <strong>{formatPercentage(weekly.percentage)}</strong>
            <span>Weekly{weekly.resetAt ? ` · ${formatReset(weekly.resetAt)}` : ""}</span>
          </span>
        )}
        <span className="provider-orb-flags">
          {isMock && <span>mock</span>}
          {isStale && <span className="is-stale">cached</span>}
        </span>
      </span>
    </button>
  );
}

function usageStatus(usage: ProviderUsage): string {
  if (usage.status === "error" || usage.status === "unavailable") return "is-error";
  if (usage.percentage == null) return "is-unknown";
  if (usage.percentage >= 95) return "is-critical";
  if (usage.percentage >= 85) return "is-high";
  if (usage.percentage >= 70) return "is-warning";
  return "is-ok";
}

function usageStatusForPercentage(percentage: number): string {
  return usageStatus({ status: "ok", percentage } as ProviderUsage);
}

function formatPercentage(value: number): string {
  return value.toFixed(Number.isInteger(value) ? 0 : 1) + "%";
}

function formatReset(value: string): string {
  const timestamp = Date.parse(value);
  if (Number.isNaN(timestamp)) return value;

  return new Intl.DateTimeFormat(undefined, {
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(timestamp));
}

function friendlyError(value: string): string {
  if (value.includes("no ChatGPT quota") || value.includes("check `codex login`")) {
    return "Sign in to Codex";
  }
  if (value.includes("no active quota") || value.includes("no usage window")) {
    return "No active quota";
  }
  if (
    value.includes("Claude CLI returned no plan rate limits") ||
    value.includes("Claude plan rate limits are not available")
  ) {
    return "No Claude plan";
  }

  return value.replace(/^Error:\s*/i, "").replace(/^Unavailable:\s*/i, "");
}
