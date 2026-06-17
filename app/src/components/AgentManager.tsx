import { useCallback, useEffect, useMemo, useState } from "react";

import { useI18n } from "../i18n";
import { agentDetect, agentInstall } from "../lib/ipc";
import type { AgentAuth, AgentId, AgentStatus, DoctorReport } from "../lib/types";
import { PtyLogin } from "./Pty/PtyLogin";

type TFn = (key: string, vars?: Record<string, string | number>) => string;

type AgentManagerProps = {
  onReportChange: (report: DoctorReport | null) => void;
};

type InstallState = {
  kind: "idle" | "working" | "success" | "error";
  message: string;
};

type AgentDefinition = {
  id: AgentId;
  name: string;
  role: string;
};

const agentDefinitions: AgentDefinition[] = [
  {
    id: "claude",
    name: "Claude",
    role: "Router, mimar ve inceleyici",
  },
  {
    id: "gemini",
    name: "Gemini",
    role: "Araştırma",
  },
  {
    id: "codex",
    name: "Codex",
    role: "Uygulama",
  },
];

const emptyInstallState: Record<AgentId, InstallState> = {
  claude: { kind: "idle", message: "" },
  gemini: { kind: "idle", message: "" },
  codex: { kind: "idle", message: "" },
};

function friendlyFailure(action: "detect" | "install", t: TFn) {
  if (action === "detect") {
    return t("common.error");
  }

  return t("common.error");
}

function authLabel(auth: AgentAuth, t: TFn) {
  switch (auth) {
    case "logged_in":
      return t("agents.auth.loggedIn");
    case "logged_out":
      return t("agents.auth.loggedOut");
    case "rate_limited":
      return t("agents.auth.rateLimited");
    case "unknown":
      return t("agents.auth.unknown");
  }
}

function authClass(auth: AgentAuth) {
  switch (auth) {
    case "logged_in":
      return "good";
    case "rate_limited":
      return "warn";
    case "logged_out":
      return "bad";
    case "unknown":
      return "";
  }
}

function boolLabel(value: boolean | null, t: TFn) {
  if (value === true) {
    return t("common.ok");
  }

  if (value === false) {
    return t("agents.auth.unknown");
  }

  return t("agents.auth.unknown");
}

function tokenLocationLabel(value: AgentStatus["token_location"], t: TFn) {
  switch (value) {
    case "keychain":
      return "Keychain";
    case "file":
      return "Dosya";
    case "unknown":
      return t("agents.auth.unknown");
  }
}

function fallbackStatus(): AgentStatus {
  return {
    installed: false,
    path: null,
    version: null,
    auth: "unknown",
    token_location: "unknown",
    can_invoke: null,
    last_error: null,
  };
}

