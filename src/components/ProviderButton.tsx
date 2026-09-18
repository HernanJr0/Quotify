import { useEffect, useState, type CSSProperties } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ProviderInstallation } from "../types/provider";
import type { ProviderUsage } from "../types/usage";
import { ProviderIcon } from "./ProviderIcon";
import { UsageRing } from "./UsageRing";

interface ProviderButtonProps {
  displayRuntimeName?: string;
  hidden?: boolean;
  installation: ProviderInstallation;
  onHoverChange: (isHovered: boolean) => void;
  onUsageLoaded: (installationId: string, usage: ProviderUsage | null) => void;
  refreshToken: number;
}

const PROVIDER_ACCENTS: Record<string, string> = {
  claude: "#dc9563",
  codex: "#82d45b",
  gemini: "#6fa4ff",
  grok: "#b28af3",
};

export function ProviderButton({
  displayRuntimeName,
  hidden = false,
  installation,
  onHoverChange,
  onUsageLoaded,
  refreshToken,
}: ProviderButtonProps) {
  const [usage, setUsage] = useState<ProviderUsage | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    onUsageLoaded(installation.id, null);
    invoke<ProviderUsage>("fetch_provider_usage", { installationId: installation.id })
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
  }, [installation.id, onUsageLoaded, refreshToken]);

  const percentage = usage?.percentage ?? 0;
  const status = usage ? usageStatus(usage) : error ? "is-error" : "is-loading";
  const isMock = usage?.source === "mock";
  const isStale = Boolean(usage?.error);
  const reset = usage?.resetAt ? formatReset(usage.resetAt) : null;
  const accent = PROVIDER_ACCENTS[installation.provider] ?? "#a3a3a3";
  const percentageLabel = usage?.percentage == null ? "—" : formatPercentage(usage.percentage);
  const detail = error
    ? friendlyError(error)
    : usage
      ? (usage.periodDescription ?? "Current window") + (reset ? " · " + reset : "")
      : "Checking…";
  const runtimeLabel = displayRuntimeName ?? installation.runtimeName;
  const buttonStyle = {
    "--provider-accent": accent,
  } as CSSProperties;

  return (
    <button
      className={
        "provider-orb " + status + (error ? " has-error" : "") + (hidden ? " is-hidden" : "")
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
