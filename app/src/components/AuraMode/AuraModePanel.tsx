import { FormEvent, useRef, useState } from "react";

import { cancelJob, pickVaultFolder, runMode } from "../../lib/ipc";
import type { AiEvent, AiLane, AuraMode } from "../../lib/types";

type ModeOption = {
  id: AuraMode;
  label: string;
  helper: string;
};

const modeOptions: ModeOption[] = [
  { id: "plan", label: "Plan", helper: "oku-planla (yazmaz)" },
  { id: "review", label: "Review", helper: "git diff incele" },
  { id: "fix", label: "Fix", helper: "yamayı önizle (güvenli, --apply yok)" },
  { id: "ship", label: "Ship", helper: "plan -> uygula -> review" },
];

function modeLabel(mode: AuraMode) {
  const option = modeOptions.find((item) => item.id === mode);
  return option?.label ?? mode;
}

function laneLabel(lane: AiLane | null) {
  if (!lane) {
    return "Hazır";
  }

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
    default:
      return lane;
  }
}

function friendlyAiError(taxonomy?: string) {
  switch (taxonomy) {
    case "cancelled":
      return "Çalışma durduruldu.";
    case "auth":
      return "AI ajanı için giriş veya yetki gerekiyor.";
    case "rate_limit":
      return "AI ajanı şu anda limitte. Biraz sonra yeniden deneyin.";
    case "timeout":
      return "Çalışma süresi doldu. Daha dar bir istek deneyin.";
    default:
      return "Aura modu çalıştırılamadı. Ajan durumunu kontrol edip yeniden deneyin.";
  }
}

function requiresProjectDir(mode: AuraMode) {
  return mode === "review" || mode === "fix" || mode === "ship";
}

export function AuraModePanel() {
  const [mode, setMode] = useState<AuraMode>("plan");
  const [projectDir, setProjectDir] = useState<string | null>(null);
  const [prompt, setPrompt] = useState("");
  const [output, setOutput] = useState("");
  const [lane, setLane] = useState<AiLane | null>(null);
  const [streaming, setStreaming] = useState(false);
  const [jobId, setJobId] = useState<string | null>(null);
  const [runDir, setRunDir] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const activeRequest = useRef(0);

  const handleAiEvent = (requestId: number, event: AiEvent) => {
    if (requestId !== activeRequest.current) {
      return;
    }

    switch (event.kind) {
      case "start":
        setLane(event.lane);
        setStreaming(true);
        break;
      case "chunk":
        setOutput((current) => current + event.text);
        break;
      case "cached":
        setLane("cached");
        setOutput(event.text);
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

  const chooseProjectDir = async () => {
    setError(null);

    try {
      const picked = await pickVaultFolder();
      if (picked) {
        setProjectDir(picked);
      }
    } catch {
      setError("Proje klasörü seçilemedi.");
    }
  };

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    const trimmed = prompt.trim();

    if (!trimmed || streaming) {
      return;
    }

    if (requiresProjectDir(mode) && !projectDir) {
      setError(`${modeLabel(mode)} için proje klasörü gerekli.`);
      return;
    }

    const requestId = activeRequest.current + 1;
    activeRequest.current = requestId;
    setOutput("");
    setLane(null);
    setRunDir(null);
    setError(null);
    setJobId(null);
    setStreaming(true);

    try {
      const id = await runMode(mode, trimmed, projectDir, (aiEvent) =>
        handleAiEvent(requestId, aiEvent),
      );

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
    <section className="task-panel aura-mode-panel" aria-labelledby="aura-mode-title">
      <header className="panel-header">
        <div>
          <p className="eyebrow">AURA-MODE</p>
          <h1 id="aura-mode-title">Aura modu</h1>
        </div>
        <span className={`lane-badge lane-${lane ?? mode}`}>
          {modeLabel(mode)} · {laneLabel(lane)}
        </span>
      </header>

      <form className="aura-mode-form" onSubmit={submit}>
        <fieldset className="mode-selector" aria-label="Aura modu seçimi">
          <legend>Mod</legend>
          <div className="mode-segmented" role="group" aria-label="Mod seç">
            {modeOptions.map((option) => (
              <button
                aria-pressed={mode === option.id}
                className={`mode-option ${mode === option.id ? "is-active" : ""}`}
                disabled={streaming}
                key={option.id}
                onClick={() => setMode(option.id)}
                type="button"
              >
                <span>{option.label}</span>
                <small>{option.helper}</small>
              </button>
            ))}
          </div>
        </fieldset>

        <div className="project-picker">
          <button className="button" disabled={streaming} onClick={chooseProjectDir} type="button">
            Proje Klasörü
          </button>
          <span className="path-label mono">
            {projectDir ?? "Seçilmedi"}
            {requiresProjectDir(mode) ? " · gerekli" : ""}
          </span>
        </div>

        {mode === "fix" ? (
          <p className="notice aura-note">Fix yalnız ÖNİZLER; dosya değiştirmez (güvenli).</p>
        ) : null}

        <label className="field-label" htmlFor="aura-mode-prompt">
          Prompt
        </label>
        <textarea
          className="prompt-input"
          id="aura-mode-prompt"
          onChange={(event) => setPrompt(event.currentTarget.value)}
          placeholder="Bu proje için ne yapılacağını yaz"
          rows={6}
          value={prompt}
        />

        <div className="toolbar ask-actions">
          <button className="button primary" disabled={streaming || !prompt.trim()} type="submit">
            Çalıştır
          </button>
          <button
            className="button"
            disabled={!streaming || !jobId}
            onClick={stop}
            type="button"
          >
            Durdur
          </button>
          {streaming ? <span className="thinking">çalışıyor...</span> : null}
        </div>
      </form>

      {error ? <p className="notice error">{error}</p> : null}

      <article className="answer-box" aria-live="polite" aria-label="Aura modu çıktısı">
        {output ? <pre>{output}</pre> : <p className="empty-state">Çıktı burada görünür.</p>}
      </article>

      {runDir ? <p className="path-label mono">Run: {runDir}</p> : null}
    </section>
  );
}
