import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

import { AgentManager } from "./components/AgentManager";
import { AppShell, type ActiveView } from "./components/AppShell";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { AuraModePanel } from "./components/AuraMode/AuraModePanel";
import { AskPanel } from "./components/Ask/AskPanel";
import { NoteEditor } from "./components/Editor/NoteEditor";
import { GraphView } from "./components/Graph/GraphView";
import { SearchPanel } from "./components/Search/SearchPanel";
import { SettingsPanel } from "./components/Settings/SettingsPanel";
import { VaultExplorer } from "./components/Sidebar/VaultExplorer";
import { agentDetect } from "./lib/ipc";
import type { DoctorReport, NoteRef } from "./lib/types";

function App() {
  const [activeView, setActiveView] = useState<ActiveView>("workspace");
  const [selectedNote, setSelectedNote] = useState<NoteRef | null>(null);
  const [noteCount, setNoteCount] = useState(0);
  const [doctorReport, setDoctorReport] = useState<DoctorReport | null>(null);
  // Arka plan otomatik-reindex bitince (backend "index-updated" emit eder)
  // ilgili görünümler remount olup yeniden veri çeker.
  const [dataVersion, setDataVersion] = useState(0);

  useEffect(() => {
    const unlisten = listen("index-updated", () => setDataVersion((v) => v + 1));
    return () => {
      void unlisten.then((dispose) => dispose());
    };
  }, []);

  useEffect(() => {
    let alive = true;

    void agentDetect(false)
      .then((report) => {
        if (alive) {
          setDoctorReport(report);
        }
      })
      .catch(() => {
        if (alive) {
          setDoctorReport(null);
        }
      });

    return () => {
      alive = false;
    };
  }, []);

  const openNote = useCallback((note: NoteRef) => {
    setSelectedNote(note);
    setActiveView("workspace");
  }, []);

  const content = (() => {
    switch (activeView) {
      case "workspace":
        return (
          <div className="workspace-layout">
            <VaultExplorer
              key={dataVersion}
              activePath={selectedNote?.path ?? null}
              onNotesChange={setNoteCount}
              onOpenNote={openNote}
            />
            <NoteEditor note={selectedNote} />
          </div>
        );
      case "search":
        return <SearchPanel onOpenNote={openNote} />;
      case "ask":
        return <AskPanel />;
      case "aura-mode":
        return <AuraModePanel />;
      case "graph":
        return <GraphView key={dataVersion} onOpenNote={openNote} />;
      case "agents":
        return <AgentManager onReportChange={setDoctorReport} />;
      case "settings":
        return <SettingsPanel />;
    }
  })();

  return (
    <AppShell
      activeView={activeView}
      doctorReport={doctorReport}
      noteCount={noteCount}
      onActiveViewChange={setActiveView}
    >
      <ErrorBoundary resetKey={activeView}>{content}</ErrorBoundary>
    </AppShell>
  );
}

export default App;
