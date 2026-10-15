import { useEffect, useState } from "react";
import { emit } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  APPEARANCE_VISIBILITY_EVENT,
  MAX_OPACITY,
  MIN_OPACITY,
  OPACITY_CHANGED_EVENT,
  RESET_SIZE_EVENT,
  readOpacity,
  storeOpacity,
} from "./appearance";
import { CloseIcon, IconButton } from "./components/IconButton";
import "./App.css";

function AppearanceWindow() {
  const [opacity, setOpacity] = useState(readOpacity);

  useEffect(() => {
    storeOpacity(opacity);
    emit(OPACITY_CHANGED_EVENT, opacity).catch(() => undefined);
  }, [opacity]);

  const hide = () => {
    getCurrentWindow()
      .hide()
      .catch(() => undefined);
    emit(APPEARANCE_VISIBILITY_EVENT, false).catch(() => undefined);
  };

  return (
    <main className="overlay-shell">
      <section className="overlay-panel appearance-panel">
        <header className="overlay-header appearance-header" data-tauri-drag-region>
          <span className="appearance-title" data-tauri-drag-region>
            Appearance
          </span>
          <div className="window-actions">
            <IconButton danger label="Close appearance" title="Close" onClick={hide}>
              <CloseIcon />
            </IconButton>
          </div>
        </header>

        <div className="appearance-controls">
          <label htmlFor="overlay-opacity">
            <span>Background opacity</span>
            <strong>{opacity}%</strong>
          </label>
          <input
            id="overlay-opacity"
            type="range"
            min={MIN_OPACITY}
            max={MAX_OPACITY}
            step="1"
            value={opacity}
            onChange={(event) => setOpacity(Number(event.target.value))}
          />
          <button
            className="appearance-reset"
            type="button"
            onClick={() => {
              emit(RESET_SIZE_EVENT).catch(() => undefined);
            }}
          >
            Reset size
          </button>
        </div>
      </section>
    </main>
  );
}

export default AppearanceWindow;
