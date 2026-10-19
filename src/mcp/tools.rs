//! MCP tool definitions. Every tool has a CLI twin in `cli.rs`.

use super::Redline;
use crate::api::Api;
use crate::model::{RegisterArtifact, STATUSES};
use base64::Engine as _;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::ContentBlock;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::atomic::Ordering;

const DEFAULT_WAIT_SECS: u64 = 120;
/// Below common MCP client tool timeouts.
const MAX_WAIT_SECS: u64 = 540;

#[derive(Deserialize, schemars::JsonSchema)]
pub struct RegisterArgs {
    title: String,
    /// Where it is served or the file path, for your reference.
    uri: Option<String>,
    /// html, streamlit, react, ... (default html)
    kind: Option<String>,
    /// Existing artifact id to re-attach to this session.
    id: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct WaitArgs {
    /// Only this artifact; default: every artifact registered by this session.
    artifact_id: Option<String>,
    /// Seconds to block (default 120, max 540).
    timeout_s: Option<u64>,
    /// Event cursor; defaults to where the last call stopped.
    since: Option<i64>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct ListArgs {
    artifact_id: String,
    /// open | working | answered | resolved
    status: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct ReplyArgs {
    thread_id: String,
    /// Markdown.
    body: String,
    /// Defaults to `answered`.
    status: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct StatusArgs {
    thread_id: String,
    /// open | working | answered | resolved
    status: String,
    /// Optional one-line progress note shown to the user under the status, e.g. "Rebuilding the chart".
    /// Cleared by your next reply or status change.
    note: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct ImageArgs {
    /// The `image` id from a region anchor (`att_…`).
    attachment_id: String,
}

/// Tells the agent what to do with `update` (CLI agents read the same in SKILL.md).
const UPDATE_HINT: &str = "A newer redline is out. Tell the user in one line (current → latest) and offer to run \
    `redline update`; don't update without their OK. They can skip this release (`redline update skip`) or turn \
    notices off (`redline update off`).";

/// Tool output: pretty JSON on success, an `isError` text result on failure.
type ToolResult = Result<String, String>;

/// Runs a blocking API call off the async runtime and renders it as pretty JSON.
async fn blocking(f: impl FnOnce(Api) -> anyhow::Result<Value> + Send + 'static) -> ToolResult {
    tokio::task::spawn_blocking(move || Api::connect().and_then(f))
        .await
        .map_err(|e| e.to_string())?
        .map(|v| serde_json::to_string_pretty(&v).unwrap_or_default())
        .map_err(|e| format!("{e:#}"))
}

#[tool_router(vis = "pub(crate)")]
impl Redline {
    #[tool(
        description = "Register an artifact for inline review (or re-attach one by id). Returns the `embed` snippet to put in the page."
    )]
    async fn register_artifact(&self, Parameters(a): Parameters<RegisterArgs>) -> ToolResult {
        let session = self.session.to_string();
        blocking(move |api| {
            let req = RegisterArtifact { id: a.id, title: a.title, uri: a.uri, kind: a.kind, session: Some(session.clone()) };
            let art = api.register(&req)?;
            let since = api.status()?.head;
            let update = crate::update::info(&crate::update::Files::default());
            let mut v = json!({
                "artifact_id": art.id,
                "embed": api.embed(&art),
                "session": session,
                "since": since,
                "next": format!(
                    "Add `embed` to the artifact's HTML (end of <body>), then call wait_for_comments. \
                     With a background shell you can instead run `redline wait --session {session} --since {since} --timeout 600` \
                     in the background and act when it exits."
                ),
            });
            if update.notify {
                v["update"] = json!(update);
                v["update_hint"] = json!(UPDATE_HINT);
            }
            Ok(v)
        })
        .await
    }

    #[tool(
        description = "Block until the user submits comments or replies on this session's artifacts (or one artifact), or until timeout. Returns events with full thread state. Call again with no `since` to continue."
    )]
    async fn wait_for_comments(&self, Parameters(a): Parameters<WaitArgs>) -> ToolResult {
        let timeout = a.timeout_s.unwrap_or(DEFAULT_WAIT_SECS).min(MAX_WAIT_SECS);
        let since = a.since.unwrap_or_else(|| self.cursor.load(Ordering::SeqCst));
        let session = a.artifact_id.is_none().then(|| self.session.to_string());
        let cursor = self.cursor.clone();
        blocking(move |api| {
            let (events, next) =
                api.wait_through_restarts(since, timeout, a.artifact_id.as_deref(), session.as_deref())?;
            cursor.fetch_max(next, Ordering::SeqCst);
            Ok(json!({ "events": events, "next": next, "timed_out": events.is_empty() }))
        })
        .await
    }

    #[tool(description = "All comment threads on an artifact, with messages and status.")]
    async fn list_threads(&self, Parameters(a): Parameters<ListArgs>) -> ToolResult {
        blocking(move |api| {
            let mut threads = api.threads(&a.artifact_id)?;
            if let Some(st) = a.status {
                threads.retain(|t| t.status == st);
            }
            Ok(json!(threads))
        })
        .await
    }

    #[tool(description = "Answer inline under the user's comment. Sets status to `answered` unless given.")]
    async fn reply(&self, Parameters(a): Parameters<ReplyArgs>) -> ToolResult {
        blocking(move |api| Ok(json!(api.reply(&a.thread_id, &a.body, a.status.as_deref())?))).await
    }

    #[tool(
        description = "Mark a thread working (you're on it; the user sees a spinner and your optional `note`), answered, resolved, or open."
    )]
    async fn set_status(&self, Parameters(a): Parameters<StatusArgs>) -> ToolResult {
        if !STATUSES.contains(&a.status.as_str()) {
            return Err(format!("status must be one of {STATUSES:?}"));
        }
        blocking(move |api| Ok(json!(api.set_status(&a.thread_id, &a.status, a.note.as_deref())?))).await
    }

    #[tool(
        description = "Fetch the screenshot attached to a region comment (anchor `image`: \"att_…\") so you can see what the user pointed at."
    )]
    async fn get_image(&self, Parameters(a): Parameters<ImageArgs>) -> Result<ContentBlock, String> {
        let (mime, data) = tokio::task::spawn_blocking(move || Api::connect()?.attachment(&a.attachment_id))
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| format!("{e:#}"))?;
        Ok(ContentBlock::image(base64::engine::general_purpose::STANDARD.encode(data), mime))
    }
}
