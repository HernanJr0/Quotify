import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App";
import { APPEARANCE_WINDOW_LABEL } from "./appearance";
import AppearanceWindow from "./AppearanceWindow";
import { DETAILS_WINDOW_LABEL } from "./details";
import DetailsWindow from "./DetailsWindow";

const windowLabel = getCurrentWindow().label;
const Root =
  windowLabel === APPEARANCE_WINDOW_LABEL
    ? AppearanceWindow
    : windowLabel === DETAILS_WINDOW_LABEL
      ? DetailsWindow
      : App;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Root />
  </React.StrictMode>,
);
