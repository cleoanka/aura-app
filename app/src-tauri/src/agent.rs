use std::collections::BTreeMap;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentAuth {
    LoggedIn,
    LoggedOut,
    RateLimited,
    Unknown,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenLocation {
    Keychain,
    File,
    Unknown,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct AgentStatus {
    pub installed: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub auth: AgentAuth,
    pub token_location: TokenLocation,
    pub can_invoke: Option<bool>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct DoctorReport {
    pub schema: String,
    pub agents: BTreeMap<String, AgentStatus>,
}
