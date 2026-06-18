import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { useI18n } from "../../i18n";
import {
  agentDetect,
  agentInstall,
  embeddingStatus,
  ollamaPull,
  ollamaStatus,
  prepareEmbeddingModel,
  type EmbeddingStatus,
  type OllamaStatus,
} from "../../lib/ipc";
import type { AgentId, AgentStatus, DoctorReport } from "../../lib/types";
import { PtyLogin } from "../Pty/PtyLogin";

type TFn = (key: string, vars?: Record<string, string | number>) => string;

type ModelManagerProps = {
  onReportChange: (report: DoctorReport | null) => void;
};

type AgentDefinition = {
  id: AgentId;
  name: string;
  roleKey: string;
};

type CtaKind = "install" | "login" | "ready" | "retry";

const agentDefinitions: AgentDefinition[] = [
  { id: "claude", name: "Claude", roleKey: "agents.role.claude" },
  { id: "gemini", name: "Gemini", roleKey: "agents.role.gemini" },
  { id: "codex", name: "Codex", roleKey: "agents.role.codex" },
];

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

function agentCta(status: AgentStatus): CtaKind {
  if (!status.installed) {
    return "install";
  }
  if (status.auth === "logged_in") {
    return "ready";
  }
  if (status.auth === "logged_out") {
    return "login";
  }
  return "retry";
}

function statusBadge(status: AgentStatus, t: TFn) {
  if (status.installed && status.auth === "logged_in") {
    return { cls: "good", label: t("status.ready") };
  }
  if (!status.installed) {
    return { cls: "", label: t("agents.notInstalled") };
  }
  if (status.auth === "logged_out") {
    return { cls: "bad", label: t("agents.auth.loggedOut") };
  }
  if (status.auth === "rate_limited") {
    return { cls: "warn", label: t("agents.auth.rateLimited") };
  }
  return { cls: "", label: t("agents.auth.unknown") };
}

function tailLines(text: string, max = 12): string {
  const lines = text.split("\n").filter((line) => line.trim().length > 0);
  return lines.slice(-max).join("\n");
}

