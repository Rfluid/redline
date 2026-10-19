//! Update notices for the overlay: what's new, and dismissing it (one version, or all notices).
//! Any token valid for the artifact may read and dismiss: dismissing only hides a notice.

use super::App;
use super::auth::{ApiResult, bearer};
use super::limits::JsonIn;
use crate::model::UpdateInfo;
use crate::update::{self, Files};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use serde::Deserialize;
use std::sync::Arc;

pub async fn info(State(app): State<Arc<App>>, h: HeaderMap, Path(id): Path<String>) -> ApiResult<Json<UpdateInfo>> {
    app.author_for(bearer(&h), &id)?;
    Ok(Json(update::info(&app.update)))
}

/// `{"version": "0.3.0"}` skips that release; `{"all": true}` turns checks and notices off.
#[derive(Deserialize)]
pub struct Dismiss {
    version: Option<String>,
    #[serde(default)]
    all: bool,
}

pub async fn dismiss(
    State(app): State<Arc<App>>,
    h: HeaderMap,
    Path(id): Path<String>,
    JsonIn(d): JsonIn<Dismiss>,
) -> ApiResult<Json<UpdateInfo>> {
    app.author_for(bearer(&h), &id)?;
    let u = match (d.all, d.version.as_deref()) {
        (true, _) => update::set_check(&app.update, false)?,
        (false, Some(v)) => update::skip(&app.update, Some(v))?,
        (false, None) => return Err(anyhow::anyhow!("give `version` or `all: true`").into()),
    };
    tracing::info!(version = ?d.version, all = d.all, "update notice dismissed");
    Ok(Json(u))
}

/// Background release checks, at most daily (see [`update::auto_check`]). Never fails the daemon.
pub async fn checks(files: Files) {
    let mut tick = tokio::time::interval(super::UPDATE_TICK);
    loop {
        tick.tick().await;
        let f = files.clone();
        match tokio::task::spawn_blocking(move || update::auto_check(&f)).await {
            Ok(Some(c)) if c.error.is_some() => tracing::info!(error = ?c.error, "release check failed"),
            Ok(Some(c)) => tracing::info!(latest = ?c.latest, "release check"),
            _ => {}
        }
    }
}
