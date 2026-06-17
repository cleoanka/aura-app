import { useCallback, useEffect, useMemo, useState } from "react";

import { agentDetect, agentInstall } from "../lib/ipc";
import type { AgentAuth, AgentId, AgentStatus, DoctorReport } from "../lib/types";
import { PtyLogin } from "./Pty/PtyLogin";

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

function friendlyFailure(action: "detect" | "install") {
  if (action === "detect") {
    return "Ajan durumu alınamadı. Birazdan yeniden deneyin veya arka uç komutlarını kontrol edin.";
  }

  return "Kurulum başlatılamadı. Yetkileri ve ağ bağlantısını kontrol edip yeniden deneyin.";
}

function authLabel(auth: AgentAuth) {
  switch (auth) {
    case "logged_in":
      return "Giriş yapılmış";
    case "logged_out":
      return "Giriş gerekli";
    case "rate_limited":
      return "Limitte";
    case "unknown":
      return "Bilinmiyor";
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

function boolLabel(value: boolean | null) {
  if (value === true) {
    return "Evet";
  }

  if (value === false) {
    return "Hayır";
  }

  return "Bilinmiyor";
}

function tokenLocationLabel(value: AgentStatus["token_location"]) {
  switch (value) {
    case "keychain":
      return "Keychain";
    case "file":
      return "Dosya";
    case "unknown":
      return "Bilinmiyor";
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
        setError(friendlyFailure("detect"));
        setReport(null);
        onReportChange(null);
      } finally {
        setLoading(false);
        setRefreshing(false);
      }
    },
    [onReportChange],
  );

  useEffect(() => {
    refresh(false);
  }, [refresh]);

  const install = async (id: AgentId) => {
    setInstallState((current) => ({
      ...current,
      [id]: { kind: "working", message: "Kurulum başlatılıyor..." },
    }));

    try {
      const message = await agentInstall(id);
      setInstallState((current) => ({
        ...current,
        [id]: {
          kind: "success",
          message: message || "Kurulum tamamlandı. Durum yenileniyor.",
        },
      }));
      await refresh(true);
    } catch {
      setInstallState((current) => ({
        ...current,
        [id]: { kind: "error", message: friendlyFailure("install") },
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
          <p className="eyebrow">Ajan Yöneticisi</p>
          <h1 id="agent-manager-title">Aura çalışma ortamı</h1>
          <p>
            Claude ana beyin olarak yönlendirme ve mimari kararları taşır; Gemini
            araştırma, Codex ise uygulama işleri için hazır tutulur.
          </p>
        </div>

        <div className="toolbar" aria-label="Ajan yönetimi araçları">
          <span className="badge accent">{installedCount}/3 kurulu</span>
          <button
            aria-label="Ajan durumunu yenile"
            className="button primary"
            disabled={refreshing}
            onClick={() => refresh(true)}
            type="button"
          >
            {refreshing ? "Yenileniyor" : "Yenile"}
          </button>
        </div>
      </header>

      {loading ? <p className="notice">Ajanlar taranıyor...</p> : null}
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
                  <span className="badge accent">ANA BEYİN</span>
                ) : null}
              </div>

              <div className="badge-row" aria-label={`${definition.name} özet durumu`}>
                <span className={`badge ${status.installed ? "good" : ""}`}>
                  {status.installed ? "Kurulu" : "Kurulu değil"}
                </span>
                <span className={`badge ${authClass(status.auth)}`}>
                  {authLabel(status.auth)}
                </span>
              </div>

              <dl className="details">
                <div className="detail-row">
                  <dt className="detail-label">Sürüm</dt>
                  <dd className={`detail-value ${status.version ? "" : "muted"}`}>
                    {status.version ?? "Bilinmiyor"}
                  </dd>
                </div>
                <div className="detail-row">
                  <dt className="detail-label">Token</dt>
                  <dd className="detail-value">{tokenLocationLabel(status.token_location)}</dd>
                </div>
                <div className="detail-row">
                  <dt className="detail-label">Çalıştırma</dt>
                  <dd className="detail-value">{boolLabel(status.can_invoke)}</dd>
                </div>
                <div className="detail-row">
                  <dt className="detail-label">Yol</dt>
                  <dd className={`detail-value mono ${status.path ? "" : "muted"}`}>
                    {status.path ?? "Bulunamadı"}
                  </dd>
                </div>
                <div className="detail-row">
                  <dt className="detail-label">Son durum</dt>
                  <dd className={`detail-value ${status.last_error ? "" : "muted"}`}>
                    {status.last_error
                      ? "Son denetimde sorun bildirildi."
                      : "Ek hata yok."}
                  </dd>
                </div>
              </dl>

              <div className="card-actions">
                <button
                  aria-label={`${definition.name} ajanını kur`}
                  className="button primary"
                  disabled={status.installed || isInstalling}
                  onClick={() => install(definition.id)}
                  type="button"
                >
                  {isInstalling ? "Kuruluyor" : "Kur"}
                </button>
                <button
                  aria-label={`${definition.name} ajanını yeniden dene`}
                  className="button"
                  disabled={refreshing}
                  onClick={() => refresh(true)}
                  type="button"
                >
                  Yeniden Dene
                </button>
                <button
                  aria-label={`${definition.name} ajanına giriş yap`}
                  className="button ghost"
                  onClick={() => setLoginAgent(definition.id)}
                  type="button"
                >
                  Giriş
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