export function ModelManager({ onReportChange }: ModelManagerProps) {
  const { t } = useI18n();

  // ---- Section 1: cloud agents ----
  const [report, setReport] = useState<DoctorReport | null>(null);
  const [agentsLoading, setAgentsLoading] = useState(true);
  const [agentsError, setAgentsError] = useState<string | null>(null);
  const [busyAgent, setBusyAgent] = useState<AgentId | null>(null);
  const [agentNote, setAgentNote] = useState<Record<AgentId, string>>({
    claude: "",
    gemini: "",
    codex: "",
  });
  const [loginAgent, setLoginAgent] = useState<AgentId | null>(null);

  const refreshAgents = useCallback(
    async (probe = false) => {
      setAgentsError(null);
      try {
        const next = await agentDetect(probe);
        setReport(next);
        onReportChange(next);
      } catch {
        setAgentsError(t("common.error"));
        setReport(null);
        onReportChange(null);
      } finally {
        setAgentsLoading(false);
      }
    },
    [onReportChange, t],
  );

  useEffect(() => {
    void refreshAgents(false);
  }, [refreshAgents]);

  const installedCount = useMemo(() => {
    if (!report) {
      return 0;
    }
    // audit #15: eksik agent girdisinde (CLI kısmi çıktı) optional-chaining → render çökmesin.
    return agentDefinitions.filter(({ id }) => report.agents[id]?.installed)
      .length;
  }, [report]);

  const installAgent = async (id: AgentId) => {
    setBusyAgent(id);
    setAgentNote((current) => ({ ...current, [id]: "" }));
    try {
      await agentInstall(id);
      await refreshAgents(true);
    } catch {
      setAgentNote((current) => ({ ...current, [id]: t("common.error") }));
    } finally {
      setBusyAgent(null);
    }
  };

  const closeLogin = useCallback(() => {
    setLoginAgent(null);
    void refreshAgents(true);
  }, [refreshAgents]);

  // ---- Section 2: local embedding ----
  const [embedding, setEmbedding] = useState<EmbeddingStatus | null>(null);
  const [embeddingLoading, setEmbeddingLoading] = useState(true);
  const [embeddingError, setEmbeddingError] = useState<string | null>(null);
  const [embeddingDownloading, setEmbeddingDownloading] = useState(false);
  const [embeddingLog, setEmbeddingLog] = useState("");

  const refreshEmbedding = useCallback(async () => {
    setEmbeddingError(null);
    try {
      const next = await embeddingStatus();
      setEmbedding(next);
    } catch {
      setEmbeddingError(t("common.error"));
      setEmbedding(null);
    } finally {
      setEmbeddingLoading(false);
    }
  }, [t]);

  useEffect(() => {
    void refreshEmbedding();
  }, [refreshEmbedding]);

  const downloadEmbedding = async () => {
    setEmbeddingDownloading(true);
    setEmbeddingError(null);
    setEmbeddingLog("");
    try {
      const next = await prepareEmbeddingModel((line) => {
        setEmbeddingLog((current) => `${current}${line}\n`);
      });
      setEmbedding(next);
    } catch {
      setEmbeddingError(t("common.error"));
      await refreshEmbedding();
    } finally {
      setEmbeddingDownloading(false);
    }
  };

  // ---- Section 3: ollama ----
  const [ollama, setOllama] = useState<OllamaStatus | null>(null);
  const [ollamaLoading, setOllamaLoading] = useState(true);
  const [ollamaError, setOllamaError] = useState<string | null>(null);
  const [pullModel, setPullModel] = useState("");
  const [pulling, setPulling] = useState(false);
  const [pullLog, setPullLog] = useState("");

  const refreshOllama = useCallback(async () => {
    setOllamaError(null);
    try {
      const next = await ollamaStatus();
      setOllama(next);
    } catch {
      setOllamaError(t("common.error"));
      setOllama(null);
    } finally {
      setOllamaLoading(false);
    }
  }, [t]);

  useEffect(() => {
    void refreshOllama();
  }, [refreshOllama]);

  const pullModelRef = useRef(pullModel);
  pullModelRef.current = pullModel;

  const startPull = async () => {
    const model = pullModelRef.current.trim();
    if (!model) {
      return;
    }
    setPulling(true);
    setOllamaError(null);
    setPullLog("");
    try {
      await ollamaPull(model, undefined, (line) => {
        setPullLog((current) => `${current}${line}\n`);
      });
      setPullModel("");
      await refreshOllama();
    } catch {
      setOllamaError(t("common.error"));
    } finally {
      setPulling(false);
    }
  };

  return (
    <section className="agent-manager" aria-labelledby="models-title">
      <style>{`
        .model-section { margin-bottom: 28px; }
        .model-section-head {
          display: flex;
          align-items: center;
          justify-content: space-between;
          gap: 12px;
          flex-wrap: wrap;
          margin-bottom: 14px;
        }
        .model-section-head h2 {
          margin: 0;
          font-size: 18px;
          line-height: 1.2;
        }
        .model-log {
          max-height: 160px;
          overflow: auto;
          margin: 14px 0 0;
          padding: 12px;
          border: 1px solid var(--border);
          border-radius: var(--radius);
          background: color-mix(in srgb, var(--bg-secondary) 60%, var(--bg));
          color: var(--text-muted);
          font-family: "SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace;
          font-size: 12px;
          line-height: 1.5;
          white-space: pre-wrap;
          overflow-wrap: anywhere;
        }
      `}</style>
      <header className="manager-header">
        <div>
          <p className="eyebrow">{t("models.title")}</p>
          <h1 id="models-title">{t("models.title")}</h1>
        </div>
      </header>

      {/* Section 1: cloud agents */}
      <div className="model-section">
        <div className="model-section-head">
          <h2>{t("models.cloudAgents")}</h2>
          <div className="toolbar">
            <span className="badge accent">
              {t("agents.installed")}: {installedCount}/3
            </span>
            <button
              className="button"
              disabled={agentsLoading}
              onClick={() => void refreshAgents(true)}
              type="button"
            >
              {t("graph.refresh")}
            </button>
          </div>
        </div>

        {agentsError ? <p className="notice error">{agentsError}</p> : null}

        <div className="agent-grid">
          {agentDefinitions.map((definition) => {
            const status =
              report?.agents[definition.id] ?? fallbackStatus();
            const cta = agentCta(status);
            const badge = statusBadge(status, t);
            const isBusy = busyAgent === definition.id;
            const isPrimary = definition.id === "claude";

            return (
              <article
                className={`agent-card ${isPrimary ? "is-primary" : ""}`}
                key={definition.id}
              >
                <div className="agent-card-header">
                  <div className="agent-title">
                    <h2 className="agent-name">{definition.name}</h2>
                    <p className="agent-role">{t(definition.roleKey)}</p>
                  </div>
                  {isPrimary ? (
                    <span className="badge accent">
                      {t("agents.brainBadge")}
                    </span>
                  ) : null}
                </div>

                <div className="badge-row">
                  <span className={`badge ${badge.cls}`}>{badge.label}</span>
                  {status.version ? (
                    <span className="badge">{status.version}</span>
                  ) : null}
                </div>

                <div className="card-actions">
                  {cta === "ready" ? (
                    <button
                      className="button primary"
                      disabled
                      type="button"
                    >
                      <span aria-hidden="true">✓</span> {t("status.ready")}
                    </button>
                  ) : cta === "install" ? (
                    <button
                      className="button primary"
                      disabled={isBusy}
                      onClick={() => void installAgent(definition.id)}
                      type="button"
                    >
                      {isBusy ? t("common.loading") : t("agents.install")}
                    </button>
                  ) : cta === "login" ? (
                    <button
                      className="button primary"
                      onClick={() => setLoginAgent(definition.id)}
                      type="button"
                    >
                      {t("agents.login")}
                    </button>
                  ) : (
                    <button
                      className="button primary"
                      disabled={agentsLoading}
                      onClick={() => void refreshAgents(true)}
                      type="button"
                    >
                      {t("agents.retry")}
                    </button>
                  )}
                  <button
                    className="button ghost"
                    disabled={agentsLoading}
                    onClick={() => void refreshAgents(true)}
                    type="button"
                  >
                    {t("agents.retry")}
                  </button>
                </div>

                {agentNote[definition.id] ? (
                  <p className="inline-result error">
                    {agentNote[definition.id]}
                  </p>
                ) : null}
              </article>
            );
          })}
        </div>
      </div>

      {/* Section 2: local embedding */}
      <div className="model-section">
        <div className="model-section-head">
          <h2>{t("models.localEmbedding")}</h2>
        </div>

        <article className="agent-card" style={{ minHeight: 0 }}>
          {embeddingLoading ? (
            <p className="empty-state">{t("common.loading")}</p>
          ) : embedding ? (
            <>
              <div className="badge-row" style={{ marginTop: 0 }}>
                <span className={`badge ${embedding.ready ? "good" : ""}`}>
                  {embedding.ready
                    ? t("models.embedding.ready")
                    : t("models.embedding.notReady")}
                </span>
                <span className="badge">
                  {t("models.embedding.backend")}: {embedding.backend}
                </span>
                <span className="badge">
                  {t("models.embedding.device")}: {embedding.device}
                </span>
              </div>

              <dl className="details">
                <div className="detail-row">
                  <dt className="detail-label">{t("models.embedding.backend")}</dt>
                  <dd className="detail-value">{embedding.model}</dd>
                </div>
                {embedding.cache_path ? (
                  <div className="detail-row">
                    <dt className="detail-label">{t("agents.tokenLocation")}</dt>
                    <dd className="detail-value mono">{embedding.cache_path}</dd>
                  </div>
                ) : null}
              </dl>

              {embeddingDownloading && embeddingLog ? (
                <pre className="model-log">{tailLines(embeddingLog)}</pre>
              ) : null}

              {embeddingError ? (
                <p className="inline-result error">{embeddingError}</p>
              ) : null}

              <div className="card-actions">
                {embedding.ready && !embeddingDownloading ? (
                  <button className="button primary" disabled type="button">
                    <span aria-hidden="true">✓</span>{" "}
                    {t("models.embedding.ready")}
                  </button>
                ) : (
                  <button
                    className="button primary"
                    disabled={embeddingDownloading}
                    onClick={() => void downloadEmbedding()}
                    type="button"
                  >
                    {embeddingDownloading
                      ? t("models.embedding.downloading")
                      : t("models.embedding.download")}
                  </button>
                )}
              </div>
            </>
          ) : (
            <>
              <p className="inline-result error">
                {embeddingError ?? t("common.error")}
              </p>
              <div className="card-actions">
                <button
                  className="button"
                  onClick={() => void refreshEmbedding()}
                  type="button"
                >
                  {t("common.retry")}
                </button>
              </div>
            </>
          )}
        </article>
      </div>

      {/* Section 3: ollama */}
      <div className="model-section">
        <div className="model-section-head">
          <h2>{t("models.ollama")}</h2>
          <div className="toolbar">
            <button
              className="button"
              disabled={ollamaLoading}
              onClick={() => void refreshOllama()}
              type="button"
            >
              {t("graph.refresh")}
            </button>
          </div>
        </div>

        <article className="agent-card" style={{ minHeight: 0 }}>
          {ollamaLoading ? (
            <p className="empty-state">{t("common.loading")}</p>
          ) : ollama && !ollama.installed ? (
            <>
              <div className="badge-row" style={{ marginTop: 0 }}>
                <span className="badge bad">
                  {t("models.ollama.notInstalled")}
                </span>
              </div>
              <p className="agent-role" style={{ marginTop: 12 }}>
                {t("models.ollama.modelPlaceholder")}
              </p>
            </>
          ) : ollama ? (
            <>
              <div className="badge-row" style={{ marginTop: 0 }}>
                <span className="badge good">
                  {t("models.ollama.installed")}
                </span>
                <span className={`badge ${ollama.running ? "good" : "warn"}`}>
                  {ollama.running
                    ? t("models.ollama.running")
                    : t("models.ollama.notRunning")}
                </span>
              </div>

              <div className="badge-row">
                {ollama.models.length > 0 ? (
                  ollama.models.map((model) => (
                    <span className="badge" key={model}>
                      {model}
                    </span>
                  ))
                ) : (
                  <span className="badge">{t("models.ollama.noModels")}</span>
                )}
              </div>

              <div
                className="input-row"
                style={{ marginTop: 16 }}
              >
                <input
                  className="text-input"
                  disabled={pulling}
                  onChange={(event) => setPullModel(event.target.value)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter") {
                      void startPull();
                    }
                  }}
                  placeholder={t("models.ollama.modelPlaceholder")}
                  type="text"
                  value={pullModel}
                />
                <button
                  className="button primary"
                  disabled={pulling || pullModel.trim().length === 0}
                  onClick={() => void startPull()}
                  type="button"
                >
                  {pulling
                    ? t("models.ollama.pulling")
                    : t("models.ollama.pull")}
                </button>
              </div>

              {pulling && pullLog ? (
                <pre className="model-log">{tailLines(pullLog)}</pre>
              ) : null}

              {ollamaError ? (
                <p className="inline-result error">{ollamaError}</p>
              ) : null}
            </>
          ) : (
            <>
              <p className="inline-result error">
                {ollamaError ?? t("common.error")}
              </p>
              <div className="card-actions">
                <button
                  className="button"
                  onClick={() => void refreshOllama()}
                  type="button"
                >
                  {t("common.retry")}
                </button>
              </div>
            </>
          )}
        </article>
      </div>

      {loginAgent ? (
        <PtyLogin agent={loginAgent} onClose={closeLogin} />
      ) : null}
    </section>
  );
}
