import { useCallback, useEffect, useState, type CSSProperties, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { ProviderButton } from "./components/ProviderButton";
import type { ProviderInstallation } from "./types/provider";
import type { RuntimeInfo } from "./types/runtime";
import type { ProviderUsage } from "./types/usage";
import "./App.css";

const AUTO_REFRESH_INTERVAL_MS = 2 * 60 * 1000;

function App() {
  const [runtimes, setRuntimes] = useState<RuntimeInfo[]>([]);
  const [installations, setInstallations] = useState<ProviderInstallation[]>([]);
  const [refreshRequest, setRefreshRequest] = useState({ token: 0, force: false });
  const [controlsOpen, setControlsOpen] = useState(false);
  const [windowHovered, setWindowHovered] = useState(false);
  const [hoveredProviderId, setHoveredProviderId] = useState<string | null>(null);
  const [usageByInstallation, setUsageByInstallation] = useState<
    Record<string, ProviderUsage | null>
  >({});
  const [alwaysOnTop, setAlwaysOnTop] = useState(
    () => localStorage.getItem("quotify:always-on-top") !== "false",
  );
  const [opacity, setOpacity] = useState(() => {
    const stored = Number(localStorage.getItem("quotify:opacity"));
    return Number.isFinite(stored) && stored >= 35 && stored <= 100 ? stored : 88;
  });

  useEffect(() => {
    invoke<RuntimeInfo[]>("list_runtimes")
      .then(setRuntimes)
      .catch(() => setRuntimes([]));
    invoke<ProviderInstallation[]>("list_providers")
      .then(setInstallations)
      .catch(() => setInstallations([]));
  }, []);

  useEffect(() => {
    const interval = window.setInterval(() => {
      setRefreshRequest((current) => ({ token: current.token + 1, force: false }));
    }, AUTO_REFRESH_INTERVAL_MS);

    return () => window.clearInterval(interval);
  }, []);

  useEffect(() => {
    localStorage.setItem("quotify:always-on-top", String(alwaysOnTop));
    getCurrentWindow()
      .setAlwaysOnTop(alwaysOnTop)
      .catch(() => undefined);
  }, [alwaysOnTop]);

  useEffect(() => {
    localStorage.setItem("quotify:opacity", String(opacity));
  }, [opacity]);

  const handleUsageLoaded = useCallback((installationId: string, usage: ProviderUsage | null) => {
    setUsageByInstallation((current) => ({ ...current, [installationId]: usage }));
  }, []);

  const displayModel = mergeInstallations(installations, usageByInstallation);
  const displayInstallations = displayModel.groups;

  const compactMode = alwaysOnTop && !windowHovered;
  const showControls = controlsOpen && !compactMode;
  const rows = Math.max(1, Math.ceil(displayInstallations.length / 4));
  const providerAreaHeight = displayInstallations.length > 0 ? rows * 65 + 14 : 82;
  const detailsHeight = hoveredProviderId ? 72 : 0;
  const windowHeight = Math.min(
    460,
    providerAreaHeight + (compactMode ? 0 : 41) + (showControls ? 48 : 0) + detailsHeight,
  );

  useEffect(() => {
    getCurrentWindow()
      .setSize(new LogicalSize(360, windowHeight))
      .catch(() => undefined);
  }, [windowHeight]);

  const overlayStyle = {
    "--overlay-opacity": opacity / 100,
  } as CSSProperties;

  return (
    <main
      className="overlay-shell"
      style={overlayStyle}
      onMouseEnter={() => setWindowHovered(true)}
      onMouseLeave={() => setWindowHovered(false)}
      onFocusCapture={() => setWindowHovered(true)}
    >
      <section className={"overlay-panel" + (compactMode ? " is-compact" : "")}>
        <header className="overlay-header" data-tauri-drag-region>
          <div className="brand" data-tauri-drag-region>
            <div data-tauri-drag-region>
              <h1 data-tauri-drag-region>Quotify</h1>
            </div>
            {displayInstallations.length > 0 && (
              <span className="provider-count" data-tauri-drag-region>
                {displayInstallations.length}
              </span>
            )}
          </div>

          <div className="window-actions">
            <IconButton
              label="Refresh usage"
              title="Refresh usage now · automatic every 2 minutes"
              onClick={() =>
                setRefreshRequest((current) => ({ token: current.token + 1, force: true }))
              }
            >
              <RefreshIcon />
            </IconButton>
            <IconButton
              active={alwaysOnTop}
              label="Keep overlay on top"
              title={alwaysOnTop ? "Always on top" : "Keep on top"}
              pressed={alwaysOnTop}
              onClick={() => setAlwaysOnTop((value) => !value)}
            >
              <PinIcon />
            </IconButton>
            <IconButton
              active={controlsOpen}
              label="Overlay appearance"
              title="Appearance"
              expanded={controlsOpen}
              onClick={() => setControlsOpen((value) => !value)}
            >
              <SlidersIcon />
            </IconButton>
            <IconButton
              label="Hide overlay"
              title="Hide overlay"
              onClick={() =>
                getCurrentWindow()
                  .hide()
                  .catch(() => undefined)
              }
            >
              <CloseIcon />
            </IconButton>
          </div>
        </header>

        {showControls && (
          <div className="appearance-controls">
            <label htmlFor="overlay-opacity">
              <span>Background opacity</span>
              <strong>{opacity}%</strong>
            </label>
            <input
              id="overlay-opacity"
              type="range"
              min="35"
              max="100"
              step="1"
              value={opacity}
              onChange={(event) => setOpacity(Number(event.target.value))}
            />
          </div>
        )}

        <div className="provider-grid">
          {displayInstallations.length > 0 ? (
            installations.map((installation) => {
              const display = displayModel.byInstallationId.get(installation.id);
              if (!display) return null;

              return (
                <ProviderButton
                  key={installation.id}
                  displayRuntimeName={display.runtimeName}
                  displayUsage={display.usage}
                  hidden={display.installation.id !== installation.id}
                  installation={installation}
                  onUsageLoaded={handleUsageLoaded}
                  forceRefresh={refreshRequest.force}
                  refreshToken={refreshRequest.token}
                  onHoverChange={(isHovered) =>
                    setHoveredProviderId(isHovered ? installation.id : null)
                  }
                  tooltipAlignment={tooltipAlignment(display.index)}
                />
              );
            })
          ) : (
            <div className="empty-state">
              <span className="empty-state-dot" />
              <div>
                <strong>No agents detected</strong>
                <p>{runtimes.map((runtime) => runtime.name).join(" · ") || "No environments"}</p>
              </div>
            </div>
          )}
        </div>
      </section>
    </main>
  );
}

interface DisplayInstallation {
  installation: ProviderInstallation;
  index: number;
  runtimeName: string;
  usage: ProviderUsage | null | undefined;
}

interface DisplayModel {
  byInstallationId: Map<string, DisplayInstallation>;
  groups: DisplayInstallation[];
}

function mergeInstallations(
  installations: ProviderInstallation[],
  usageByInstallation: Record<string, ProviderUsage | null>,
): DisplayModel {
  const merged = new Map<string, DisplayInstallation>();
  const byInstallationId = new Map<string, DisplayInstallation>();

  for (const installation of installations) {
    const usage = usageByInstallation[installation.id];
    const mergeKey = consolidationKey(installation, usage);
    const current = merged.get(mergeKey);

    if (!current) {
      const display = {
        installation,
        index: merged.size,
        runtimeName: installation.runtimeName,
        usage,
      };
      merged.set(mergeKey, display);
      byInstallationId.set(installation.id, display);
      continue;
    }

    current.runtimeName += ` + ${installation.runtimeName}`;
    current.usage = preferredUsage(current.usage, usage);
    byInstallationId.set(installation.id, current);
  }

  return {
    byInstallationId,
    groups: [...merged.values()],
  };
}

function consolidationKey(
  installation: ProviderInstallation,
  usage: ProviderUsage | null | undefined,
): string {
  if (!usage?.accountKey) {
    return installation.id;
  }

  // The opaque key is issued only after the provider confirms the local
  // credentials. It remains valid even when that account has no active quota.
  return [installation.provider, usage.accountKey].join(":");
}

function preferredUsage(
  current: ProviderUsage | null | undefined,
  candidate: ProviderUsage | null | undefined,
): ProviderUsage | null | undefined {
  if (candidate?.percentage != null && current?.percentage == null) return candidate;
  return current ?? candidate;
}

function tooltipAlignment(index: number): "start" | "center" | "end" {
  const column = index % 4;
  if (column === 0) return "start";
  if (column === 3) return "end";
  return "center";
}

interface IconButtonProps {
  active?: boolean;
  children: ReactNode;
  expanded?: boolean;
  label: string;
  onClick: () => void;
  pressed?: boolean;
  title: string;
}

function IconButton({
  active = false,
  children,
  expanded,
  label,
  onClick,
  pressed,
  title,
}: IconButtonProps) {
  return (
    <button
      className={"icon-button" + (active ? " is-active" : "")}
      type="button"
      aria-label={label}
      aria-expanded={expanded}
      aria-pressed={pressed}
      title={title}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

function RefreshIcon() {
  return (
    <svg viewBox="0 0 20 20" aria-hidden="true">
      <path d="M15.7 6.1A6.2 6.2 0 1 0 16 13" />
      <path d="M15.7 2.8v3.7h-3.8" />
    </svg>
  );
}

function PinIcon() {
  return (
    <svg viewBox="0 0 20 20" aria-hidden="true">
      <path d="m7.1 3.5 5.8 5.8M11.8 2.8l5.4 5.4-3 1.1-2.7 2.7-1 4.2-3.4-3.4 4.2-1 2.7-2.7 1.1-3Z" />
      <path d="m3.2 16.8 4.1-4.1" />
    </svg>
  );
}

function SlidersIcon() {
  return (
    <svg viewBox="0 0 20 20" aria-hidden="true">
      <path d="M4 5h5M13 5h3M4 10h2M10 10h6M4 15h7M15 15h1" />
      <circle cx="11" cy="5" r="1.7" />
      <circle cx="8" cy="10" r="1.7" />
      <circle cx="13" cy="15" r="1.7" />
    </svg>
  );
}

function CloseIcon() {
  return (
    <svg viewBox="0 0 20 20" aria-hidden="true">
      <path d="m5.5 5.5 9 9M14.5 5.5l-9 9" />
    </svg>
  );
}

export default App;
