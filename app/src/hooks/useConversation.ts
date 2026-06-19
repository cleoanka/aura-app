import { useCallback, useRef, useState } from "react";

import type { AiEvent, AiLane } from "../lib/types";

export type ChatRole = "user" | "assistant";

export type ChatMessage = {
  id: number;
  role: ChatRole;
  content: string;
  lane: AiLane | null;
  status: string | null;
  statusLog: string[];
  streaming: boolean;
  errorKey: string | null;
  runDir: string | null;
};

/** Bir AI çağrısını (ask/chat/consensus/runMode) sarmalayan çalıştırıcı. */
export type AiRunner = (
  prompt: string,
  onEvent: (event: AiEvent) => void,
) => Promise<unknown>;

function errorKeyFor(taxonomy?: string): string {
  switch (taxonomy) {
    case "cancelled":
      return "ask.stop";
    case "auth":
      return "agents.auth.loggedOut";
    case "rate_limit":
      return "agents.auth.rateLimited";
    default:
      return "ask.error";
  }
}

/**
 * Gerçek konuşma (chat) durumu: mesajlar geçmişe BİRİKİR, gönderince input çağıran tarafça
 * temizlenir, akış mevcut asistan mesajına yazılır. Tüm AI panellerinde ortak kullanılır.
 */
export function useConversation() {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [streaming, setStreaming] = useState(false);
  const [jobId, setJobId] = useState<string | null>(null);
  const activeRequest = useRef(0);
  const idSeq = useRef(0);

  const patch = (
    id: number,
    update: Partial<ChatMessage> | ((m: ChatMessage) => Partial<ChatMessage>),
  ) => {
    setMessages((msgs) =>
      msgs.map((m) =>
        m.id === id
          ? { ...m, ...(typeof update === "function" ? update(m) : update) }
          : m,
      ),
    );
  };

  /** Kullanıcı mesajını ekler, akan asistan mesajını oluşturur, runner'ı çalıştırır. */
  const send = useCallback(
    async (
      prompt: string,
      runner: AiRunner,
      opts?: { laneOverride?: AiLane },
    ): Promise<boolean> => {
      const trimmed = prompt.trim();
      if (!trimmed || streaming) {
        return false;
      }

      const requestId = activeRequest.current + 1;
      activeRequest.current = requestId;
      const userId = (idSeq.current += 1);
      const assistantId = (idSeq.current += 1);

      setMessages((msgs) => [
        ...msgs,
        {
          id: userId,
          role: "user",
          content: trimmed,
          lane: null,
          status: null,
          statusLog: [],
          streaming: false,
          errorKey: null,
          runDir: null,
        },
        {
          id: assistantId,
          role: "assistant",
          content: "",
          lane: opts?.laneOverride ?? null,
          status: null,
          statusLog: [],
          streaming: true,
          errorKey: null,
          runDir: null,
        },
      ]);
      setStreaming(true);
      setJobId(null);

      const onEvent = (event: AiEvent) => {
        if (requestId !== activeRequest.current) {
          return;
        }
        switch (event.kind) {
          case "start":
            patch(assistantId, { lane: opts?.laneOverride ?? event.lane, streaming: true });
            break;
          case "job":
            setJobId(event.job_id);
            break;
          case "chunk":
            patch(assistantId, (m) => ({ content: m.content + event.text }));
            break;
          case "status":
            patch(assistantId, (m) => ({
              status: event.text,
              statusLog: [...m.statusLog.slice(-40), event.text],
            }));
            break;
          case "cached":
            patch(assistantId, {
              lane: opts?.laneOverride ?? "cached",
              content: event.text,
              status: null,
              streaming: false,
            });
            setStreaming(false);
            break;
          case "done":
            patch(assistantId, { streaming: false, status: null, runDir: event.run_dir ?? null });
            setStreaming(false);
            break;
          case "error":
            patch(assistantId, {
              streaming: false,
              status: null,
              errorKey: errorKeyFor(event.taxonomy),
            });
            setStreaming(false);
            break;
        }
      };

      try {
        await runner(trimmed, onEvent);
      } catch {
        if (requestId === activeRequest.current) {
          patch(assistantId, { streaming: false, status: null, errorKey: "ask.error" });
          setStreaming(false);
        }
      }
      return true;
    },
    [streaming],
  );

  const stop = useCallback(
    async (cancel: (jobId: string) => Promise<unknown>) => {
      if (!jobId) {
        return;
      }
      // requestId'yi ilerlet → iptal sonrası geç gelen event'ler yok sayılır.
      activeRequest.current += 1;
      setStreaming(false);
      setMessages((msgs) =>
        msgs.map((m) =>
          m.streaming ? { ...m, streaming: false, status: null, errorKey: "ask.stop" } : m,
        ),
      );
      try {
        await cancel(jobId);
      } catch {
        /* iptal hatası yutulur */
      }
    },
    [jobId],
  );

  const clear = useCallback(() => {
    activeRequest.current += 1;
    setMessages([]);
    setStreaming(false);
    setJobId(null);
  }, []);

  return { messages, streaming, jobId, send, stop, clear };
}
