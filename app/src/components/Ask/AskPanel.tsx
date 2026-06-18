import { FormEvent, useEffect, useRef, useState } from "react";

import { ask, askConsensus, cancelJob, getSettings } from "../../lib/ipc";
import type { AiEvent, AiLane } from "../../lib/types";
import { useI18n } from "../../i18n";
import { LiveActivity } from "../LiveActivity";
import { MarkdownView } from "../MarkdownView";

type TFn = (key: string, vars?: Record<string, string | number>) => string;

function laneLabel(lane: AiLane | null, t: TFn) {
  switch (lane) {
    case "cached":
      return t("ask.lane.cached");
    case "fast":
      return t("ask.lane.fast");
    case "deep":
      return t("ask.lane.deep");
    case "consensus":
      return t("ask.lane.consensus");
    case "lane0":
      return t("ask.lane.lane0");
    case null:
      return t("status.ready");
    default:
      return lane;
  }
}

function friendlyAiError(t: TFn, taxonomy?: string) {
  switch (taxonomy) {
    case "cancelled":
      return t("ask.stop");
    case "auth":
      return t("agents.auth.loggedOut");
    case "rate_limit":
      return t("agents.auth.rateLimited");
    case "timeout":
      return t("ask.error");
    default:
      return t("ask.error");
  }
}

export function AskPanel() {
  const { t } = useI18n();
  const [query, setQuery] = useState("");
  const [answer, setAnswer] = useState("");
  const [lane, setLane] = useState<AiLane | null>(null);
  const [streaming, setStreaming] = useState(false);
  const [statusText, setStatusText] = useState<string | null>(null);
  const [statusLog, setStatusLog] = useState<string[]>([]);
  const [jobId, setJobId] = useState<string | null>(null);
  const [runDir, setRunDir] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [consensusAvailable, setConsensusAvailable] = useState(false);
  const [consensusChecked, setConsensusChecked] = useState(false);
  const activeRequest = useRef(0);
  const activeConsensus = useRef(false);

  useEffect(() => {
    let alive = true;

    // Panel HEP MOUNT olduğu için ayarları bir kez değil, kaydedildikçe de oku
    // (Settings'te consensus açılınca burada anında yansısın).
    const loadSettings = () => {
      void getSettings()
        .then((settings) => {
          if (alive) {
            setConsensusAvailable(settings.consensus_enabled === true);
          }
        })
        .catch(() => {
          if (alive) {
            setConsensusAvailable(false);
          }
        });
    };

    loadSettings();
    window.addEventListener("aura:settings-saved", loadSettings);

    return () => {
      alive = false;
      window.removeEventListener("aura:settings-saved", loadSettings);
    };
  }, []);

  const handleAiEvent = (requestId: number, event: AiEvent) => {
    if (requestId !== activeRequest.current) {
      return;
    }

    switch (event.kind) {
      case "start":
        setLane(activeConsensus.current ? "consensus" : event.lane);
        setStreaming(true);
        break;
      case "job":
        // job_id baştan gelir → Stop butonu akış sırasında çalışır
        setJobId(event.job_id);
        break;
      case "chunk":
        setAnswer((current) => current + event.text);
        break;
      case "status":
        setStatusText(event.text);
        setStatusLog((log) => [...log.slice(-40), event.text]);
        break;
      case "cached":
        setLane(activeConsensus.current ? "consensus" : "cached");
        setStatusText(t("ask.lane.cached"));
        setAnswer(event.text);
        setStreaming(false); // cache hit terminaldir → spinner'ı durdur
        break;
      case "done":
        setStreaming(false);
        setRunDir(event.run_dir ?? null);
        break;
      case "error":
        setStreaming(false);
        setError(friendlyAiError(t, event.taxonomy));
        break;
    }
  };

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const trimmed = query.trim();

    if (!trimmed || streaming) {
      return;
    }

    const requestId = activeRequest.current + 1;
    activeRequest.current = requestId;
    setAnswer("");
    setLane(null);
    setRunDir(null);
    setError(null);
    setJobId(null);
    setStatusText(null);
    setStatusLog([]);
    setStreaming(true);
    activeConsensus.current = consensusAvailable && consensusChecked;

    try {
      const run = activeConsensus.current ? askConsensus : ask;
      // job_id artık "job" event'inden gelir (akış sırasında); dönüş = cevap metni.
      await run(trimmed, (aiEvent) => handleAiEvent(requestId, aiEvent));
    } catch {
      if (requestId === activeRequest.current) {
        setStreaming(false);
        setError(friendlyAiError(t));
      }
    }
  };

  const stop = async () => {
    if (!jobId) {
      return;
    }

    // requestId'yi ilerlet → iptal sonrası geç gelen event'ler yok sayılır (race-safe).
    activeRequest.current += 1;
    setStreaming(false);

    try {
      await cancelJob(jobId);
      setError(friendlyAiError(t, "cancelled"));
    } catch {
      setError(t("ask.error"));
    }
  };

  return (
    <section className="task-panel ask-panel" aria-labelledby="ask-title">
      <header className="panel-header">
        <div>
          <p className="eyebrow">{t("nav.ask")}</p>
          <h1 id="ask-title">{t("settings.mode.ask")}</h1>
        </div>
        <span className={`lane-badge lane-${lane ?? "idle"}`}>{laneLabel(lane, t)}</span>
      </header>

      <form className="ask-form" onSubmit={submit}>
        <label className="field-label" htmlFor="ask-query">
          {t("nav.ask")}
        </label>
        <textarea
          className="prompt-input"
          id="ask-query"
          onChange={(event) => setQuery(event.currentTarget.value)}
          placeholder={t("ask.placeholder")}
          rows={4}
          value={query}
        />
        <div className="toolbar ask-actions">
          <button className="button primary" disabled={streaming} type="submit">
            {t("ask.button")}
          </button>
          {consensusAvailable ? (
            <label className="consensus-toggle">
              <input
                checked={consensusChecked}
                disabled={streaming}
                onChange={(event) => setConsensusChecked(event.currentTarget.checked)}
                type="checkbox"
              />
              <span>{t("ask.consensus")}</span>
              <small>{t("ask.consensusHint")}</small>
            </label>
          ) : null}
          <button
            className="button"
            disabled={!streaming || !jobId}
            onClick={stop}
            type="button"
          >
            {t("ask.stop")}
          </button>
        </div>
      </form>

      <LiveActivity streaming={streaming} status={statusText} log={statusLog} />

      {error ? <p className="notice error">{error}</p> : null}

      <article className="answer-box" aria-live="polite" aria-label={t("settings.mode.ask")}>
        {answer ? (
          <MarkdownView text={answer} />
        ) : (
          <p className="empty-state">{t("ask.placeholder")}</p>
        )}
      </article>

      {runDir ? <p className="path-label mono">Run: {runDir}</p> : null}
    </section>
  );
}
