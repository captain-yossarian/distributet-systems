use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateMessage {
    pub message: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggedMessage {
    pub message: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MasterLogEntry {
    pub message: String,
    pub timestamp: String,
    /// Number of secondaries the message was broadcast to.
    pub sent: usize,
    /// Number of secondaries that ACKed receipt (a successful `/receive` response).
    pub acked: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SecondaryInfo {
    pub id: String,
    pub address: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BroadcastResult {
    pub message: String,
    pub timestamp: String,
    pub delivered: Vec<String>,
    pub failed: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SecondaryLog {
    pub id: String,
    pub messages: Vec<LoggedMessage>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogsResponse {
    pub master: Vec<MasterLogEntry>,
    /// Ordered by registration order (oldest first), not alphabetically or by id hash.
    pub secondaries: Vec<SecondaryLog>,
}
