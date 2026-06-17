import { ModelManager } from "./Models/ModelManager";
import type { DoctorReport } from "../lib/types";

type AgentManagerProps = {
  onReportChange: (report: DoctorReport | null) => void;
};

// The legacy "Agent Manager" has been rebuilt as the unified "AI & Models"
// view. AgentManager is kept as a thin wrapper so existing imports/usages keep
// working while delegating to the new ModelManager.
export function AgentManager(props: AgentManagerProps) {
  return <ModelManager {...props} />;
}

export { ModelManager };
