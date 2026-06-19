import { useEffect, useState } from "react";

import { ask, askConsensus, cancelJob, getSettings } from "../../lib/ipc";
import { useI18n } from "../../i18n";
import { ChatView } from "../Chat/ChatView";
import { useConversation } from "../../hooks/useConversation";

export function AskPanel() {
  const { t } = useI18n();
  const convo = useConversation();
  const [consensusAvailable, setConsensusAvailable] = useState(false);
  const [consensusChecked, setConsensusChecked] = useState(false);

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
    const useConsensus = consensusAvailable && consensusChecked;
    void convo.send(
      text,
      (prompt, onEvent) => (useConsensus ? askConsensus(prompt, onEvent) : ask(prompt, onEvent)),
      useConsensus ? { laneOverride: "consensus" } : undefined,
    );
  };

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
        toolbar={
          consensusAvailable ? (
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
          ) : undefined
        }
      />
    </section>
  );
}
