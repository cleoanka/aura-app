import { Channel, invoke } from "@tauri-apps/api/core";

import type {
  AgentId,
  AgentTestResult,
  AiEvent,
  AuraMode,
  DoctorReport,
  GraphData,
  IndexStats,
  NoteRef,
  SearchHit,
  Settings,
} from "./types";

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

export type EmbeddingStatus = {
  backend: string;
  model: string;
  ready: boolean;
  downloading: boolean;
  device: string;
  cache_path: string;
};

export type OllamaStatus = {
  installed: boolean;
  running: boolean;
  models: string[];
};

export async function embeddingStatus(): Promise<EmbeddingStatus> {
  try {
    return await invoke<EmbeddingStatus>("embedding_status");
  } catch (error) {
    throw readableError(error);
  }
}

export async function prepareEmbeddingModel(
  onLine: (line: string) => void,
): Promise<EmbeddingStatus> {
  const channel = new Channel<string>();
  channel.onmessage = onLine;

  try {
    return await invoke<EmbeddingStatus>("prepare_embedding_model", {
      onEvent: channel,
    });
  } catch (error) {
    throw readableError(error);
  }
}

export async function ollamaStatus(baseUrl?: string): Promise<OllamaStatus> {
  try {
    return await invoke<OllamaStatus>("ollama_status", { baseUrl });
  } catch (error) {
    throw readableError(error);
  }
}

export async function ollamaPull(
  model: string,
  baseUrl: string | undefined,
  onLine: (line: string) => void,
): Promise<void> {
  const channel = new Channel<string>();
  channel.onmessage = onLine;

  try {
    await invoke<void>("ollama_pull", { model, baseUrl, onEvent: channel });
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

export async function agentTest(agent: AgentId): Promise<AgentTestResult> {
  try {
    return await invoke<AgentTestResult>("agent_test", { agent });
  } catch (error) {
    throw readableError(error);
  }
}

export async function ptyOpen(
  agent: AgentId,
  onOutput: (s: string) => void,
): Promise<string> {
  const channel = new Channel<string>();
  channel.onmessage = onOutput;

  try {
    return await invoke<string>("pty_open", { agent, onOutput: channel });
  } catch (error) {
    throw readableError(error);
  }
}

export async function ptyWrite(sessionId: string, data: string): Promise<void> {
  try {
    await invoke<void>("pty_write", { sessionId, data });
  } catch (error) {
    throw readableError(error);
  }
}

export async function ptyResize(
  sessionId: string,
  rows: number,
  cols: number,
): Promise<void> {
  try {
    await invoke<void>("pty_resize", { sessionId, rows, cols });
  } catch (error) {
    throw readableError(error);
  }
}

export async function ptyClose(sessionId: string): Promise<void> {
  try {
    await invoke<void>("pty_close", { sessionId });
  } catch (error) {
    throw readableError(error);
  }
}

export async function listNotes(): Promise<NoteRef[]> {
  try {
    return await invoke<NoteRef[]>("list_notes");
  } catch (error) {
    throw readableError(error);
  }
}

export async function readNote(path: string): Promise<string> {
  try {
    return await invoke<string>("read_note", { path });
  } catch (error) {
    throw readableError(error);
  }
}

export async function writeNote(path: string, content: string): Promise<void> {
  try {
    await invoke<void>("write_note", { path, content });
  } catch (error) {
    throw readableError(error);
  }
}

export async function saveNote(kind: string, content: string): Promise<string> {
  try {
    return await invoke<string>("save_note", { kind, content });
  } catch (error) {
    throw readableError(error);
  }
}

export async function searchHybrid(query: string, k = 10): Promise<SearchHit[]> {
  try {
    return await invoke<SearchHit[]>("search_hybrid", { query, k });
  } catch (error) {
    throw readableError(error);
  }
}

export async function pickVaultFolder(): Promise<string | null> {
  try {
    return await invoke<string | null>("pick_vault_folder");
  } catch (error) {
    throw readableError(error);
  }
}

export async function indexVault(path: string): Promise<IndexStats> {
  try {
    return await invoke<IndexStats>("index_vault", { path });
  } catch (error) {
    throw readableError(error);
  }
}

export async function getGraph(): Promise<GraphData> {
  try {
    return await invoke<GraphData>("get_graph");
  } catch (error) {
    throw readableError(error);
  }
}

export async function getSettings(): Promise<Settings> {
  try {
    return await invoke<Settings>("get_settings");
  } catch (error) {
    throw readableError(error);
  }
}

export async function setSettings(settings: Settings): Promise<void> {
  try {
    await invoke<void>("set_settings", { settings });
  } catch (error) {
    throw readableError(error);
  }
}

export async function ask(
  query: string,
  onEvent: (event: AiEvent) => void,
): Promise<string> {
  const channel = new Channel<AiEvent>();
  channel.onmessage = onEvent;

  try {
    return await invoke<string>("ask", { query, onEvent: channel });
  } catch (error) {
    throw readableError(error);
  }
}

export async function runMode(
  mode: AuraMode,
  prompt: string,
  projectDir: string | null,
  onEvent: (event: AiEvent) => void,
): Promise<string> {
  const channel = new Channel<AiEvent>();
  channel.onmessage = onEvent;

  try {
    return await invoke<string>("run_mode", {
      mode,
      prompt,
      projectDir,
      onEvent: channel,
    });
  } catch (error) {
    throw readableError(error);
  }
}

export async function askConsensus(
  query: string,
  onEvent: (event: AiEvent) => void,
): Promise<string> {
  const channel = new Channel<AiEvent>();
  channel.onmessage = onEvent;

  try {
    return await invoke<string>("ask_consensus", { query, onEvent: channel });
  } catch (error) {
    throw readableError(error);
  }
}

export async function chat(
  message: string,
  onEvent: (event: AiEvent) => void,
): Promise<string> {
  const channel = new Channel<AiEvent>();
  channel.onmessage = onEvent;

  try {
    return await invoke<string>("chat", { message, onEvent: channel });
  } catch (error) {
    throw readableError(error);
  }
}

export async function cancelJob(id: string): Promise<void> {
  try {
    await invoke<void>("cancel_job", { jobId: id });
  } catch (error) {
    throw readableError(error);
  }
}
