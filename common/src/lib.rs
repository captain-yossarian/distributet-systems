use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateMessage {
    pub message: String,
    /// Write concern ("w"): total number of nodes — master plus secondaries
    /// — that must have the message before responding to the client.
    /// Master's own write always counts as 1, so `w: 1` (the default when
    /// omitted) means "master only, don't wait on any secondary", and
    /// `w: 1 + N` means "master plus all N currently registered
    /// secondaries". Clamped to `[1, 1 + secondaries.len()]`.
    #[serde(default)]
    pub write_concern: Option<usize>,
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

/// User-controllable test knobs for a secondary: a display name (purely
/// cosmetic — the registry `id` is still used for routing/keys) and an
/// artificial delay before it ACKs a received message. Actually taking a
/// secondary down for testing is a real `docker stop`/`start` (see
/// `SecondaryNode.running` and the `/secondaries/:id/stop` and `/start`
/// endpoints), not a setting.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SecondarySettings {
    pub name: String,
    pub delay_ms: u64,
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

/// A secondary as shown to the UI: the union of what Docker knows (does the
/// container exist, is it running) and what the live self-registration
/// registry knows (its reachable address, only present while running and
/// heartbeating). A stopped secondary still appears here — with
/// `running: false` and `address: None` — so the UI has something to call
/// `/secondaries/:id/start` on.
#[derive(Debug, Clone, Serialize)]
pub struct SecondaryNode {
    pub id: String,
    pub address: Option<String>,
    pub running: bool,
}

/// One row of a `docker ps`-style listing, read live from the docker socket.
#[derive(Debug, Clone, Serialize)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub state: String,
    pub status: String,
}
