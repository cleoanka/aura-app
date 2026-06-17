import type { ReactNode } from "react";

import type { DoctorReport } from "../lib/types";

export type ActiveView = "workspace" | "search" | "ask" | "graph" | "agents" | "settings";

type AppShellProps = {
  activeView: ActiveView;
  doctorReport: DoctorReport | null;
  noteCount: number;
  onActiveViewChange: (view: ActiveView) => void;
  children: ReactNode;
};

const navigationItems = [
  { id: "workspace", label: "Workspace", icon: "W" },
  { id: "search", label: "Arama", icon: "/" },
  { id: "ask", label: "ASK", icon: "?" },
  { id: "graph", label: "Graf", icon: "G" },
  { id: "agents", label: "Ajanlar", icon: "A" },
  { id: "settings", label: "Ayarlar", icon: "*" },
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
  const activeItem = navigationItems.find((item) => item.id === activeView);

  return (
    <div className="app-shell">
      <aside className="icon-rail" aria-label="Ana gezinme">
        <div className="rail-brand" aria-label="Aura">
          AU
        </div>
        {navigationItems.map((item) => (
          <button
            aria-label={item.label}
            aria-pressed={item.id === activeView}
            className={`rail-button ${item.id === activeView ? "is-active" : ""}`}
            key={item.id}
            onClick={() => onActiveViewChange(item.id)}
            title={item.label}
            type="button"
          >
            <span aria-hidden="true">{item.icon}</span>
          </button>
        ))}
        <div className="rail-spacer" />
      </aside>

      <main className="main-view">{children}</main>

      <footer className="status-bar" aria-label="Durum çubuğu">
        <div className="status-group" aria-label="Ajan sağlık durumu">
          {(["claude", "gemini", "codex"] as const).map((id) => (
            <span className="health-item" key={id}>
              <span className={`health-dot ${healthClass(doctorReport, id)}`} />
              <span>{id}</span>
            </span>
          ))}
        </div>
        <div className="status-group">
          <span>{activeItem?.label ?? "Workspace"}</span>
          <span>{noteCount} not</span>
        </div>
      </footer>
    </div>
  );
}
