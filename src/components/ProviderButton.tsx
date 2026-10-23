import { useCallback, useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ProviderDetails, ProviderDetailsWindow } from "../details";
import type { ProviderInstallation } from "../types/provider";
import type { ProviderUsage } from "../types/usage";
import { splitUsageWindows, type UsageWindowView } from "../usageWindows";
import { ProviderIcon } from "./ProviderIcon";
import { UsageRing } from "./UsageRing";

interface ProviderButtonProps {
  displayUsage?: ProviderUsage | null;
  displayRuntimeName?: string;
  forceRefresh: boolean;
  hidden?: boolean;
  installation: ProviderInstallation;
  onDetailsChange: (details: ProviderDetails | null, anchor?: DOMRect) => void;
  onUsageLoaded: (installationId: string, usage: ProviderUsage | null) => void;
  refreshToken: number;
}

interface UsageRequest {
  forceRefresh: boolean;
  installationId: string;
  refreshToken: number;
}

interface UsageRequestState {
  activeRequest: UsageRequest | null;
  disposed: boolean;
  inFlight: boolean;
  latestRequest: UsageRequest | null;
  pendingRequest: UsageRequest | null;
  sequence: number;
}

const INNER_RING_RADIUS = 35;

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
  onDetailsChange,
  onUsageLoaded,
  refreshToken,
}: ProviderButtonProps) {
  const [usage, setUsage] = useState<ProviderUsage | null>(null);
  const [hovered, setHovered] = useState(false);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const [error, setError] = useState<string | null>(null);
  const usageRef = useRef<ProviderUsage | null>(null);
  const requestState = useRef<UsageRequestState>({
    activeRequest: null,
    disposed: false,
    inFlight: false,
    latestRequest: null,
    pendingRequest: null,
    sequence: 0,
  });
  const loadUsageRef = useRef<(request: UsageRequest) => void>(() => undefined);

  const loadUsage = useCallback(
    (request: UsageRequest): void => {
      const state = requestState.current;
      state.latestRequest = request;

      if (state.disposed) return;
      if (state.inFlight) {
        if (!sameUsageRequest(state.activeRequest, request)) {
          state.pendingRequest = request;
        }
        return;
      }

      state.inFlight = true;
      state.activeRequest = request;
      const sequence = ++state.sequence;
      setError(null);
      // Keep showing the last known usage (and its accountKey, which drives
      // cross-runtime merging) while a refresh is in flight. Resetting to
      // null here would momentarily split an already-merged provider card
      // back into one icon per runtime on every auto-refresh.
      if (!usageRef.current) {
        onUsageLoaded(request.installationId, null);
      }

      invoke<ProviderUsage>("fetch_provider_usage", {
        installationId: request.installationId,
        forceRefresh: request.forceRefresh,
      })
        .then((nextUsage) => {
          if (!isCurrentUsageRequest(state, request, sequence)) return;

          usageRef.current = nextUsage;
          setUsage(nextUsage);
          setError(null);
          onUsageLoaded(request.installationId, nextUsage);
        })
        .catch((reason: unknown) => {
          if (!isCurrentUsageRequest(state, request, sequence)) return;

          setError(errorMessage(reason));
          if (!usageRef.current) {
            onUsageLoaded(request.installationId, null);
          }
        })
        .finally(() => {
          if (state.sequence !== sequence) return;

          state.inFlight = false;
          state.activeRequest = null;
          const pendingRequest = state.pendingRequest;
          state.pendingRequest = null;
          if (pendingRequest && !state.disposed) {
            loadUsageRef.current(pendingRequest);
          }
        });
    },
    [onUsageLoaded],
  );

  useEffect(() => {
    loadUsageRef.current = loadUsage;
  }, [loadUsage]);

  useEffect(() => {
    const state = requestState.current;
    requestState.current.disposed = false;
    loadUsage({
      forceRefresh,
      installationId: installation.id,
      refreshToken,
    });

    return () => {
      state.disposed = true;
      state.pendingRequest = null;
      state.latestRequest = null;
    };
  }, [forceRefresh, installation.id, loadUsage, refreshToken]);

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
  const accent = PROVIDER_ACCENTS[installation.provider] ?? "#a3a3a3";
  const percentageLabel =
    visibleUsage?.percentage == null ? "—" : formatPercentage(visibleUsage.percentage);
  const detail = visibleError ? friendlyError(visibleError) : visibleUsage ? "" : "Checking…";
  const runtimeLabel = displayRuntimeName ?? installation.runtimeName;
  const { session, weekly, single } = useMemo(() => splitUsageWindows(visibleUsage), [visibleUsage]);
  // Two concentric rings: weekly outside, 5h inside. The inner ring stays as an
  // empty track while no 5h window is reported.
  const isDual = weekly != null;
  const outerWindow = weekly ?? single;
  const details = useMemo<ProviderDetails>(() => {
    const windows: ProviderDetailsWindow[] = [];
    if (!visibleError) {
      if (isDual) windows.push(detailsWindow(session, "5h"), detailsWindow(weekly, "Weekly"));
      else if (single) windows.push(detailsWindow(single, single.label));
    }

    return {
      accent,
      detail,
      isMock,
      isStale,
      percentageLabel,
      providerName: installation.providerName,
      runtimeLabel,
      status,
      windows,
    };
  }, [
    accent,
    detail,
    installation.providerName,
    isDual,
    isMock,
    isStale,
    percentageLabel,
    runtimeLabel,
    session,
    single,
    status,
    visibleError,
    weekly,
  ]);

  // Keep the floating card in sync while hovered, including usage that
  // refreshes underneath the pointer.
  useEffect(() => {
    if (!hovered || hidden || !buttonRef.current) return;
    onDetailsChange(details, buttonRef.current.getBoundingClientRect());
  }, [details, hidden, hovered, onDetailsChange]);

  const showDetails = () => setHovered(true);
  const hideDetails = () => {
    setHovered(false);
    onDetailsChange(null);
  };

  const buttonStyle = {
    "--provider-accent": accent,
  } as CSSProperties;

  return (
    <button
      className={
        "provider-orb " +
        status +
        (visibleError ? " has-error" : "") +
        (hidden ? " is-hidden" : "")
      }
      ref={buttonRef}
      style={buttonStyle}
      type="button"
      aria-label={`${installation.providerName}: ${percentageLabel} · ${runtimeLabel}`}
      aria-hidden={hidden}
      tabIndex={hidden ? -1 : undefined}
      onBlur={hideDetails}
      onFocus={showDetails}
      onMouseEnter={showDetails}
      onMouseLeave={hideDetails}
    >
      <span className="provider-orb-visual">
        <UsageRing
          percentage={outerWindow ? outerWindow.percentage : percentage}
          status={outerWindow ? usageStatusForPercentage(outerWindow.percentage) : status}
        />
        {isDual && (
          <UsageRing
            className="usage-ring-inner"
            percentage={session ? session.percentage : null}
            radius={INNER_RING_RADIUS}
            status={session ? usageStatusForPercentage(session.percentage) : "is-unknown"}
          />
        )}
        <span className={"provider-orb-icon" + (isDual ? " is-compact" : "")}>
          <ProviderIcon provider={installation.provider} />
        </span>
      </span>
      <span className="provider-orb-label">{installation.providerName}</span>

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

function detailsWindow(window: UsageWindowView | null, label: string): ProviderDetailsWindow {
  if (!window) {
    return { label, percentage: null, percentageLabel: "—", resetLabel: null, status: "is-unknown" };
  }

  return {
    label,
    percentage: window.percentage,
    percentageLabel: formatPercentage(window.percentage),
    resetLabel: window.resetAt ? formatReset(window.resetAt) : null,
    status: usageStatusForPercentage(window.percentage),
  };
}

// Time only when the reset lands today; otherwise add the weekday and date so a
// weekly reset reads like "Tue 27/10 · 04:00" instead of a bare clock time.
function formatReset(value: string): string {
  const timestamp = Date.parse(value);
  if (Number.isNaN(timestamp)) return value;

  const date = new Date(timestamp);
  const time = new Intl.DateTimeFormat(undefined, { hour: "2-digit", minute: "2-digit" }).format(
    date,
  );
  if (date.toDateString() === new Date().toDateString()) return time;

  const weekday = new Intl.DateTimeFormat(undefined, { weekday: "short" })
    .format(date)
    .replace(/\.$/, "");
  const day = new Intl.DateTimeFormat(undefined, { day: "2-digit", month: "2-digit" }).format(date);
  return `${weekday} ${day} · ${time}`;
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

function sameUsageRequest(left: UsageRequest | null, right: UsageRequest): boolean {
  return Boolean(
    left &&
    left.installationId === right.installationId &&
    left.refreshToken === right.refreshToken &&
    left.forceRefresh === right.forceRefresh,
  );
}

function isCurrentUsageRequest(
  state: UsageRequestState,
  request: UsageRequest,
  sequence: number,
): boolean {
  return (
    !state.disposed &&
    state.sequence === sequence &&
    sameUsageRequest(state.activeRequest, request) &&
    sameUsageRequest(state.latestRequest, request)
  );
}

function errorMessage(reason: unknown): string {
  if (reason instanceof Error && reason.message) return reason.message;
  if (typeof reason === "string" && reason) return reason;

  try {
    const serialized = JSON.stringify(reason);
    return serialized && serialized !== "{}" ? serialized : "Unknown backend error";
  } catch {
    return "Unknown backend error";
  }
}
