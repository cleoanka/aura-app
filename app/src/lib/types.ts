export type AgentId = "claude" | "gemini" | "codex";

export type AgentAuth =
  | "logged_in"
  | "logged_out"
  | "rate_limited"
  | "unknown";

export type TokenLocation = "keychain" | "file" | "unknown";

export type AgentStatus = {
  installed: boolean;
  path: string | null;
  version: string | null;
  auth: AgentAuth;
  token_location: TokenLocation;
  can_invoke: boolean | null;
  last_error: string | null;
};

export type DoctorReport = {
  schema: "aura.doctor.v1";
  agents: Record<AgentId, AgentStatus>;
};

export type NoteRef = {
  path: string;
  title: string;
};

export type SearchVia = "fts" | "vec" | "both" | string;

export type SearchHit = {
  note_path: string;
  heading_path: string;
  snippet: string;
  score: number;
  via: SearchVia;
};

export type IndexStats = {
  notes: number;
  chunks: number;
  skipped: number;
};

export type GraphNode = {
  id: string;
  title: string;
  dangling: boolean;
};

export type GraphLink = {
  source: string;
  target: string;
};

export type GraphData = {
  nodes: GraphNode[];
  links: GraphLink[];
};

export type ThemeMode = "dark" | "light";

export type DefaultMode = "ask" | "aura";

export type AuraMode = "chat" | "plan" | "review" | "fix" | "ship";

export type CacheMode = "off" | "read" | "write" | "read_write" | string;

export type LaneSettings = {
  fast?: boolean;
  deep?: boolean;
  lane0?: boolean;
};

export type Settings = {
  theme?: ThemeMode;
  default_mode?: DefaultMode;
  lanes?: LaneSettings;
  consensus_enabled?: boolean;
  cache_mode?: CacheMode;
  semantic_search?: boolean;
  [key: string]: unknown;
};

export type AiLane = "cached" | "fast" | "deep" | "consensus" | "lane0" | string;

export type AiEvent =
  | { kind: "start"; lane: AiLane }
  | { kind: "chunk"; text: string }
  | { kind: "cached"; text: string }
  | { kind: "done"; run_dir?: string }
  | { kind: "error"; reason: string; taxonomy: string };
