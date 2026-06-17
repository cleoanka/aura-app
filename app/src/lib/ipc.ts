import { invoke } from "@tauri-apps/api/core";

import type { AgentId, DoctorReport } from "./types";

function readableError(error: unknown): Error {
  if (error instanceof Error) {
    return new Error(error.message || "Beklenmeyen bir hata oluştu.");
  }

  if (typeof error === "string") {
    return new Error(error || "Beklenmeyen bir hata oluştu.");
  }

  if (typeof error === "object" && error !== null) {
    const maybeMessage = "message" in error ? error.message : undefined;

    if (typeof maybeMessage === "string" && maybeMessage.trim().length > 0) {
      return new Error(maybeMessage);
    }
  }

  return new Error("Beklenmeyen bir hata oluştu.");
}

export async function agentDetect(probe = false): Promise<DoctorReport> {
  try {
    return await invoke<DoctorReport>("agent_detect", { probe });
  } catch (error) {
    throw readableError(error);
  }
}

export async function agentInstall(id: AgentId): Promise<string> {
  try {
    return await invoke<string>("agent_install", { id });
  } catch (error) {
    throw readableError(error);
  }
}
