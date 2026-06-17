import { FormEvent, useRef, useState } from "react";

import { useI18n } from "../../i18n";
import { cancelJob, chat, pickVaultFolder, runMode } from "../../lib/ipc";
import type { AiEvent, AiLane, AuraMode } from "../../lib/types";
import { LiveActivity } from "../LiveActivity";

type ModeOption = {
  id: AuraMode;
  labelKey: string;
  helperKey: string;
};

const modeOptions: ModeOption[] = [
  { id: "chat", labelKey: "auraMode.chat", helperKey: "auraMode.chatHint" },
  { id: "plan", labelKey: "auraMode.plan", helperKey: "auraMode.planHint" },
  { id: "review", labelKey: "auraMode.review", helperKey: "auraMode.reviewHint" },
  { id: "fix", labelKey: "auraMode.fix", helperKey: "auraMode.fixHint" },
  { id: "ship", labelKey: "auraMode.ship", helperKey: "auraMode.shipHint" },
];

function modeLabelKey(mode: AuraMode) {
  const option = modeOptions.find((item) => item.id === mode);
  return option?.labelKey ?? mode;
}

function laneLabelKey(lane: AiLane | null) {
  if (!lane) {
    return "status.ready";
  }

  switch (lane) {
    case "cached":
      return "ask.lane.cached";
    case "fast":
      return "ask.lane.fast";
    case "deep":
      return "ask.lane.deep";
    case "consensus":
      return "ask.lane.consensus";
    case "lane0":
      return "ask.lane.lane0";
    default:
      return lane;
  }
}

function friendlyAiErrorKey(taxonomy?: string) {
  switch (taxonomy) {
    case "cancelled":
      return "ask.stop";
    case "auth":
      return "agents.auth.loggedOut";
    case "rate_limit":
      return "agents.auth.rateLimited";
    case "timeout":
      return "common.error";
    default:
      return "ask.error";
  }
}

function requiresProjectDir(mode: AuraMode) {
  return mode === "review" || mode === "fix" || mode === "ship";
}

export function AuraModePanel() {
  const { t } = useI18n();
  const [mode, setMode] = useState<AuraMode>("chat");
  const [projectDir, setProjectDir] = useState<string | null>(null);
  const [prompt, setPrompt] = useState("");
  const [output, setOutput] = useState("");
  const [lane, setLane] = useState<AiLane | null>(null);
  const [streaming, setStreaming] = useState(false);
  const [statusText, setStatusText] = useState<string | null>(null);
  const [statusLog, setStatusLog] = useState<string[]>([]);
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
      case "job":
        setJobId(event.job_id);
        break;
      case "chunk":
        setOutput((current) => current + event.text);
        break;
      case "status":
        setStatusText(event.text);
        setStatusLog((log) => [...log.slice(-40), event.text]);
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
        setError(t(friendlyAiErrorKey(event.taxonomy)));
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
      setError(t("common.error"));
    }
  };

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    const trimmed = prompt.trim();

    if (!trimmed || streaming) {
      return;
    }

    if (requiresProjectDir(mode) && !projectDir) {
      setError(`${t(modeLabelKey(mode))} · ${t("auraMode.projectFolder")}`);
      return;
    }

    const requestId = activeRequest.current + 1;
    activeRequest.current = requestId;
    setOutput("");
    setLane(null);
    setRunDir(null);
    setError(null);
    setJobId(null);
    setStatusText(null);
    setStatusLog([]);
    setStreaming(true);

    try {
      const onEvt = (aiEvent: AiEvent) => handleAiEvent(requestId, aiEvent);
      // job_id artık "job" event'inden gelir (akış sırasında); dönüş = çıktı metni.
      if (mode === "chat") {
        await chat(trimmed, onEvt);
      } else {
        await runMode(mode, trimmed, projectDir, onEvt);
      }
    } catch {
      if (requestId === activeRequest.current) {
        setStreaming(false);
        setError(t(friendlyAiErrorKey()));
      }
    }
  };

  const stop = async () => {
    if (!jobId) {
      return;
    }

    activeRequest.current += 1;
    setStreaming(false);

    try {
      await cancelJob(jobId);
      setError(t(friendlyAiErrorKey("cancelled")));
    } catch {
      setError(t("common.error"));
    }
  };

  return (
    <section className="task-panel aura-mode-panel" aria-labelledby="aura-mode-title">
      <header className="panel-header">
        <div>
          <p className="eyebrow">AURA-MODE</p>
          <h1 id="aura-mode-title">{t("auraMode.title")}</h1>
        </div>
        <span className={`lane-badge lane-${lane ?? mode}`}>
          {t(modeLabelKey(mode))} · {t(laneLabelKey(lane))}
        </span>
      </header>

      <form className="aura-mode-form" onSubmit={submit}>
        <fieldset className="mode-selector" aria-label={t("auraMode.title")}>
          <legend>{t("settings.defaultMode")}</legend>
          <div className="mode-segmented" role="group" aria-label={t("auraMode.title")}>
            {modeOptions.map((option) => (
              <button
                aria-pressed={mode === option.id}
                className={`mode-option ${mode === option.id ? "is-active" : ""}`}
                disabled={streaming}
                key={option.id}
                onClick={() => setMode(option.id)}
                type="button"
              >
                <span>{t(option.labelKey)}</span>
                <small>{t(option.helperKey)}</small>
              </button>
            ))}
          </div>
        </fieldset>

        {requiresProjectDir(mode) ? (
          <div className="project-picker">
            <button className="button" disabled={streaming} onClick={chooseProjectDir} type="button">
              {t("auraMode.projectFolder")}
            </button>
            <span className="path-label mono">{projectDir ?? t("workspace.selectNote")}</span>
          </div>
        ) : null}

        {mode === "fix" ? (
          <p className="notice aura-note">{t("auraMode.fixSafeNote")}</p>
        ) : null}

        <label className="field-label" htmlFor="aura-mode-prompt">
          {t("ask.button")}
        </label>
        <textarea
          className="prompt-input"
          id="aura-mode-prompt"
          onChange={(event) => setPrompt(event.currentTarget.value)}
          placeholder={t("ask.placeholder")}
          rows={6}
          value={prompt}
        />

        <div className="toolbar ask-actions">
          <button className="button primary" disabled={streaming || !prompt.trim()} type="submit">
            {t("auraMode.run")}
          </button>
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

      <article className="answer-box" aria-live="polite" aria-label={t("auraMode.title")}>
        {output ? <pre>{output}</pre> : <p className="empty-state">{t("graph.empty")}</p>}
      </article>

      {runDir ? <p className="path-label mono">Run: {runDir}</p> : null}
    </section>
  );
}
