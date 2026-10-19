//! Live delivery: long-poll for agents, WebSocket push for artifacts.

use super::App;
use super::auth::ApiResult;
use crate::model::Event;
use crate::store::EventFilter;
use axum::Json;
use axum::extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::Response;
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;

const MAX_WAIT_SECS: u64 = 600;

#[derive(Deserialize)]
pub struct EventsQuery {
    #[serde(default)]
    since: i64,
    #[serde(default)]
    timeout: u64,
    artifact: Option<String>,
    session: Option<String>,
    author: Option<String>,
}

/// Long-poll: returns as soon as there are events after `since`, or empty after `timeout` seconds.
pub async fn events(State(app): State<Arc<App>>, h: HeaderMap, Query(q): Query<EventsQuery>) -> ApiResult<Json<Value>> {
    app.require_admin(&h)?;
    // Subscribe before the first read so an event landing in between still wakes us.
    let mut rx = app.tx.subscribe();
    let filter = EventFilter {
        since: q.since,
        artifact: q.artifact.as_deref(),
        session: q.session.as_deref(),
        author: q.author.as_deref(),
    };
    let deadline = tokio::time::Instant::now() + Duration::from_secs(q.timeout.min(MAX_WAIT_SECS));
    let key = match (&q.artifact, &q.session) {
        (Some(a), _) => format!("artifact:{a}"),
        (None, Some(s)) => format!("session:{s}"),
        (None, None) => "all".into(),
    };
    let _waiting = app.wait_guard(key);
    loop {
        let evs = app.store().events(&filter)?;
        if let Some(last) = evs.last() {
            let next = last.seq;
            return Ok(Json(json!({ "events": evs, "next": next })));
        }
        match tokio::time::timeout_at(deadline, rx.recv()).await {
            Ok(Ok(_) | Err(RecvError::Lagged(_))) => {}
            Ok(Err(RecvError::Closed)) | Err(_) => return Ok(Json(json!({ "events": [], "next": q.since }))),
        }
    }
}

#[derive(Deserialize)]
pub struct TokenQuery {
    token: Option<String>,
}

/// Push channel for the artifact side: every event on this artifact, as JSON text frames.
pub async fn ws(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
    Query(q): Query<TokenQuery>,
    up: WebSocketUpgrade,
) -> ApiResult<Response> {
    app.author_for(q.token.as_deref(), &id)?;
    let rx = app.tx.subscribe();
    Ok(up.on_upgrade(move |sock| ws_loop(sock, rx, id)))
}

async fn ws_loop(mut sock: WebSocket, mut rx: broadcast::Receiver<Event>, id: String) {
    tracing::debug!(artifact = %id, "ws open");
    loop {
        tokio::select! {
            ev = rx.recv() => match ev {
                Ok(ev) if ev.artifact_id == id => {
                    let Ok(text) = serde_json::to_string(&ev) else { continue };
                    if sock.send(WsMessage::Text(text.into())).await.is_err() { break }
                }
                // Lagged: the client resyncs via GET /threads on reconnect; keep going.
                Ok(_) | Err(RecvError::Lagged(_)) => {}
                Err(RecvError::Closed) => break,
            },
            msg = sock.recv() => if !matches!(msg, Some(Ok(_))) { break },
        }
    }
    tracing::debug!(artifact = %id, "ws closed");
}
