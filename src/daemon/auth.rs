//! Bearer-token auth and API errors.
//!
//! - admin token (`admin.token`, 0600): agents, MCP server, CLI. Speaks as `agent`.
//! - artifact token (embedded in the page): that artifact only. Speaks as `user`.

use super::App;
use axum::Json;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde_json::json;

pub struct ApiError(StatusCode, String);

impl ApiError {
    pub fn new(status: StatusCode, msg: String) -> Self {
        ApiError(status, msg)
    }

    #[must_use]
    pub fn with_status(self, status: StatusCode) -> Self {
        ApiError(status, self.1)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        tracing::info!(status = self.0.as_u16(), error = %self.1, "request failed");
        (self.0, Json(json!({ "error": self.1 }))).into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(e: anyhow::Error) -> Self {
        ApiError(StatusCode::BAD_REQUEST, format!("{e:#}"))
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

pub fn unauthorized() -> ApiError {
    ApiError(StatusCode::UNAUTHORIZED, "bad or missing token".into())
}

pub fn not_found(what: &str) -> ApiError {
    ApiError(StatusCode::NOT_FOUND, format!("no {what}"))
}

pub fn bearer(h: &HeaderMap) -> Option<&str> {
    h.get(header::AUTHORIZATION)?.to_str().ok()?.strip_prefix("Bearer ")
}

/// Who a token speaks as.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Author {
    Agent,
    User,
}

impl Author {
    pub fn as_str(self) -> &'static str {
        match self {
            Author::Agent => "agent",
            Author::User => "user",
        }
    }
}

impl App {
    pub(super) fn require_admin(&self, h: &HeaderMap) -> ApiResult<()> {
        if bearer(h) == Some(self.admin.as_str()) { Ok(()) } else { Err(unauthorized()) }
    }

    /// Admin token → agent; this artifact's token → user; anything else → 401/404.
    pub(super) fn author_for(&self, token: Option<&str>, artifact_id: &str) -> ApiResult<Author> {
        let token = token.ok_or_else(unauthorized)?;
        if token == self.admin {
            return Ok(Author::Agent);
        }
        let a = self.store().artifact(artifact_id)?.ok_or_else(|| not_found("artifact"))?;
        if a.token.as_deref() == Some(token) { Ok(Author::User) } else { Err(unauthorized()) }
    }

    pub(super) fn thread_author(&self, h: &HeaderMap, thread_id: &str) -> ApiResult<Author> {
        let t = self.store().thread(thread_id)?.ok_or_else(|| not_found("thread"))?;
        self.author_for(bearer(h), &t.artifact_id)
    }
}
