import type { DoctorReport } from "../lib/types";

type AppShellProps = {
  activeView: string;
  doctorReport: DoctorReport | null;
  children: React.ReactNode;
};

const navigationItems = [
  { id: "vault", label: "Vault", icon: "V" },
  { id: "search", label: "Arama", icon: "/" },
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

export function AppShell({ activeView, doctorReport, children }: AppShellProps) {
  return (
    <div className="app-shell">
      <aside className="icon-rail" aria-label="Ana gezinme">
        <div className="rail-brand" aria-label="Aura">
          AU
        </div>
        {navigationItems.map((item) => (
          <button
            aria-label={item.label}
            className={`rail-button ${item.id === "agents" ? "is-active" : ""}`}
            disabled={item.id !== "agents"}
            key={item.id}
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
          <span>{activeView}</span>
        </div>
      </footer>
    </div>
  );
}
