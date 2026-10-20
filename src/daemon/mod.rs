//! `redline serve`: the local broker between artifacts (browser side) and agents.
//!
//! Binds 127.0.0.1 only. See `docs/PROTOCOL.md` for the API and `auth` for the token model.

mod auth;
mod handlers;
mod limits;
mod live;
mod update;

use crate::ids::now_ms;
use crate::model::{DaemonState, Event};
use crate::paths;
use crate::store::Store;
use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::http::Request;
use axum::http::header;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::broadcast;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use tracing::{Level, info_span};
use tracing_subscriber::EnvFilter;

/// How often the daemon checks that `daemon.json` still names it.
const ORPHAN_CHECK: Duration = Duration::from_secs(10);
/// How often the daemon asks whether a release check is due (the check itself runs at most daily).
const UPDATE_TICK: Duration = Duration::from_hours(1);
/// How long browsers may cache a CORS preflight.
const CORS_MAX_AGE_SECS: u64 = 600;
const DEFAULT_LOG_FILTER: &str = "redline=info,tower_http=info";
/// `/client.js` is the anchors helpers plus the protocol client; `/overlay.js` the styles, icons, theme,
/// region picker, markdown renderer, markers, popovers, dock, update notice, view (state + painting), input handling, drawer bulk actions and
/// the overlay itself.
/// One URL each, so the embed snippet doesn't change when files split.
const CLIENT_JS: &str = concat!(include_str!("../../web/anchors.js"), include_str!("../../web/client.js"));
const OVERLAY_JS: &str = concat!(
    include_str!("../../web/overlay-style.js"),
    include_str!("../../web/icons.js"),
    include_str!("../../web/theme.js"),
    include_str!("../../web/regions.js"),
    include_str!("../../web/markdown.js"),
    include_str!("../../web/markers.js"),
    include_str!("../../web/popover.js"),
    include_str!("../../web/dock.js"),
    include_str!("../../web/update.js"),
    include_str!("../../web/overlay-view.js"),
    include_str!("../../web/overlay-input.js"),
    include_str!("../../web/bulk.js"),
    include_str!("../../web/overlay.js")
);

/// Shared daemon state.
pub struct App {
    store: Mutex<Store>,
    admin: String,
    tx: broadcast::Sender<Event>,
    /// Unix ms at startup; clients compare it with their binary's mtime (see `crate::lifecycle`).
    started: i64,
    /// Long-polls in progress, by filter (for `redline status`).
    waiting: Mutex<BTreeMap<String, usize>>,
    /// `config.toml` and the cached release check (see `crate::update`).
    update: crate::update::Files,
}

/// Counts one long-poll for its lifetime; dropping it (return or client gone) uncounts it.
pub(super) struct Waiting<'a> {
    app: &'a App,
    key: String,
}

impl Drop for Waiting<'_> {
    fn drop(&mut self) {
        let mut w = self.app.waiting.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(n) = w.get_mut(&self.key) {
            *n -= 1;
            if *n == 0 {
                w.remove(&self.key);
            }
        }
    }
}

impl App {
    #[must_use]
    pub fn new(store: Store, admin_token: String) -> Arc<Self> {
        Self::with_update_files(store, admin_token, crate::update::Files::default())
    }

    /// [`App::new`] with the update settings and cache somewhere else (tests).
    #[must_use]
    pub fn with_update_files(store: Store, admin_token: String, update: crate::update::Files) -> Arc<Self> {
        let (tx, _) = broadcast::channel(256);
        Arc::new(Self {
            store: Mutex::new(store),
            admin: admin_token,
            tx,
            started: now_ms(),
            waiting: Mutex::default(),
            update,
        })
    }

    /// Fans an event out to live subscribers (`WebSocket`s, long-polls) and hands it back.
    fn publish(&self, ev: Event) -> Event {
        tracing::debug!(seq = ev.seq, artifact = %ev.artifact_id, thread = %ev.thread.id, kind = %ev.kind, author = %ev.author, "event");
        // No receivers is fine: nobody is listening right now.
        let _ = self.tx.send(ev.clone());
        ev
    }

    pub(super) fn wait_guard(&self, key: String) -> Waiting<'_> {
        *self.waiting.lock().unwrap_or_else(std::sync::PoisonError::into_inner).entry(key.clone()).or_default() += 1;
        Waiting { app: self, key }
    }

    pub(super) fn waiting(&self) -> BTreeMap<String, usize> {
        self.waiting.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
    }

    /// Locks the store. A poisoned lock means a handler panicked mid-write; keep serving.
    fn store(&self) -> std::sync::MutexGuard<'_, Store> {
        self.store.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// All routes, with CORS open to any origin (the bearer token is the boundary).
pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/client.js", get(|| async { js(CLIENT_JS) }))
        .route("/overlay.js", get(|| async { js(OVERLAY_JS) }))
        .route("/api/artifacts", post(handlers::register).get(handlers::list_artifacts))
        .route("/api/artifacts/{id}", get(handlers::get_artifact))
        .route("/api/artifacts/{id}/threads", get(handlers::list_threads))
        .route("/api/artifacts/{id}/submit", post(handlers::submit))
        .route("/api/artifacts/{id}/ws", get(live::ws))
        .route("/api/threads/{id}/messages", post(handlers::add_message))
        .route("/api/threads/{id}/status", post(handlers::set_status))
        .route("/api/artifacts/{id}/update", get(update::info))
        .route("/api/artifacts/{id}/update/dismiss", post(update::dismiss))
        .route("/api/attachments/{id}", get(handlers::attachment))
        .route("/api/events", get(live::events))
        .route("/api/status", get(handlers::status))
        .layer(DefaultBodyLimit::max(limits::MAX_REQUEST_BYTES))
        .layer(
            // Path only: the WebSocket URL carries `?token=`, which must never reach the log.
            TraceLayer::new_for_http()
                .make_span_with(|req: &Request<_>| info_span!("http", method = %req.method(), path = %req.uri().path()))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        // After the trace layer, so it isn't logged: every client call checks it.
        .route("/health", get(handlers::health))
        .layer(cors())
        .with_state(app)
}

/// Any origin may call (artifacts load from file://, dev servers, ...): the bearer token is the
/// boundary. Headers are listed because `*` doesn't cover `Authorization` (Chrome deprecates relying on it).
fn cors() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        .max_age(Duration::from_secs(CORS_MAX_AGE_SECS))
}

/// Opens the store in the state dir, binds `127.0.0.1:port`, writes `daemon.json`, serves forever.
pub async fn serve(port: u16) -> anyhow::Result<()> {
    init_logging();
    let store = Store::open(&paths::state_dir()?.join("redline.db"))?;
    let app = App::new(store, paths::admin_token()?);
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    let port = listener.local_addr()?.port();
    let pid = std::process::id();
    paths::write_daemon_state(&DaemonState {
        port,
        pid,
        version: env!("CARGO_PKG_VERSION").into(),
        started: app.started,
    })?;
    tracing::info!(port, pid, version = env!("CARGO_PKG_VERSION"), "listening on 127.0.0.1");
    // `redline stop`: SIGTERM on Unix. Windows has no SIGTERM (stop kills the process there); Ctrl-Break
    // still reaches a daemon run in a console.
    #[cfg(unix)]
    let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    #[cfg(windows)]
    let mut term = tokio::signal::windows::ctrl_break()?;
    tokio::spawn(update::checks(app.update.clone()));
    tokio::select! {
        r = axum::serve(listener, router(app.clone())) => r?,
        () = orphaned(pid) => tracing::info!("daemon.json no longer names this process (state dir gone, or replaced); stopping"),
        _ = term.recv() => tracing::info!("terminated, stopping"),
        _ = tokio::signal::ctrl_c() => tracing::info!("interrupted, stopping"),
    }
    // Don't remove a newer daemon's state file.
    if paths::read_daemon_state().is_some_and(|s| s.pid == pid) {
        paths::remove_daemon_state()?;
    }
    Ok(())
}

/// Resolves once `daemon.json` stops naming `pid`: the state dir was deleted (tests, a user's
/// cleanup) or another daemon took over. A daemon nobody can find shouldn't keep running.
async fn orphaned(pid: u32) {
    let mut tick = tokio::time::interval(ORPHAN_CHECK);
    tick.tick().await;
    loop {
        tick.tick().await;
        let path = paths::state_dir_path().join("daemon.json");
        let mine = std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice::<DaemonState>(&b).ok())
            .is_some_and(|s| s.pid == pid);
        if !mine {
            return;
        }
    }
}

/// Logs to stderr (`daemon.log` when auto-started), or appends to `$REDLINE_LOG_FILE` when set (the
/// Windows auto-start can't redirect stderr). `RUST_LOG` overrides the default filter.
fn init_logging() {
    use tracing_subscriber::fmt::writer::BoxMakeWriter;
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_LOG_FILTER));
    let file = std::env::var_os("REDLINE_LOG_FILE")
        .and_then(|p| std::fs::OpenOptions::new().create(true).append(true).open(p).ok());
    let writer = match file {
        Some(f) => BoxMakeWriter::new(std::sync::Mutex::new(f)),
        None => BoxMakeWriter::new(std::io::stderr),
    };
    // Already set (tests, embedding): keep the existing subscriber.
    let _ = tracing_subscriber::fmt().with_env_filter(filter).with_ansi(false).with_writer(writer).try_init();
}

fn js(src: &'static str) -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "application/javascript; charset=utf-8")], src)
}

#[cfg(test)]
mod tests;
