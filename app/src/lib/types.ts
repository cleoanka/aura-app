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
