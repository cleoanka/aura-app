import { useEffect, useRef, useState } from "react";
import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";

import { ptyClose, ptyOpen, ptyResize, ptyWrite } from "../../lib/ipc";
import type { AgentId } from "../../lib/types";
import { useI18n } from "../../i18n";

type PtyLoginProps = {
  agent: AgentId;
  onClose: () => void;
};

function agentLabel(agent: AgentId) {
  switch (agent) {
    case "claude":
      return "Claude";
    case "gemini":
      return "Gemini";
    case "codex":
      return "Codex";
  }
}

export function PtyLogin({ agent, onClose }: PtyLoginProps) {
  const { t } = useI18n();
  const containerRef = useRef<HTMLDivElement | null>(null);
  const terminalRef = useRef<Terminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const sessionIdRef = useRef<string | null>(null);
  const closeStartedRef = useRef(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let disposed = false;
    let resizeObserver: ResizeObserver | null = null;

    const closeSession = async () => {
      const sessionId = sessionIdRef.current;

      if (!sessionId || closeStartedRef.current) {
        return;
      }

      closeStartedRef.current = true;
      sessionIdRef.current = null;

      try {
        await ptyClose(sessionId);
      } catch {
        // The session may already be closed by the backend; closing the modal should continue.
      }
    };

    const fitAndResize = () => {
      const fitAddon = fitAddonRef.current;
      const terminal = terminalRef.current;
      const sessionId = sessionIdRef.current;

      if (!fitAddon || !terminal) {
        return;
      }

      fitAddon.fit();

      if (sessionId) {
        void ptyResize(sessionId, terminal.rows, terminal.cols);
      }
    };

    const openTerminal = async () => {
      if (!containerRef.current) {
        return;
      }

      const terminal = new Terminal({
        cursorBlink: true,
        convertEol: true,
        fontFamily: '"SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace',
        fontSize: 13,
        lineHeight: 1.25,
        theme: {
          background: "#17171c",
          cursor: "#ececf1",
          foreground: "#ececf1",
          selectionBackground: "#343440",
        },
      });
      const fitAddon = new FitAddon();

      terminal.loadAddon(fitAddon);
      terminal.open(containerRef.current);
      terminalRef.current = terminal;
      fitAddonRef.current = fitAddon;

      terminal.writeln(`${agentLabel(agent)} — ${t("common.loading")}`);
      fitAddon.fit();

      try {
        const sessionId = await ptyOpen(agent, (chunk) => {
          if (!disposed) {
            terminal.write(chunk);
          }
        });

        if (disposed) {
          await ptyClose(sessionId);
          return;
        }

        sessionIdRef.current = sessionId;
        terminal.onData((data) => {
          if (sessionIdRef.current) {
            void ptyWrite(sessionIdRef.current, data);
          }
        });
        fitAndResize();

        resizeObserver = new ResizeObserver(fitAndResize);
        resizeObserver.observe(containerRef.current);
        window.addEventListener("resize", fitAndResize);
      } catch {
        if (!disposed) {
          setError(t("common.error"));
          terminal.dispose();
          terminalRef.current = null;
          fitAddonRef.current = null;
        }
      }
    };

    void openTerminal();

    return () => {
      disposed = true;
      resizeObserver?.disconnect();
      window.removeEventListener("resize", fitAndResize);
      void closeSession();
      terminalRef.current?.dispose();
      terminalRef.current = null;
      fitAddonRef.current = null;
    };
  }, [agent, t]);

  const close = async () => {
    const sessionId = sessionIdRef.current;

    if (sessionId && !closeStartedRef.current) {
      closeStartedRef.current = true;
      sessionIdRef.current = null;

      try {
        await ptyClose(sessionId);
      } catch {
        // Close is best effort; the parent should still refresh detection state.
      }
    }

    onClose();
  };

  return (
    <div
      aria-labelledby="pty-login-title"
      aria-modal="true"
      className="pty-overlay"
      role="dialog"
    >
      <section className="pty-panel">
        <header className="pty-header">
          <h2 id="pty-login-title">{`${agentLabel(agent)} — ${t("agents.login")}`}</h2>
          <button className="button ghost" onClick={close} type="button">
            {t("common.close")}
          </button>
        </header>
        <p className="pty-instructions">
          {`${agentLabel(agent)} — ${t("agents.login")} · ${t("agents.retry")}`}
        </p>

        {error ? (
          <div className="pty-error" role="alert">
            <p>{error}</p>
            <button className="button primary" onClick={close} type="button">
              {t("common.close")}
            </button>
          </div>
        ) : (
          <div
            aria-label={`${agentLabel(agent)} — ${t("agents.login")}`}
            className="pty-terminal"
            ref={containerRef}
          />
        )}
      </section>
    </div>
  );
}
