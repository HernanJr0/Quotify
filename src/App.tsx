import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type CSSProperties,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { emitTo, listen } from "@tauri-apps/api/event";
import {
  currentMonitor,
  getCurrentWindow,
  LogicalSize,
  PhysicalPosition,
  Window,
} from "@tauri-apps/api/window";
import {
  APPEARANCE_VISIBILITY_EVENT,
  APPEARANCE_WINDOW_LABEL,
  APPEARANCE_WINDOW_WIDTH,
  OPACITY_CHANGED_EVENT,
  RESET_SIZE_EVENT,
  readOpacity,
} from "./appearance";
import { CloseIcon, IconButton } from "./components/IconButton";
import {
  DETAILS_CARD_WIDTH,
  DETAILS_CHANGED_EVENT,
  DETAILS_SHADOW_MARGIN,
  DETAILS_WINDOW_LABEL,
  type ProviderDetails,
} from "./details";
import { ProviderButton } from "./components/ProviderButton";
import type { ProviderInstallation } from "./types/provider";
import type { RuntimeInfo } from "./types/runtime";
import type { ProviderUsage } from "./types/usage";
import "./App.css";

const AUTO_REFRESH_INTERVAL_MS = 2 * 60 * 1000;
const DEFAULT_WINDOW_WIDTH = 360;
const MIN_WINDOW_WIDTH = 260;
const MAX_WINDOW_WIDTH = 4096;
const GRID_ITEM_WIDTH = 64;
const GRID_COLUMN_GAP = 10;
const GRID_HORIZONTAL_CHROME = 28;
const GRID_ITEM_HEIGHT = 65;
const GRID_ROW_GAP = 4;
const EMPTY_STATE_HEIGHT = 105;
// Shell padding (2 × 5px) + panel border (2 × 1px) + grid padding (2 × 7px).
const PANEL_VERTICAL_CHROME = 26;
const MAX_WINDOW_HEIGHT = 460;

type DiscoveryStatus = "loading" | "ready" | "error";

interface DiscoveryRunState {
  activeToken: number | null;
  disposed: boolean;
  inFlight: boolean;
  latestToken: number;
  pendingToken: number | null;
  sequence: number;
}

