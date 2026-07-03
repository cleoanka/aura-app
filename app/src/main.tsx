import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { I18nProvider } from "./i18n";
import { getSettings } from "./lib/ipc";
import "./styles/theme.css";

// FOUC fix: gerçek kaynak (settings) gelene dek son bilinen temayı senkron uygula.
try {
  document.documentElement.dataset.theme = localStorage.getItem("aura.theme") ?? "dark";
} catch {
  document.documentElement.dataset.theme = "dark";
}

void getSettings()
  .then((settings) => {
    const theme = settings.theme === "light" ? "light" : "dark";
    document.documentElement.dataset.theme = theme;
    try {
      localStorage.setItem("aura.theme", theme);
    } catch {
      /* yoksay */
    }
  })
  .catch(() => {
    /* senkron fallback zaten uygulandı; son bilinen temayı ezme */
  });

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <I18nProvider>
      <App />
    </I18nProvider>
  </React.StrictMode>,
);
