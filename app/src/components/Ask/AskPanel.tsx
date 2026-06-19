import { useEffect, useState } from "react";

import { ask, askConsensus, cancelJob, getSettings, saveNote } from "../../lib/ipc";
import { useI18n } from "../../i18n";
import { ChatView } from "../Chat/ChatView";
import { useConversation, type ChatMessage } from "../../hooks/useConversation";

export function AskPanel() {
  const { t } = useI18n();
  const convo = useConversation();
  const [consensusAvailable, setConsensusAvailable] = useState(false);
  const [consensusChecked, setConsensusChecked] = useState(false);
  const [actionMsg, setActionMsg] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    // Panel hep mount; ayarları kaydedildikçe de oku (consensus toggle anında yansısın).
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

  const onSend = (text: string) => {
    setActionMsg(null);
    const useConsensus = consensusAvailable && consensusChecked;
    void convo.send(
      text,
      (prompt, onEvent) => (useConsensus ? askConsensus(prompt, onEvent) : ask(prompt, onEvent)),
      useConsensus ? { laneOverride: "consensus" } : undefined,
    );
  };

  const saveAnswer = async (content: string) => {
    setActionMsg(null);
    try {
      const path = await saveNote("ask", content);
      setActionMsg(`${t("auraMode.saved")} ${path}`);
    } catch {
      setActionMsg(t("auraMode.saveError"));
    }
  };

  const copyAnswer = async (content: string) => {
    try {
      await navigator.clipboard.writeText(content);
      setActionMsg(t("auraMode.copied"));
    } catch {
      setActionMsg(t("common.error"));
    }
  };

  const assistantActions = (message: ChatMessage) => (
    <div className="result-actions">
      <button className="button" onClick={() => void saveAnswer(message.content)} type="button">
        💾 {t("auraMode.saveNote")}
      </button>
      <button className="button" onClick={() => void copyAnswer(message.content)} type="button">
        📋 {t("auraMode.copy")}
      </button>
    </div>
  );

  return (
    <section className="task-panel chat-panel" aria-labelledby="ask-title">
      <header className="panel-header">
        <div>
          <p className="eyebrow">{t("nav.ask")}</p>
          <h1 id="ask-title">{t("settings.mode.ask")}</h1>
        </div>
      </header>

      <ChatView
        messages={convo.messages}
        streaming={convo.streaming}
        canStop={Boolean(convo.jobId)}
        onSend={onSend}
        onStop={() => void convo.stop(cancelJob)}
        onClear={convo.clear}
        placeholder={t("ask.placeholder")}
        emptyHint={t("ask.placeholder")}
        renderAssistantActions={assistantActions}
        toolbar={
          consensusAvailable || actionMsg ? (
            <div className="ask-toolbar">
              {consensusAvailable ? (
                <label className="consensus-toggle">
                  <input
                    checked={consensusChecked}
                    disabled={convo.streaming}
                    onChange={(event) => setConsensusChecked(event.currentTarget.checked)}
                    type="checkbox"
                  />
                  <span>{t("ask.consensus")}</span>
                  <small>{t("ask.consensusHint")}</small>
                </label>
              ) : null}
              {actionMsg ? <p className="notice success">{actionMsg}</p> : null}
            </div>
          ) : undefined
        }
      />
    </section>
  );
}
