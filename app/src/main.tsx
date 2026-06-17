import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { I18nProvider } from "./i18n";
import { getSettings } from "./lib/ipc";
import "./styles/theme.css";

void getSettings()
  .then((settings) => {
    document.documentElement.dataset.theme = settings.theme === "light" ? "light" : "dark";
  })
  .catch(() => {
    document.documentElement.dataset.theme = "dark";
  });

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <I18nProvider>
      <App />
    </I18nProvider>
  </React.StrictMode>,
);
