import { useEffect, useState, type CSSProperties } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ProviderInstallation } from "../types/provider";
import type { ProviderUsage } from "../types/usage";
import { ProviderIcon } from "./ProviderIcon";
import { UsageBar } from "./UsageBar";

interface ProviderCardProps {
  installation: ProviderInstallation;
  refreshToken: number;
}

const PROVIDER_ACCENTS: Record<string, string> = {
  claude: "#dc9563",
  codex: "#82d45b",
  gemini: "#6fa4ff",
  grok: "#b28af3",
};

export function ProviderCard({ installation, refreshToken }: ProviderCardProps) {
  const [usage, setUsage] = useState<ProviderUsage | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<ProviderUsage>("fetch_provider_usage", { installationId: installation.id })
      .then((nextUsage) => {
        setUsage(nextUsage);
        setError(null);
      })
      .catch((reason: unknown) => {
        setUsage(null);
        setError(String(reason));
      });
  }, [installation.id, refreshToken]);

  const percentage = usage?.percentage ?? 0;
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
  const cardStyle = {
    "--provider-accent": accent,
  } as CSSProperties;

  return (
    <article
      className={"provider-card" + (error ? " has-error" : "")}
      style={cardStyle}
      title={error ?? usage?.error ?? undefined}
    >
      <div className="provider-card-header">
        <span className="provider-glyph" aria-hidden="true">
          <ProviderIcon provider={installation.provider} />
        </span>
        <div className="provider-identity">
          <strong>{installation.providerName}</strong>
          <span>{installation.runtimeName}</span>
        </div>
        <strong className="provider-percentage">{percentageLabel}</strong>
      </div>

      <UsageBar percentage={percentage} />

      <div className="provider-detail">
        <span>{detail}</span>
        <div className="provider-flags">
          {isMock && <span>mock</span>}
          {isStale && <span className="is-stale">cached</span>}
        </div>
      </div>
    </article>
  );
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
