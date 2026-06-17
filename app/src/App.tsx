import { useState } from "react";

import { AgentManager } from "./components/AgentManager";
import { AppShell } from "./components/AppShell";
import type { DoctorReport } from "./lib/types";

function App() {
  const [doctorReport, setDoctorReport] = useState<DoctorReport | null>(null);

  return (
    <AppShell activeView="Ajanlar" doctorReport={doctorReport}>
      <AgentManager onReportChange={setDoctorReport} />
    </AppShell>
  );
}

export default App;
