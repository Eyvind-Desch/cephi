import { isTauri } from "@tauri-apps/api/core";
import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
import "./i18n";
import "./styles.css";

async function start() {
  // Outside the desktop app (plain browser on the dev server) render with sample data.
  if (import.meta.env.DEV && !isTauri()) {
    const { installPreview } = await import("./dev/preview");
    installPreview();
  }

  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
}

void start();
