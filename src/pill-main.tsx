import React from "react";
import ReactDOM from "react-dom/client";
import { Pill } from "./components/Pill";
import i18n from "./i18n";
import { listen } from "@tauri-apps/api/event";
import { getConfig } from "./shared/platform";
import "./pill-styles.css";

// The pill runs in a separate Tauri webview, so it needs to load
// the saved UI language independently from the main window.
getConfig()
  .then((config) => {
    if (config.ui_language && config.ui_language !== i18n.language) {
      i18n.changeLanguage(config.ui_language);
    }
  })
  .catch(() => {});

// Listen for language changes from the main window settings.
listen<string>("ui-language-changed", (event) => {
  i18n.changeLanguage(event.payload);
});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Pill />
  </React.StrictMode>,
);
