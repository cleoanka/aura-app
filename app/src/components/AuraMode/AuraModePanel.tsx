import { useState } from "react";

import { useI18n } from "../../i18n";
import { askConsensus, cancelJob, chat, pickVaultFolder, runMode, saveNote } from "../../lib/ipc";
import type { AuraMode } from "../../lib/types";
import { ChatView } from "../Chat/ChatView";
import { useConversation, type AiRunner, type ChatMessage } from "../../hooks/useConversation";

type ModeOption = {
  id: AuraMode;
  labelKey: string;
  helperKey: string;
};

const modeOptions: ModeOption[] = [
  { id: "chat", labelKey: "auraMode.chat", helperKey: "auraMode.chatHint" },
  { id: "consensus", labelKey: "auraMode.consensus", helperKey: "auraMode.consensusHint" },
  { id: "plan", labelKey: "auraMode.plan", helperKey: "auraMode.planHint" },
  { id: "review", labelKey: "auraMode.review", helperKey: "auraMode.reviewHint" },
  { id: "fix", labelKey: "auraMode.fix", helperKey: "auraMode.fixHint" },
  { id: "ship", labelKey: "auraMode.ship", helperKey: "auraMode.shipHint" },
];

function modeLabelKey(mode: AuraMode) {
  return modeOptions.find((item) => item.id === mode)?.labelKey ?? mode;
}

function requiresProjectDir(mode: AuraMode) {
  return mode === "review" || mode === "fix" || mode === "ship";
}

export function AuraModePanel() {
  const { t } = useI18n();
  const convo = useConversation();
  const [mode, setMode] = useState<AuraMode>("chat");
  const [projectDir, setProjectDir] = useState<string | null>(null);
  const [actionMsg, setActionMsg] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

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

  const runnerFor = (targetMode: AuraMode): AiRunner => {
    if (targetMode === "chat") {
      return (prompt, onEvent) => chat(prompt, onEvent);
    }
    if (targetMode === "consensus") {
      return (prompt, onEvent) => askConsensus(prompt, onEvent);
    }
    return (prompt, onEvent) => runMode(targetMode, prompt, projectDir, onEvent);
  };

  const sendFor = (targetMode: AuraMode, text: string) => {
    setError(null);
    setActionMsg(null);
    if (requiresProjectDir(targetMode) && !projectDir) {
      setError(`${t(modeLabelKey(targetMode))} · ${t("auraMode.projectFolder")}`);
      return;
    }
    void convo.send(
      text,
      runnerFor(targetMode),
      targetMode === "consensus" ? { laneOverride: "consensus" } : undefined,
    );
  };

  const saveResult = async (content: string) => {
    setActionMsg(null);
    try {
      const path = await saveNote(mode, content);
      setActionMsg(`${t("auraMode.saved")} ${path}`);
    } catch {
      setActionMsg(t("auraMode.saveError"));
    }
  };

  const copyResult = async (content: string) => {
    try {
      await navigator.clipboard.writeText(content);
      setActionMsg(t("auraMode.copied"));
    } catch {
      setActionMsg(t("common.error"));
    }
  };

  const applyAsFix = (content: string) => {
    setMode("fix");
    sendFor("fix", `Şu planı/öneriyi uygula:\n\n${content}`);
  };

  const projectMissing = requiresProjectDir(mode) && !projectDir;

  const assistantActions = (message: ChatMessage) => (
    <div className="result-actions">
      <button className="button" onClick={() => void saveResult(message.content)} type="button">
        💾 {t("auraMode.saveNote")}
      </button>
      <button className="button" onClick={() => void copyResult(message.content)} type="button">
        📋 {t("auraMode.copy")}
      </button>
      <button className="button primary" onClick={() => applyAsFix(message.content)} type="button">
        🛠️ {t("auraMode.sendToFix")}
      </button>
    </div>
  );

  return (
    <section className="task-panel chat-panel aura-mode-panel" aria-labelledby="aura-mode-title">
      <header className="panel-header">
        <div>
          <p className="eyebrow">AURA-MODE</p>
          <h1 id="aura-mode-title">{t("auraMode.title")}</h1>
        </div>
        <span className={`lane-badge lane-${mode}`}>{t(modeLabelKey(mode))}</span>
      </header>

      <ChatView
        messages={convo.messages}
        streaming={convo.streaming}
        canStop={Boolean(convo.jobId)}
        onSend={(text) => sendFor(mode, text)}
        onStop={() => void convo.stop(cancelJob)}
        onClear={convo.clear}
        placeholder={t("ask.placeholder")}
        emptyHint={t("auraMode.title")}
        sendLabel={t("auraMode.run")}
        composerDisabled={projectMissing}
        renderAssistantActions={assistantActions}
        toolbar={
          <div className="aura-toolbar">
            <div className="mode-segmented" role="group" aria-label={t("auraMode.title")}>
              {modeOptions.map((option) => (
                <button
                  aria-pressed={mode === option.id}
                  className={`mode-option ${mode === option.id ? "is-active" : ""}`}
                  disabled={convo.streaming}
                  key={option.id}
                  onClick={() => setMode(option.id)}
                  title={t(option.helperKey)}
                  type="button"
                >
                  {t(option.labelKey)}
                </button>
              ))}
            </div>
            {requiresProjectDir(mode) ? (
              <div className="project-picker">
                <button
                  className="button"
                  disabled={convo.streaming}
                  onClick={chooseProjectDir}
                  type="button"
                >
                  {t("auraMode.projectFolder")}
                </button>
                <span className="path-label mono">{projectDir ?? t("workspace.selectNote")}</span>
              </div>
            ) : null}
            {mode === "fix" ? <p className="notice aura-note">{t("auraMode.fixSafeNote")}</p> : null}
            {error ? <p className="notice error">{error}</p> : null}
            {actionMsg ? <p className="notice success">{actionMsg}</p> : null}
          </div>
        }
      />
    </section>
  );
}