function App() {
  const [runtimes, setRuntimes] = useState<RuntimeInfo[]>([]);
  const [installations, setInstallations] = useState<ProviderInstallation[]>([]);
  const [discoveryStatus, setDiscoveryStatus] = useState<DiscoveryStatus>("loading");
  const [discoveryError, setDiscoveryError] = useState<string | null>(null);
  const [discoveryRequest, setDiscoveryRequest] = useState(0);
  const [refreshRequest, setRefreshRequest] = useState({ token: 0, force: false });
  const [controlsOpen, setControlsOpen] = useState(false);
  const [actionsExpanded, setActionsExpanded] = useState(
    () => localStorage.getItem("quotify:actions-expanded") === "true",
  );
  const [usageByInstallation, setUsageByInstallation] = useState<
    Record<string, ProviderUsage | null>
  >({});
  const [alwaysOnTop, setAlwaysOnTop] = useState(
    () => localStorage.getItem("quotify:always-on-top") !== "false",
  );
  const [opacity, setOpacity] = useState(readOpacity);
  const [manualWindowWidth, setManualWindowWidth] = useState<number | null>(
    readWindowWidthPreference,
  );
  const applyingLayoutResize = useRef(false);
  const discoveryRun = useRef<DiscoveryRunState>({
    activeToken: null,
    disposed: false,
    inFlight: false,
    latestToken: 0,
    pendingToken: null,
    sequence: 0,
  });

  useEffect(() => {
    const run = discoveryRun.current;
    run.disposed = false;
    run.latestToken = discoveryRequest;
    const runDiscovery = async (requestToken: number): Promise<void> => {
      if (run.disposed || run.inFlight) return;

      run.inFlight = true;
      run.activeToken = requestToken;
      const sequence = ++run.sequence;
      if (requestToken === run.latestToken) {
        setDiscoveryStatus("loading");
        setDiscoveryError(null);
      }

      try {
        const [runtimesResult, installationsResult] = await Promise.allSettled([
          invoke<RuntimeInfo[]>("list_runtimes"),
          invoke<ProviderInstallation[]>("list_providers"),
        ]);

        const currentRun = run;
        if (
          !currentRun.disposed &&
          currentRun.sequence === sequence &&
          currentRun.latestToken === requestToken
        ) {
          const errors: string[] = [];
          if (runtimesResult.status === "fulfilled") {
            setRuntimes(runtimesResult.value);
          } else {
            errors.push(`runtimes: ${friendlyDiscoveryError(errorMessage(runtimesResult.reason))}`);
          }

          if (installationsResult.status === "fulfilled") {
            setInstallations(installationsResult.value);
          } else {
            errors.push(
              `providers: ${friendlyDiscoveryError(errorMessage(installationsResult.reason))}`,
            );
          }

          if (errors.length > 0) {
            setDiscoveryStatus("error");
            setDiscoveryError(errors.join(" · "));
          } else {
            setDiscoveryStatus("ready");
            setDiscoveryError(null);
          }
        }
      } finally {
        if (run.sequence === sequence) {
          run.inFlight = false;
          run.activeToken = null;
          const pendingToken = run.pendingToken;
          run.pendingToken = null;
          if (pendingToken !== null && !run.disposed) {
            void runDiscovery(pendingToken);
          }
        }
      }
    };

    if (run.inFlight) {
      if (run.activeToken !== discoveryRequest) {
        run.pendingToken = discoveryRequest;
      }
    } else {
      void runDiscovery(discoveryRequest);
    }

    return () => {
      run.disposed = true;
      run.pendingToken = null;
    };
  }, [discoveryRequest]);

  useEffect(() => {
    const interval = window.setInterval(() => {
      setRefreshRequest((current) => ({ token: current.token + 1, force: false }));
    }, AUTO_REFRESH_INTERVAL_MS);

    return () => window.clearInterval(interval);
  }, []);

  useEffect(() => {
    localStorage.setItem("quotify:actions-expanded", String(actionsExpanded));
  }, [actionsExpanded]);

  useEffect(() => {
    localStorage.setItem("quotify:always-on-top", String(alwaysOnTop));
    getCurrentWindow()
      .setAlwaysOnTop(alwaysOnTop)
      .catch(logWindowError);
  }, [alwaysOnTop]);

  useEffect(() => {
    // The appearance window persists its own changes; this window only
    // mirrors them live.
    const subscriptions = [
      listen<number>(OPACITY_CHANGED_EVENT, ({ payload }) => setOpacity(payload)),
      listen(RESET_SIZE_EVENT, () => setManualWindowWidth(null)),
      listen<boolean>(APPEARANCE_VISIBILITY_EVENT, ({ payload }) => setControlsOpen(payload)),
    ];

    return () => {
      for (const subscription of subscriptions) {
        subscription.then((unlisten) => unlisten()).catch(() => undefined);
      }
    };
  }, []);

  const detailsSequence = useRef(0);
  const handleDetailsChange = useCallback(
    (details: ProviderDetails | null, anchor?: DOMRect) => {
      const sequence = ++detailsSequence.current;
      void (async () => {
        const detailsWindow = await Window.getByLabel(DETAILS_WINDOW_LABEL);
        if (!detailsWindow) return;

        if (!details || !anchor) {
          await detailsWindow.hide();
          return;
        }

        // Anchor the card just below the hovered orb, centered on it, and keep
        // it inside the monitor's work area.
        const mainWindow = getCurrentWindow();
        const [origin, scale, monitor] = await Promise.all([
          mainWindow.innerPosition(),
          mainWindow.scaleFactor(),
          currentMonitor(),
        ]);
        if (sequence !== detailsSequence.current) return;

        const windowWidth = (DETAILS_CARD_WIDTH + DETAILS_SHADOW_MARGIN * 2) * scale;
        let x = origin.x + (anchor.left + anchor.width / 2) * scale - windowWidth / 2;
        const y = origin.y + (anchor.bottom + 2 - DETAILS_SHADOW_MARGIN) * scale;
        if (monitor) {
          const area = monitor.workArea;
          x = Math.min(Math.max(x, area.position.x), area.position.x + area.size.width - windowWidth);
        }

        await emitTo(DETAILS_WINDOW_LABEL, DETAILS_CHANGED_EVENT, details);
        await detailsWindow.setPosition(new PhysicalPosition(Math.round(x), Math.round(y)));
        if (sequence !== detailsSequence.current) return;
        await detailsWindow.show();
        // The card is purely informational; let clicks fall through to
        // whatever is underneath it. This must run after the first show: on
        // Linux, tao panics when the native window has not been realized yet.
        await detailsWindow.setIgnoreCursorEvents(true);
      })().catch(logWindowError);
    },
    [],
  );

  const toggleAppearanceWindow = useCallback(async () => {
    const appearanceWindow = await Window.getByLabel(APPEARANCE_WINDOW_LABEL);
    if (!appearanceWindow) return;

    if (await appearanceWindow.isVisible()) {
      await appearanceWindow.hide();
      setControlsOpen(false);
      return;
    }

    // Open the panel just below the overlay, right-aligned with it.
    const mainWindow = getCurrentWindow();
    // A window that has never been shown can report a zero size, so align
    // with the panel's known width instead of measuring it.
    const [position, size, scale] = await Promise.all([
      mainWindow.outerPosition(),
      mainWindow.outerSize(),
      mainWindow.scaleFactor(),
    ]);
    await appearanceWindow.setPosition(
      new PhysicalPosition(
        Math.round(position.x + size.width - APPEARANCE_WINDOW_WIDTH * scale),
        Math.round(position.y + size.height + 6 * scale),
      ),
    );
    await appearanceWindow.show();
    await appearanceWindow.setFocus();
    setControlsOpen(true);
  }, []);

  useEffect(() => {
    if (manualWindowWidth) {
      localStorage.setItem("quotify:window-size", JSON.stringify({ width: manualWindowWidth }));
    } else {
      localStorage.removeItem("quotify:window-size");
    }
  }, [manualWindowWidth]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;

    getCurrentWindow()
      .onResized(({ payload }) => {
        if (applyingLayoutResize.current) return;

        const scaleFactor = window.devicePixelRatio || 1;
        setManualWindowWidth(Math.round(payload.width / scaleFactor));
      })
      .then((nextUnlisten) => {
        if (disposed) nextUnlisten();
        else unlisten = nextUnlisten;
      })
      .catch(() => undefined);

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  const handleUsageLoaded = useCallback((installationId: string, usage: ProviderUsage | null) => {
    setUsageByInstallation((current) => ({ ...current, [installationId]: usage }));
  }, []);

  const requestDiscovery = useCallback(() => {
    setDiscoveryRequest((current) => current + 1);
  }, []);

  const displayModel = mergeInstallations(installations, usageByInstallation);
  const displayInstallations = displayModel.groups;

  const activeWindowWidth = manualWindowWidth ?? DEFAULT_WINDOW_WIDTH;
  const columns = Math.max(
    1,
    Math.floor(
      (activeWindowWidth - GRID_HORIZONTAL_CHROME + GRID_COLUMN_GAP) /
        (GRID_ITEM_WIDTH + GRID_COLUMN_GAP),
    ),
  );
  const rows = Math.max(1, Math.ceil(displayInstallations.length / columns));
  const providerAreaHeight =
    displayInstallations.length > 0
      ? rows * GRID_ITEM_HEIGHT + (rows - 1) * GRID_ROW_GAP
      : EMPTY_STATE_HEIGHT;
  // The action buttons float over the content, so no row is reserved for them.
  const windowHeight = Math.min(MAX_WINDOW_HEIGHT, providerAreaHeight + PANEL_VERTICAL_CHROME);

  useEffect(() => {
    const appWindow = getCurrentWindow();
    applyingLayoutResize.current = true;

    // The height always follows the content; only the width is user-resizable.
    Promise.all([
      appWindow.setMinSize(new LogicalSize(MIN_WINDOW_WIDTH, windowHeight)),
      appWindow.setMaxSize(new LogicalSize(MAX_WINDOW_WIDTH, windowHeight)),
    ])
      .then(() => appWindow.setSize(new LogicalSize(activeWindowWidth, windowHeight)))
      .catch(logWindowError)
      .finally(() => {
        window.requestAnimationFrame(() => {
          applyingLayoutResize.current = false;
        });
      });
  }, [activeWindowWidth, windowHeight]);

  const overlayStyle = {
    "--overlay-opacity": opacity / 100,
  } as CSSProperties;

  return (
    <main className="overlay-shell" style={overlayStyle}>
      <section className="overlay-panel" data-tauri-drag-region>
        <div className={"window-actions" + (actionsExpanded ? " is-expanded" : "")}>
          <div className="window-actions-extra" aria-hidden={!actionsExpanded}>
            <IconButton
              label="Refresh usage"
              tabIndex={actionsExpanded ? undefined : -1}
              title={
                discoveryStatus === "error"
                  ? "Retry usage and discovery"
                  : "Refresh usage now · automatic every 2 minutes"
              }
              onClick={() => {
                setRefreshRequest((current) => ({ token: current.token + 1, force: true }));
                if (discoveryStatus === "error") requestDiscovery();
              }}
            >
              <RefreshIcon />
            </IconButton>
            <IconButton
              active={alwaysOnTop}
              label="Keep overlay on top"
              tabIndex={actionsExpanded ? undefined : -1}
              title={alwaysOnTop ? "Always on top" : "Keep on top"}
              pressed={alwaysOnTop}
              onClick={() => setAlwaysOnTop((value) => !value)}
            >
              <PinIcon />
            </IconButton>
            <IconButton
              active={controlsOpen}
              label="Overlay appearance"
              tabIndex={actionsExpanded ? undefined : -1}
              title="Appearance"
              expanded={controlsOpen}
              onClick={() => {
                toggleAppearanceWindow().catch(logWindowError);
              }}
            >
              <SlidersIcon />
            </IconButton>
          </div>
          <IconButton
            className="window-actions-toggle"
            label={actionsExpanded ? "Hide actions" : "Show actions"}
            title={actionsExpanded ? "Hide actions" : "More actions"}
            expanded={actionsExpanded}
            onClick={() => setActionsExpanded((value) => !value)}
          >
            <ChevronIcon />
          </IconButton>
          <IconButton
            danger
            label="Hide overlay"
            title="Hide overlay"
            onClick={() => {
              Window.getByLabel(APPEARANCE_WINDOW_LABEL)
                .then((appearanceWindow) => appearanceWindow?.hide())
                .catch(logWindowError);
              setControlsOpen(false);
              handleDetailsChange(null);
              getCurrentWindow().hide().catch(logWindowError);
            }}
          >
            <CloseIcon />
          </IconButton>
        </div>

        <div className="provider-grid" data-tauri-drag-region>
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
                  onDetailsChange={handleDetailsChange}
                />
              );
            })
          ) : (
            <div className="empty-state">
              <span className="empty-state-dot" />
              <div>
                {discoveryStatus === "loading" ? (
                  <>
                    <strong>Discovering agents…</strong>
                    <p>Checking available environments</p>
                  </>
                ) : discoveryStatus === "error" ? (
                  <>
                    <strong>Unable to discover agents</strong>
                    <p>{friendlyDiscoveryError(discoveryError ?? "Discovery failed")}</p>
                    <button className="appearance-reset" type="button" onClick={requestDiscovery}>
                      Retry discovery
                    </button>
                  </>
                ) : (
                  <>
                    <strong>No agents detected</strong>
                    <p>
                      {runtimes.map((runtime) => runtime.name).join(" · ") || "No environments"}
                    </p>
                  </>
                )}
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
  if (!current) return candidate;

  // Two installations of the same account (e.g. native + WSL) can each
  // return a partial view of the rate limits. Keep whichever carries the
  // primary percentage, but don't let it shadow a weekly window the other
  // installation reported.
  if (!current.weekly && candidate?.weekly) {
    return { ...current, weekly: candidate.weekly };
  }
  return current;
}

function readWindowWidthPreference(): number | null {
  try {
    const stored = JSON.parse(localStorage.getItem("quotify:window-size") ?? "null") as unknown;
    if (
      stored &&
      typeof stored === "object" &&
      "width" in stored &&
      typeof stored.width === "number" &&
      stored.width >= MIN_WINDOW_WIDTH
    ) {
      return stored.width;
    }
  } catch {
    // A malformed local preference should not block the overlay from opening.
  }

  return null;
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

function friendlyDiscoveryError(value: string): string {
  return value.replace(/^Error:\s*/i, "").replace(/^Unavailable:\s*/i, "");
}

function logWindowError(reason: unknown): void {
  console.error("[quotify] window operation failed:", reason);
}

function ChevronIcon() {
  return (
    <svg viewBox="0 0 20 20" aria-hidden="true">
      <path d="m12 5-5 5 5 5" />
    </svg>
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

export default App;
