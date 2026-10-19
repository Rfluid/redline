//! Wire types shared by the daemon, the MCP server and the CLI.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Thread lifecycle. Drafts never reach the daemon: the browser keeps them until submit.
pub const STATUSES: &[&str] = &["open", "working", "answered", "resolved"];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Artifact {
    pub id: String,
    pub title: String,
    pub uri: Option<String>,
    /// Free-form surface hint: html, streamlit, react, notebook, ...
    pub kind: String,
    /// MCP session that owns the artifact; events for it are routed there.
    pub session: Option<String>,
    /// Browser-side token. Only returned to the agent (admin) side.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    pub created: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Message {
    pub id: String,
    pub thread_id: String,
    /// "user" (artifact side) or "agent".
    pub author: String,
    pub body: String,
    pub created: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Thread {
    pub id: String,
    pub artifact_id: String,
    /// Opaque, surface-defined location (see docs/ARCHITECTURE.md#anchors).
    pub anchor: Value,
    pub status: String,
    pub created: i64,
    pub updated: i64,
    /// What the agent says it's doing right now (set with a status change; cleared by the next message
    /// or status change).
    #[serde(default)]
    pub note: Option<String>,
    pub messages: Vec<Message>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Event {
    pub seq: i64,
    pub artifact_id: String,
    /// "message" or "status".
    pub kind: String,
    pub author: String,
    pub at: i64,
    /// Thread state at read time.
    pub thread: Thread,
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct RegisterArtifact {
    pub id: Option<String>,
    pub title: String,
    pub uri: Option<String>,
    pub kind: Option<String>,
    pub session: Option<String>,
}

/// One submit carries any number of new threads and replies, so "send this one"
/// and "submit all drafts" are the same call.
#[derive(Deserialize, Serialize, Debug, Default)]
pub struct Submit {
    #[serde(default)]
    pub threads: Vec<NewThread>,
    #[serde(default)]
    pub replies: Vec<NewReply>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct NewThread {
    pub anchor: Value,
    pub body: String,
    /// Optional screenshot of the region (region anchors). Stored as an attachment; the stored
    /// anchor gets `image: "att_…"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<Image>,
}

/// An inline image: `mime` (png, jpeg or webp) and base64 `data`.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Image {
    pub mime: String,
    pub data: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct NewReply {
    pub thread_id: String,
    pub body: String,
    pub status: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct StatusChange {
    pub status: String,
    /// Short progress note shown under the status, e.g. "Rebuilding the chart".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DaemonState {
    pub port: u16,
    pub pid: u32,
    pub version: String,
    /// Unix ms when the daemon started. Older `daemon.json` files lack it.
    #[serde(default)]
    pub started: i64,
}

/// `GET /health`: unauthenticated, enough for a client to decide whether to restart the daemon.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Health {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub pid: u32,
    /// Unix ms when the daemon started.
    #[serde(default)]
    pub started: i64,
}

/// `GET /api/status` (admin): what `redline status` prints, minus the port.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DaemonStatus {
    pub version: String,
    /// Commit the daemon was built from.
    #[serde(default)]
    pub build: String,
    pub pid: u32,
    pub started: i64,
    pub uptime_s: i64,
    pub artifacts: i64,
    pub threads: i64,
    /// Thread count per status (`open`, `working`, ...).
    pub by_status: std::collections::BTreeMap<String, i64>,
    pub events: i64,
    /// Latest event `seq`: pass it as `since` to wait only for what comes next.
    pub head: i64,
    /// Long-polls in progress, by filter (`artifact:<id>`, `session:<id>` or `all`): who is listening.
    #[serde(default)]
    pub waiting: std::collections::BTreeMap<String, usize>,
}

/// What redline knows about a newer release (`redline update check`, `GET /api/artifacts/{id}/update`).
/// Built from the cached check and `config.toml`; reading it never touches the network.
// A wire type: the flags are independent facts clients read directly, not a state machine.
#[allow(clippy::struct_excessive_bools)]
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct UpdateInfo {
    /// This binary's version.
    pub current: String,
    /// Newest release seen by the last check, if any.
    pub latest: Option<String>,
    /// `latest` is newer than `current`.
    pub available: bool,
    /// Show a notice: available, checks on, and `latest` not skipped.
    pub notify: bool,
    /// `[update] check` in `config.toml`.
    pub check: bool,
    /// `latest` is in `[update] skip`.
    pub skipped: bool,
    /// Unix ms of the last check, if any.
    pub checked: Option<i64>,
    /// How to update.
    pub command: String,
    /// Release notes for `latest`.
    pub notes: Option<String>,
}
