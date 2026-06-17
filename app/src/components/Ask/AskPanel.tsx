import { FormEvent, useEffect, useRef, useState } from "react";

import { ask, askConsensus, cancelJob, getSettings } from "../../lib/ipc";
import type { AiEvent, AiLane } from "../../lib/types";

function laneLabel(lane: AiLane | null) {
  switch (lane) {
    case "cached":
      return "Önbellek";
    case "fast":
      return "Hızlı";
    case "deep":
      return "Derin";
    case "consensus":
      return "Konsensüs";
    case "lane0":
      return "Lane 0";
    case null:
      return "Hazır";
    default:
      return lane;
  }
}

function friendlyAiError(taxonomy?: string) {
  switch (taxonomy) {
    case "cancelled":
      return "Yanıt durduruldu.";
    case "auth":
      return "AI ajanı için giriş veya yetki gerekiyor.";
    case "rate_limit":
      return "AI ajanı şu anda limitte. Biraz sonra yeniden deneyin.";
    case "timeout":
      return "Yanıt süresi doldu. Daha kısa bir soru deneyin.";
    default:
      return "Yanıt alınamadı. Ajan durumunu kontrol edip yeniden deneyin.";
  }
}

export function AskPanel() {
  const [query, setQuery] = useState("");
  const [answer, setAnswer] = useState("");
  const [lane, setLane] = useState<AiLane | null>(null);
  const [streaming, setStreaming] = useState(false);
  const [jobId, setJobId] = useState<string | null>(null);
  const [runDir, setRunDir] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [consensusAvailable, setConsensusAvailable] = useState(false);
  const [consensusChecked, setConsensusChecked] = useState(false);
  const activeRequest = useRef(0);
  const activeConsensus = useRef(false);

  useEffect(() => {
    let alive = true;

    void getSettings()
      .then((settings) => {
        if (!alive) {
          return;
        }

        const enabled = settings.consensus_enabled === true;
        setConsensusAvailable(enabled);
        setConsensusChecked(false);
      })
      .catch(() => {
        if (alive) {
          setConsensusAvailable(false);
          setConsensusChecked(false);
        }
      });

    return () => {
      alive = false;
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
      case "chunk":
        setAnswer((current) => current + event.text);
        break;
      case "cached":
        setLane(activeConsensus.current ? "consensus" : "cached");
        setAnswer(event.text);
        break;
      case "done":
        setStreaming(false);
        setRunDir(event.run_dir ?? null);
        break;
      case "error":
        setStreaming(false);
        setError(friendlyAiError(event.taxonomy));
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
    setStreaming(true);
    activeConsensus.current = consensusAvailable && consensusChecked;

    try {
      const run = activeConsensus.current ? askConsensus : ask;
      const id = await run(trimmed, (aiEvent) => handleAiEvent(requestId, aiEvent));

      if (requestId === activeRequest.current && id.trim()) {
        setJobId(id);
      }
    } catch {
      if (requestId === activeRequest.current) {
        setStreaming(false);
        setError(friendlyAiError());
      }
    }
  };

  const stop = async () => {
    if (!jobId) {
      return;
    }

    try {
      await cancelJob(jobId);
      setStreaming(false);
      setError(friendlyAiError("cancelled"));
    } catch {
      setError("Durdurma isteği gönderilemedi.");
    }
  };

  return (
    <section className="task-panel ask-panel" aria-labelledby="ask-title">
      <header className="panel-header">
        <div>
          <p className="eyebrow">ASK</p>
          <h1 id="ask-title">AI Q&amp;A</h1>
        </div>
        <span className={`lane-badge lane-${lane ?? "idle"}`}>{laneLabel(lane)}</span>
      </header>

      <form className="ask-form" onSubmit={submit}>
        <label className="field-label" htmlFor="ask-query">
          Soru
        </label>
        <textarea
          className="prompt-input"
          id="ask-query"
          onChange={(event) => setQuery(event.currentTarget.value)}
          placeholder="Vault hakkında sor"
          rows={4}
          value={query}
        />
        <div className="toolbar ask-actions">
          <button className="button primary" disabled={streaming} type="submit">
            Sor
          </button>
          {consensusAvailable ? (
            <label className="consensus-toggle">
              <input
                checked={consensusChecked}
                disabled={streaming}
                onChange={(event) => setConsensusChecked(event.currentTarget.checked)}
                type="checkbox"
              />
              <span>Consensus</span>
              <small>3 ajan · ~3× maliyet</small>
            </label>
          ) : null}
          <button
            className="button"
            disabled={!streaming || !jobId}
            onClick={stop}
            type="button"
          >
            Durdur
          </button>
          {streaming ? <span className="thinking">düşünüyor...</span> : null}
        </div>
      </form>

      {error ? <p className="notice error">{error}</p> : null}

      <article className="answer-box" aria-live="polite" aria-label="AI yanıtı">
        {answer ? <pre>{answer}</pre> : <p className="empty-state">Yanıt burada görünür.</p>}
      </article>

      {runDir ? <p className="path-label mono">Run: {runDir}</p> : null}
    </section>
  );
}
