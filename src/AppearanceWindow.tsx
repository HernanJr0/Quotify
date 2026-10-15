import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { emit } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  APPEARANCE_VISIBILITY_EVENT,
  APPEARANCE_WINDOW_WIDTH,
  MAX_OPACITY,
  MIN_OPACITY,
  OPACITY_CHANGED_EVENT,
  RESET_SIZE_EVENT,
  readOpacity,
  storeOpacity,
} from "./appearance";
import { CloseIcon, IconButton } from "./components/IconButton";
import { pinWindowSize } from "./windowSize";
import "./App.css";

function AppearanceWindow() {
  const [opacity, setOpacity] = useState(readOpacity);
  const shellRef = useRef<HTMLElement>(null);

  // Size the window to its content; some platforms enforce a larger default.
  useLayoutEffect(() => {
    const shell = shellRef.current;
    if (!shell) return;

    const fit = () => {
      pinWindowSize(APPEARANCE_WINDOW_WIDTH, shell.offsetHeight).catch(() => undefined);
    };
    const observer = new ResizeObserver(fit);
    observer.observe(shell);
    return () => observer.disconnect();
  }, []);

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
    <main className="overlay-shell appearance-shell" ref={shellRef}>
      <section className="overlay-panel appearance-panel">
        <header className="appearance-header" data-tauri-drag-region>
          <span className="appearance-title" data-tauri-drag-region>
            Appearance
          </span>
          <IconButton danger label="Close appearance" title="Close" onClick={hide}>
            <CloseIcon />
          </IconButton>
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
