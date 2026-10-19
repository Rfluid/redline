//! Request/response handlers: artifacts, threads, messages, status.

use super::App;
use super::auth::{ApiResult, Author, bearer, not_found};
use super::limits::{self, JsonIn};
use crate::ids::now_ms;
use crate::model::{Artifact, DaemonStatus, Event, Health, RegisterArtifact, StatusChange, Submit, Thread};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, header};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use std::sync::Arc;

type S = State<Arc<App>>;

pub async fn health(State(app): S) -> Json<Health> {
    Json(Health {
        name: "redline".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        pid: std::process::id(),
        started: app.started,
    })
}

/// Daemon facts plus row counts, for `redline status`.
pub async fn status(State(app): S, h: HeaderMap) -> ApiResult<Json<DaemonStatus>> {
    app.require_admin(&h)?;
    let ((artifacts, by_status, events), head) = {
        let mut store = app.store();
        (store.counts()?, store.head()?)
    };
    Ok(Json(DaemonStatus {
        version: env!("CARGO_PKG_VERSION").into(),
        build: env!("REDLINE_GIT_SHA").into(),
        pid: std::process::id(),
        started: app.started,
        uptime_s: (now_ms() - app.started) / 1000,
        artifacts,
        threads: by_status.values().sum(),
        by_status,
        events,
        head,
        waiting: app.waiting(),
    }))
}

pub async fn register(State(app): S, h: HeaderMap, JsonIn(req): JsonIn<RegisterArtifact>) -> ApiResult<Json<Artifact>> {
    app.require_admin(&h)?;
    limits::register(&req)?;
    Ok(Json(app.store().register(&req)?))
}

pub async fn list_artifacts(State(app): S, h: HeaderMap) -> ApiResult<Json<Vec<Artifact>>> {
    app.require_admin(&h)?;
    Ok(Json(app.store().artifacts()?))
}

pub async fn get_artifact(State(app): S, h: HeaderMap, Path(id): Path<String>) -> ApiResult<Json<Artifact>> {
    let author = app.author_for(bearer(&h), &id)?;
    let mut a = app.store().artifact(&id)?.ok_or_else(|| not_found("artifact"))?;
    if author == Author::User {
        a.token = None;
    }
    Ok(Json(a))
}

pub async fn list_threads(State(app): S, h: HeaderMap, Path(id): Path<String>) -> ApiResult<Json<Vec<Thread>>> {
    app.author_for(bearer(&h), &id)?;
    Ok(Json(app.store().threads(&id)?))
}

/// New threads and replies in one call: "send this one" and "submit all drafts" are the same.
/// All or nothing: a reply to a thread on another artifact rejects the whole batch with 404.
pub async fn submit(
    State(app): S,
    h: HeaderMap,
    Path(id): Path<String>,
    JsonIn(req): JsonIn<Submit>,
) -> ApiResult<Json<Vec<Event>>> {
    let author = app.author_for(bearer(&h), &id)?.as_str();
    limits::submit(&req)?;
    let events = {
        let mut store = app.store();
        for r in &req.replies {
            if store.thread(&r.thread_id)?.is_none_or(|t| t.artifact_id != id) {
                return Err(not_found("thread"));
            }
        }
        store.submit(&id, author, &req)?
    };
    Ok(Json(events.into_iter().map(|e| app.publish(e)).collect()))
}

#[derive(Deserialize)]
pub struct MessageBody {
    body: String,
    status: Option<String>,
}

pub async fn add_message(
    State(app): S,
    h: HeaderMap,
    Path(id): Path<String>,
    JsonIn(b): JsonIn<MessageBody>,
) -> ApiResult<Json<Event>> {
    let author = app.thread_author(&h, &id)?.as_str();
    limits::message(&b.body)?;
    let ev = app.store().add_message(&id, author, &b.body, b.status.as_deref())?;
    Ok(Json(app.publish(ev)))
}

pub async fn set_status(
    State(app): S,
    h: HeaderMap,
    Path(id): Path<String>,
    JsonIn(b): JsonIn<StatusChange>,
) -> ApiResult<Json<Event>> {
    let author = app.thread_author(&h, &id)?.as_str();
    limits::note(b.note.as_deref())?;
    let ev = app.store().set_status(&id, &b.status, author, b.note.as_deref())?;
    Ok(Json(app.publish(ev)))
}

/// Raw attachment bytes (region screenshots), for the agent or that artifact's page.
pub async fn attachment(State(app): S, h: HeaderMap, Path(id): Path<String>) -> ApiResult<Response> {
    let (artifact, mime, data) = app.store().attachment(&id)?.ok_or_else(|| not_found("attachment"))?;
    app.author_for(bearer(&h), &artifact)?;
    let headers = [
        (header::CONTENT_TYPE, mime),
        (header::CACHE_CONTROL, "private, max-age=86400".into()),
        (header::X_CONTENT_TYPE_OPTIONS, "nosniff".into()),
    ];
    Ok((headers, data).into_response())
}
