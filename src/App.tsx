import { useEffect, useState, type CSSProperties, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ProviderCard } from "./components/ProviderCard";
import type { ProviderInstallation } from "./types/provider";
import type { RuntimeInfo } from "./types/runtime";
import "./App.css";

function App() {
  const [runtimes, setRuntimes] = useState<RuntimeInfo[]>([]);
  const [installations, setInstallations] = useState<ProviderInstallation[]>([]);
  const [refreshToken, setRefreshToken] = useState(0);
  const [controlsOpen, setControlsOpen] = useState(false);
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
    localStorage.setItem("quotify:always-on-top", String(alwaysOnTop));
    getCurrentWindow()
      .setAlwaysOnTop(alwaysOnTop)
      .catch(() => undefined);
  }, [alwaysOnTop]);

  useEffect(() => {
    localStorage.setItem("quotify:opacity", String(opacity));
  }, [opacity]);

  const overlayStyle = {
    "--overlay-opacity": opacity / 100,
  } as CSSProperties;

  const agentLabel =
    installations.length > 0
      ? installations.length +
        " " +
        (installations.length === 1 ? "agent connected" : "agents connected")
      : "Agent usage overlay";

  return (
    <main className="overlay-shell" style={overlayStyle}>
      <section className="overlay-panel">
        <header className="overlay-header" data-tauri-drag-region>
          <div className="brand" data-tauri-drag-region>
            <span className="brand-mark" aria-hidden="true">
              Q
            </span>
            <div data-tauri-drag-region>
              <h1 data-tauri-drag-region>Quotify</h1>
              <p data-tauri-drag-region>{agentLabel}</p>
            </div>
          </div>

          <div className="window-actions">
            <IconButton
              label="Refresh usage"
              title="Refresh usage"
              onClick={() => setRefreshToken((value) => value + 1)}
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

        {controlsOpen && (
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
          {installations.length > 0 ? (
            installations.map((installation) => (
              <ProviderCard
                key={installation.id}
                installation={installation}
                refreshToken={refreshToken}
              />
            ))
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

        <footer className="overlay-footer">
          <span className={"live-indicator" + (alwaysOnTop ? "" : " is-muted")} />
          <span>{alwaysOnTop ? "Pinned in the corner" : "Floating window"}</span>
          <kbd>Ctrl Shift U</kbd>
        </footer>
      </section>
    </main>
  );
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
