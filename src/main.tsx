import { isTauri } from "@tauri-apps/api/core";
import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
import "./i18n";
import "./styles.css";

/** The screenshot overlay is a window of its own and shares only the bundle with the app. */
const OVERLAY = new URLSearchParams(location.search).get("window") === "shot";

async function start() {
  // Outside the desktop app (plain browser on the dev server) render with sample data.
  if (import.meta.env.DEV && !isTauri()) {
    const { installPreview } = await import("./dev/preview");
    installPreview();
  }

  const root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement);
  if (OVERLAY) {
    const { RegionOverlay } = await import("./components/shot/RegionOverlay");
    root.render(
      <React.StrictMode>
        <RegionOverlay />
      </React.StrictMode>,
    );
    return;
  }

  root.render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
}

void start();