export function AgentManager({ onReportChange }: AgentManagerProps) {
  const { t } = useI18n();
  const [report, setReport] = useState<DoctorReport | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [installState, setInstallState] =
    useState<Record<AgentId, InstallState>>(emptyInstallState);
  const [loginAgent, setLoginAgent] = useState<AgentId | null>(null);

  const installedCount = useMemo(() => {
    if (!report) {
      return 0;
    }

    return agentDefinitions.filter(({ id }) => report.agents[id].installed).length;
  }, [report]);

  const refresh = useCallback(
    async (probe = false) => {
      setError(null);
      setRefreshing(true);

      try {
        const nextReport = await agentDetect(probe);
        setReport(nextReport);
        onReportChange(nextReport);
      } catch {
        setError(friendlyFailure("detect", t));
        setReport(null);
        onReportChange(null);
      } finally {
        setLoading(false);
        setRefreshing(false);
      }
    },
    [onReportChange, t],
  );

  useEffect(() => {
    refresh(false);
  }, [refresh]);

  const install = async (id: AgentId) => {
    setInstallState((current) => ({
      ...current,
      [id]: { kind: "working", message: t("common.loading") },
    }));

    try {
      const message = await agentInstall(id);
      setInstallState((current) => ({
        ...current,
        [id]: {
          kind: "success",
          message: message || t("agents.installed"),
        },
      }));
      await refresh(true);
    } catch {
      setInstallState((current) => ({
        ...current,
        [id]: { kind: "error", message: friendlyFailure("install", t) },
      }));
    }
  };

  const closeLogin = useCallback(() => {
    setLoginAgent(null);
    void refresh(true);
  }, [refresh]);

  return (
    <section className="agent-manager" aria-labelledby="agent-manager-title">
      <header className="manager-header">
        <div>
          <p className="eyebrow">{t("agents.title")}</p>
          <h1 id="agent-manager-title">{t("agents.title")}</h1>
          <p>
            Claude ana beyin olarak yönlendirme ve mimari kararları taşır; Gemini
            araştırma, Codex ise uygulama işleri için hazır tutulur.
          </p>
        </div>

        <div className="toolbar" aria-label={t("agents.title")}>
          <span className="badge accent">{t("agents.installed")}: {installedCount}/3</span>
          <button
            aria-label={t("graph.refresh")}
            className="button primary"
            disabled={refreshing}
            onClick={() => refresh(true)}
            type="button"
          >
            {refreshing ? t("common.loading") : t("graph.refresh")}
          </button>
        </div>
      </header>

      {loading ? <p className="notice">{t("common.loading")}</p> : null}
      {error ? <p className="notice error">{error}</p> : null}

      <div className="agent-grid">
        {agentDefinitions.map((definition) => {
          const status = report?.agents[definition.id] ?? fallbackStatus();
          const currentInstall = installState[definition.id];
          const isInstalling = currentInstall.kind === "working";

          return (
            <article
              className={`agent-card ${definition.id === "claude" ? "is-primary" : ""}`}
              key={definition.id}
            >
              <div className="agent-card-header">
                <div className="agent-title">
                  <h2 className="agent-name">{definition.name}</h2>
                  <p className="agent-role">{definition.role}</p>
                </div>
                {definition.id === "claude" ? (
                  <span className="badge accent">{t("agents.brainBadge")}</span>
                ) : null}
              </div>

              <div className="badge-row" aria-label={`${definition.name} ${t("status.ready")}`}>
                <span className={`badge ${status.installed ? "good" : ""}`}>
                  {status.installed ? t("agents.installed") : t("agents.notInstalled")}
                </span>
                <span className={`badge ${authClass(status.auth)}`}>
                  {authLabel(status.auth, t)}
                </span>
              </div>

              <dl className="details">
                <div className="detail-row">
                  <dt className="detail-label">Sürüm</dt>
                  <dd className={`detail-value ${status.version ? "" : "muted"}`}>
                    {status.version ?? t("agents.auth.unknown")}
                  </dd>
                </div>
                <div className="detail-row">
                  <dt className="detail-label">{t("agents.tokenLocation")}</dt>
                  <dd className="detail-value">{tokenLocationLabel(status.token_location, t)}</dd>
                </div>
                <div className="detail-row">
                  <dt className="detail-label">Çalıştırma</dt>
                  <dd className="detail-value">{boolLabel(status.can_invoke, t)}</dd>
                </div>
                <div className="detail-row">
                  <dt className="detail-label">Yol</dt>
                  <dd className={`detail-value mono ${status.path ? "" : "muted"}`}>
                    {status.path ?? t("workspace.noNotes")}
                  </dd>
                </div>
                <div className="detail-row">
                  <dt className="detail-label">Son durum</dt>
                  <dd className={`detail-value ${status.last_error ? "" : "muted"}`}>
                    {status.last_error
                      ? t("common.error")
                      : t("status.ready")}
                  </dd>
                </div>
              </dl>

              <div className="card-actions">
                <button
                  aria-label={`${definition.name} ${t("agents.install")}`}
                  className="button primary"
                  disabled={status.installed || isInstalling}
                  onClick={() => install(definition.id)}
                  type="button"
                >
                  {isInstalling ? t("common.loading") : t("agents.install")}
                </button>
                <button
                  aria-label={`${definition.name} ${t("agents.retry")}`}
                  className="button"
                  disabled={refreshing}
                  onClick={() => refresh(true)}
                  type="button"
                >
                  {t("agents.retry")}
                </button>
                <button
                  aria-label={`${definition.name} ${t("agents.login")}`}
                  className="button ghost"
                  onClick={() => setLoginAgent(definition.id)}
                  type="button"
                >
                  {t("agents.login")}
                </button>
              </div>

              <p className={`inline-result ${currentInstall.kind}`}>
                {currentInstall.message}
              </p>
            </article>
          );
        })}
      </div>

      {loginAgent ? <PtyLogin agent={loginAgent} onClose={closeLogin} /> : null}
    </section>
  );
}
