import type { ReactNode } from "react";

import type { DoctorReport } from "../lib/types";
import { useI18n } from "../i18n";
import {
  AgentsIcon,
  AskIcon,
  AuraModeIcon,
  BrandMark,
  GraphIcon,
  SearchIcon,
  SettingsIcon,
  WorkspaceIcon,
} from "./icons";

export type ActiveView =
  | "workspace"
  | "search"
  | "ask"
  | "aura-mode"
  | "graph"
  | "agents"
  | "settings";

type AppShellProps = {
  activeView: ActiveView;
  doctorReport: DoctorReport | null;
  noteCount: number;
  onActiveViewChange: (view: ActiveView) => void;
  children: ReactNode;
};

const navigationItems = [
  { id: "workspace", key: "nav.workspace", Icon: WorkspaceIcon },
  { id: "search", key: "nav.search", Icon: SearchIcon },
  { id: "ask", key: "nav.ask", Icon: AskIcon },
  { id: "aura-mode", key: "nav.auraMode", Icon: AuraModeIcon },
  { id: "graph", key: "nav.graph", Icon: GraphIcon },
  { id: "agents", key: "nav.agents", Icon: AgentsIcon },
  { id: "settings", key: "nav.settings", Icon: SettingsIcon },
] as const;

function healthClass(report: DoctorReport | null, id: keyof DoctorReport["agents"]) {
  const agent = report?.agents[id];
  if (!agent) {
    return "";
  }
  if (agent.installed && agent.auth === "logged_in" && agent.can_invoke !== false) {
    return "is-good";
  }
  if (agent.installed || agent.auth === "rate_limited") {
    return "is-warn";
  }
  return "is-bad";
}

export function AppShell({
  activeView,
  doctorReport,
  noteCount,
  onActiveViewChange,
  children,
}: AppShellProps) {
  const { t, lang, setLang } = useI18n();
  const activeItem = navigationItems.find((item) => item.id === activeView);

  return (
    <div className="app-shell">
      <aside className="icon-rail" aria-label={t("app.name")}>
        <div className="rail-brand" aria-label={t("app.name")} title={t("app.name")}>
          <BrandMark />
        </div>
        <nav className="rail-nav">
          {navigationItems.map(({ id, key, Icon }) => {
            const label = t(key);
            return (
              <button
                aria-label={label}
                aria-pressed={id === activeView}
                className={`rail-button ${id === activeView ? "is-active" : ""}`}
                key={id}
                onClick={() => onActiveViewChange(id)}
                title={label}
                type="button"
              >
                <Icon aria-hidden="true" />
              </button>
            );
          })}
        </nav>
        <div className="rail-spacer" />
        <button
          className="rail-lang"
          onClick={() => setLang(lang === "tr" ? "en" : "tr")}
          title={lang === "tr" ? "Switch to English" : "Türkçe'ye geç"}
          type="button"
        >
          {lang === "tr" ? "EN" : "TR"}
        </button>
      </aside>

      <main className="main-view">{children}</main>

      <footer className="status-bar" aria-label="status">
        <div className="status-group" aria-label="agents">
          {(["claude", "gemini", "codex"] as const).map((id) => (
            <span className="health-item" key={id}>
              <span className={`health-dot ${healthClass(doctorReport, id)}`} />
              <span>{id}</span>
            </span>
          ))}
        </div>
        <div className="status-group">
          <span>{activeItem ? t(activeItem.key) : t("nav.workspace")}</span>
          <span>{t("workspace.notesCount", { count: noteCount })}</span>
        </div>
      </footer>
    </div>
  );
}
