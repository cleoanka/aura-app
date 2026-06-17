import { useEffect, useRef, useState } from "react";

type Props = {
  /** AI şu an çalışıyor mu */
  streaming: boolean;
  /** en son durum metni (ör. "🧠 Claude düşünüyor…") */
  status: string | null;
  /** verbose aktivite geçmişi (son satırlar gösterilir) */
  log: string[];
};

// Çalışırken kullanıcı HER ZAMAN hareket görsün: dönen spinner + güncel durum +
// HER SANİYE artan geçen-süre sayacı (TTFT boşluğunda "donmuş mu?" hissini bitirir).
export function LiveActivity({ streaming, status, log }: Props) {
  const [elapsed, setElapsed] = useState(0);
  const logRef = useRef<HTMLUListElement | null>(null);

  useEffect(() => {
    if (!streaming) {
      return;
    }
    setElapsed(0);
    const id = window.setInterval(() => setElapsed((e) => e + 1), 1000);
    return () => window.clearInterval(id);
  }, [streaming]);

  useEffect(() => {
    if (logRef.current) {
      logRef.current.scrollTop = logRef.current.scrollHeight;
    }
  }, [log.length]);

  if (!streaming && log.length === 0) {
    return null;
  }

  return (
    <div className="live-activity" role="status" aria-live="polite">
      <div className="live-activity-head">
        {streaming ? (
          <span className="live-spinner" aria-hidden="true" />
        ) : (
          <span className="live-check" aria-hidden="true">
            ✓
          </span>
        )}
        <span className="live-status">{status ?? "Başlatılıyor…"}</span>
        {streaming ? <span className="live-elapsed">{elapsed}s</span> : null}
      </div>
      {log.length > 0 ? (
        <ul className="live-log" ref={logRef}>
          {log.slice(-8).map((line, index) => (
            <li key={`${index}-${line}`}>{line}</li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}
