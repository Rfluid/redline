//! `redline mcp`: MCP server over stdio, built on rmcp.
//!
//! Tools (in `tools`) wrap the blocking agent-side client ([`crate::api`]). With `--channel`,
//! new user comments are also pushed as Claude Code channel notifications.

mod tools;

use crate::api::Api;
use crate::ids::new_id;
use crate::model::Event;
use rmcp::model::{
    CustomNotification, Implementation, JsonObject, ProtocolVersion, ServerCapabilities, ServerConfig,
    ServerNotification,
};
use rmcp::{Peer, RoleServer, ServerHandler, ServiceExt, tool_handler};
use serde_json::json;
use std::borrow::Cow;
use std::sync::Arc;
use std::sync::atomic::AtomicI64;
use std::time::Duration;

const INSTRUCTIONS: &str = "redline connects artifacts you build (HTML pages, apps) to this session. \
The user leaves inline comments on the artifact; you receive them, answer inline, and change the artifact. \
Flow: register_artifact -> put the returned `embed` snippet in the page -> wait_for_comments in a loop -> \
for each thread: set_status working, make the change, reply (shows under the user's comment). \
Follow the redline skill for details.";

const CHANNEL_INSTRUCTIONS: &str = " This server is also a Claude Code channel: new comments on artifacts you \
registered arrive by themselves as <channel source=\"redline\" artifact_id=... thread_id=...> messages, so you \
don't need to call wait_for_comments. Handle each one like a wait_for_comments event: list_threads for the full \
thread if needed, set_status working, make the change, then reply on that thread_id.";

/// Claude Code channels (<https://code.claude.com/docs/en/channels-reference>, checked 2026-10-16).
const CHANNEL_CAPABILITY: &str = "claude/channel";
const CHANNEL_METHOD: &str = "notifications/claude/channel";
/// Claude Code doesn't register a channel on protocol revision 2026-07-28 (rmcp's default), so a
/// channel server offers only the revisions before it.
const CHANNEL_PROTOCOLS: &[ProtocolVersion] = &[
    ProtocolVersion::V_2024_11_05,
    ProtocolVersion::V_2025_03_26,
    ProtocolVersion::V_2025_06_18,
    ProtocolVersion::V_2025_11_25,
];
/// Long-poll length for the push loop, and the back-off after an error.
const PUSH_WAIT_SECS: u64 = 300;
const PUSH_RETRY: Duration = Duration::from_secs(5);

/// One per agent session (process).
#[derive(Clone)]
pub struct Redline {
    /// Artifacts registered here are stamped with it; waits default to it.
    session: Arc<str>,
    /// Event cursor so `wait_for_comments` continues where the last call stopped.
    cursor: Arc<AtomicI64>,
    /// Running as a Claude Code channel (`--channel`).
    channel: bool,
}

// rmcp's `#[tool_handler]` expands to `async fn`s without `.await`; the lint (clippy ≥ 1.99) fires on
// that generated code, which we can't change.
#[allow(clippy::unused_async_trait_impl)]
#[tool_handler]
impl ServerHandler for Redline {
    fn get_info(&self) -> ServerConfig {
        let mut caps = ServerCapabilities::builder().enable_tools().build();
        let mut instructions = INSTRUCTIONS.to_string();
        if self.channel {
            caps.experimental = Some([(CHANNEL_CAPABILITY.to_string(), JsonObject::new())].into());
            instructions.push_str(CHANNEL_INSTRUCTIONS);
        }
        ServerConfig::new(caps)
            .with_server_info(Implementation::new("redline", env!("CARGO_PKG_VERSION")))
            .with_instructions(instructions)
    }

    fn supported_protocol_versions(&self) -> Cow<'static, [ProtocolVersion]> {
        if self.channel { Cow::Borrowed(CHANNEL_PROTOCOLS) } else { Cow::Borrowed(ProtocolVersion::KNOWN_VERSIONS) }
    }
}

/// Serves MCP on stdio until the client disconnects.
pub async fn run(channel: bool) -> anyhow::Result<()> {
    let session: Arc<str> = std::env::var("REDLINE_SESSION").unwrap_or_else(|_| new_id("ses")).into();
    let server = Redline { session: session.clone(), cursor: Arc::new(AtomicI64::new(0)), channel };
    let running = server.serve(rmcp::transport::stdio()).await?;
    if channel {
        tokio::spawn(push_loop(running.peer().clone(), session));
    }
    running.waiting().await?;
    Ok(())
}

/// Claude Code channels: push each new user comment into the session as it arrives.
/// Starts at the current head, so a restart doesn't replay old comments.
async fn push_loop(peer: Peer<RoleServer>, session: Arc<str>) {
    let mut since = None;
    loop {
        let s = session.clone();
        let res = tokio::task::spawn_blocking(move || -> anyhow::Result<(Vec<Event>, i64)> {
            let api = Api::connect()?;
            let since = match since {
                Some(n) => n,
                None => api.status()?.head,
            };
            api.wait_through_restarts(since, PUSH_WAIT_SECS, None, Some(&s))
        })
        .await;
        let Ok(Ok((events, next))) = res else {
            tokio::time::sleep(PUSH_RETRY).await;
            continue;
        };
        since = Some(next);
        for ev in events {
            if peer.send_notification(channel_note(&ev)).await.is_err() {
                return; // client went away
            }
        }
    }
}

/// One channel message per event. `meta` values must be strings with identifier keys.
fn channel_note(ev: &Event) -> ServerNotification {
    let last = ev.thread.messages.last().map_or("", |m| m.body.as_str());
    let quote = ev.thread.anchor.get("quote").and_then(|q| q.as_str());
    let on = quote.map_or_else(String::new, |q| format!(" on \"{q}\""));
    let params = json!({
        "content": format!("New redline comment{on}: {last}"),
        "meta": {
            "artifact_id": ev.artifact_id,
            "thread_id": ev.thread.id,
            "seq": ev.seq.to_string(),
            "status": ev.thread.status,
        },
    });
    ServerNotification::CustomNotification(CustomNotification::new(CHANNEL_METHOD, Some(params)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Message, Thread};

    #[test]
    fn channel_meta_is_string_valued() {
        let ev = Event {
            seq: 7,
            artifact_id: "art_1".into(),
            kind: "message".into(),
            author: "user".into(),
            at: 0,
            thread: Thread {
                id: "thr_1".into(),
                artifact_id: "art_1".into(),
                anchor: json!({"kind": "text", "quote": "Hello"}),
                status: "open".into(),
                created: 0,
                updated: 0,
                note: None,
                messages: vec![Message {
                    id: "m".into(),
                    thread_id: "thr_1".into(),
                    author: "user".into(),
                    body: "bigger".into(),
                    created: 0,
                }],
            },
        };
        let ServerNotification::CustomNotification(n) = channel_note(&ev) else { panic!("not custom") };
        assert_eq!(n.method, CHANNEL_METHOD);
        let p = n.params.unwrap();
        assert_eq!(p["content"], "New redline comment on \"Hello\": bigger");
        let meta = p["meta"].as_object().unwrap();
        assert!(meta.values().all(serde_json::Value::is_string), "{meta:?}");
        assert!(meta.keys().all(|k| k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')));
    }

    #[test]
    fn channel_mode_skips_the_newest_protocol() {
        let r = Redline { session: "s".into(), cursor: Arc::default(), channel: true };
        assert!(!r.supported_protocol_versions().contains(&ProtocolVersion::V_2026_07_28));
        let r = Redline { channel: false, ..r };
        assert!(r.supported_protocol_versions().contains(&ProtocolVersion::V_2026_07_28));
    }
}
