import { KeyboardEvent, ReactNode, useEffect, useRef, useState } from "react";

import { useI18n } from "../../i18n";
import { MarkdownView } from "../MarkdownView";
import type { ChatMessage } from "../../hooks/useConversation";

type ChatViewProps = {
  messages: ChatMessage[];
  streaming: boolean;
  canStop: boolean;
  onSend: (text: string) => void;
  onStop: () => void;
  onClear: () => void;
  placeholder?: string;
  emptyHint?: string;
  /** Composer'ın üstündeki ekstra kontroller (mod seçici, consensus toggle vb.). */
  toolbar?: ReactNode;
  /** Akış bittiğinde asistan mesajının altına eylemler (kaydet/kopyala vb.). */
  renderAssistantActions?: (message: ChatMessage) => ReactNode;
  sendLabel?: string;
  composerDisabled?: boolean;
};

export function ChatView({
  messages,
  streaming,
  canStop,
  onSend,
  onStop,
  onClear,
  placeholder,
  emptyHint,
  toolbar,
  renderAssistantActions,
  sendLabel,
  composerDisabled,
}: ChatViewProps) {
  const { t } = useI18n();
  const [input, setInput] = useState("");
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const endRef = useRef<HTMLDivElement | null>(null);

  // Yeni içerik geldikçe en alta kaydır (kullanıcı yukarı kaydırmadıysa).
  useEffect(() => {
    const node = scrollRef.current;
    if (!node) {
      return;
    }
    const nearBottom = node.scrollHeight - node.scrollTop - node.clientHeight < 160;
    if (nearBottom) {
      endRef.current?.scrollIntoView({ behavior: "smooth", block: "end" });
    }
  }, [messages]);

  const submit = () => {
    const trimmed = input.trim();
    if (!trimmed || streaming || composerDisabled) {
      return;
    }
    onSend(trimmed);
    setInput(""); // GÖNDERİNCE OTOMATİK TEMİZLE — silip yeniden yazmak yok.
  };

  const onKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    // Enter = gönder, Shift+Enter = yeni satır (normal AI mantığı).
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      submit();
    }
  };

  return (
    <div className="chat-view">
      <div className="chat-scroll" ref={scrollRef}>
        {messages.length === 0 ? (
          <div className="chat-empty">{emptyHint ?? t("ask.placeholder")}</div>
        ) : (
          <div className="chat-thread">
            {messages.map((message) =>
              message.role === "user" ? (
                <div className="chat-msg chat-user" key={message.id}>
                  <div className="chat-bubble">{message.content}</div>
                </div>
              ) : (
                <div className="chat-msg chat-assistant" key={message.id}>
                  <div className="chat-bubble">
                    {message.status && message.streaming ? (
                      <div className="chat-status">
                        <span className="chat-spinner" aria-hidden="true" />
                        <span>{message.status}</span>
                      </div>
                    ) : null}
                    {message.content ? (
                      <MarkdownView text={message.content} />
                    ) : message.streaming && !message.status ? (
                      <div className="chat-status">
                        <span className="chat-spinner" aria-hidden="true" />
                        <span>{t("status.thinking")}</span>
                      </div>
                    ) : null}
                    {message.errorKey ? (
                      <p className="notice error chat-error">{t(message.errorKey)}</p>
                    ) : null}
                    {!message.streaming && message.content && renderAssistantActions
                      ? renderAssistantActions(message)
                      : null}
                    {message.runDir ? (
                      <p className="path-label mono chat-rundir">Run: {message.runDir}</p>
                    ) : null}
                  </div>
                </div>
              ),
            )}
            <div ref={endRef} />
          </div>
        )}
      </div>

      <div className="chat-composer">
        {toolbar ? <div className="chat-toolbar">{toolbar}</div> : null}
        <div className="chat-input-row">
          <textarea
            className="chat-input"
            value={input}
            onChange={(event) => setInput(event.currentTarget.value)}
            onKeyDown={onKeyDown}
            placeholder={placeholder ?? t("ask.placeholder")}
            rows={1}
            disabled={composerDisabled}
          />
          {streaming ? (
            <button
              className="button chat-stop"
              disabled={!canStop}
              onClick={onStop}
              type="button"
            >
              {t("ask.stop")}
            </button>
          ) : (
            <button
              className="button primary chat-send"
              disabled={!input.trim() || composerDisabled}
              onClick={submit}
              type="button"
            >
              {sendLabel ?? t("ask.button")}
            </button>
          )}
          {messages.length > 0 ? (
            <button
              className="button chat-clear"
              disabled={streaming}
              onClick={onClear}
              type="button"
              title={t("chat.new")}
            >
              {t("chat.new")}
            </button>
          ) : null}
        </div>
      </div>
    </div>
  );
}
