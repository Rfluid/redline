//! In-process HTTP tests: `router` + an in-memory store, driven with `tower::ServiceExt::oneshot`.

use super::*;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode};
use serde_json::{Value, json};
use std::path::Path;
use std::time::{Duration, Instant};
use tower::ServiceExt;

pub(super) const ADMIN: &str = "admin-token";

pub(super) struct T {
    router: Router,
    /// Holds `config.toml` and `update.json`, so no test reads or writes the user's.
    pub(super) dir: tempfile::TempDir,
}

impl T {
    pub(super) fn new() -> Self {
        let store = Store::open(Path::new(":memory:")).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let files =
            crate::update::Files { config: dir.path().join("config.toml"), cache: dir.path().join("update.json") };
        Self { router: router(App::with_update_files(store, ADMIN.into(), files)), dir }
    }

    pub(super) async fn call(
        &self,
        method: Method,
        path: &str,
        token: Option<&str>,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut req = Request::builder().method(method).uri(path);
        if let Some(t) = token {
            req = req.header("authorization", format!("Bearer {t}"));
        }
        let body = match body {
            Some(b) => {
                req = req.header("content-type", "application/json");
                Body::from(b.to_string())
            }
            None => Body::empty(),
        };
        let resp = self.router.clone().oneshot(req.body(body).unwrap()).await.unwrap();
        let status = resp.status();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
    }

    pub(super) async fn get(&self, path: &str, token: Option<&str>) -> (StatusCode, Value) {
        self.call(Method::GET, path, token, None).await
    }

    pub(super) async fn post(&self, path: &str, token: Option<&str>, body: Value) -> (StatusCode, Value) {
        self.call(Method::POST, path, token, Some(body)).await
    }

    /// Registers an artifact; returns `(id, token)`.
    pub(super) async fn artifact(&self, session: &str) -> (String, String) {
        let (st, a) = self.post("/api/artifacts", Some(ADMIN), json!({"title": "t", "session": session})).await;
        assert_eq!(st, StatusCode::OK);
        (a["id"].as_str().unwrap().into(), a["token"].as_str().unwrap().into())
    }

    pub(super) async fn comment(&self, art: &str, tok: &str, body: &str) -> String {
        let (st, evs) = self
            .post(
                &format!("/api/artifacts/{art}/submit"),
                Some(tok),
                json!({"threads": [{"anchor": {}, "body": body}]}),
            )
            .await;
        assert_eq!(st, StatusCode::OK, "{evs}");
        evs[0]["thread"]["id"].as_str().unwrap().into()
    }
}

#[tokio::test]
async fn auth_matrix() {
    let t = T::new();
    let (a, tok_a) = t.artifact("s").await;
    let (_b, tok_b) = t.artifact("s").await;
    let threads = format!("/api/artifacts/{a}/threads");

    // Artifact routes: admin and the artifact's own token pass; another artifact's token and none fail.
    for (tok, want) in [
        (Some(ADMIN), StatusCode::OK),
        (Some(tok_a.as_str()), StatusCode::OK),
        (Some(tok_b.as_str()), StatusCode::UNAUTHORIZED),
        (Some("nope"), StatusCode::UNAUTHORIZED),
        (None, StatusCode::UNAUTHORIZED),
    ] {
        assert_eq!(t.get(&threads, tok).await.0, want, "threads with {tok:?}");
    }

    // Admin-only routes reject artifact tokens.
    for path in ["/api/artifacts", "/api/events?timeout=0", "/api/status"] {
        assert_eq!(t.get(path, Some(ADMIN)).await.0, StatusCode::OK, "{path}");
        assert_eq!(t.get(path, Some(&tok_a)).await.0, StatusCode::UNAUTHORIZED, "{path}");
        assert_eq!(t.get(path, None).await.0, StatusCode::UNAUTHORIZED, "{path}");
    }
    let reg = json!({"title": "x"});
    assert_eq!(t.post("/api/artifacts", Some(&tok_a), reg).await.0, StatusCode::UNAUTHORIZED);

    // The artifact never sees its own token; the agent does.
    assert!(t.get(&format!("/api/artifacts/{a}"), Some(&tok_a)).await.1.get("token").is_none());
    assert_eq!(t.get(&format!("/api/artifacts/{a}"), Some(ADMIN)).await.1["token"], tok_a);

    // Thread routes check the thread's artifact.
    let thr = t.comment(&a, &tok_a, "hi").await;
    let msg = format!("/api/threads/{thr}/messages");
    assert_eq!(t.post(&msg, Some(&tok_b), json!({"body": "x"})).await.0, StatusCode::UNAUTHORIZED);
    let (st, ev) = t.post(&msg, Some(&tok_a), json!({"body": "more"})).await;
    assert_eq!((st, ev["author"].as_str()), (StatusCode::OK, Some("user")));
    let (st, ev) = t.post(&msg, Some(ADMIN), json!({"body": "done"})).await;
    assert_eq!(
        (st, ev["author"].as_str(), ev["thread"]["status"].as_str()),
        (StatusCode::OK, Some("agent"), Some("answered"))
    );
    assert_eq!(
        t.post("/api/threads/thr_nope/status", Some(ADMIN), json!({"status": "open"})).await.0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn submit_mixes_threads_and_replies() {
    let t = T::new();
    let (a, tok) = t.artifact("s").await;
    let thr = t.comment(&a, &tok, "first").await;
    let (st, evs) = t
        .post(
            &format!("/api/artifacts/{a}/submit"),
            Some(&tok),
            json!({
                "threads": [{"anchor": {"kind": "text", "quote": "x"}, "body": "new one"}],
                "replies": [{"thread_id": thr, "body": "and this"}],
            }),
        )
        .await;
    assert_eq!(st, StatusCode::OK);
    let evs = evs.as_array().unwrap();
    assert_eq!(evs.len(), 2);
    assert!(evs[0]["seq"].as_i64() < evs[1]["seq"].as_i64());
    assert_eq!(evs[1]["thread"]["messages"].as_array().unwrap().len(), 2);
    assert_eq!(t.get(&format!("/api/artifacts/{a}/threads"), Some(&tok)).await.1.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn cross_artifact_reply_rejects_the_whole_batch() {
    let t = T::new();
    let (a, tok_a) = t.artifact("s").await;
    let (b, tok_b) = t.artifact("s").await;
    let thr_b = t.comment(&b, &tok_b, "on b").await;
    let (st, _) = t
        .post(
            &format!("/api/artifacts/{a}/submit"),
            Some(&tok_a),
            json!({"threads": [{"anchor": {}, "body": "would be orphaned"}], "replies": [{"thread_id": thr_b, "body": "x"}]}),
        )
        .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    assert_eq!(t.get(&format!("/api/artifacts/{a}/threads"), Some(&tok_a)).await.1, json!([]), "nothing written");
    let events = t.get("/api/events?timeout=0", Some(ADMIN)).await.1;
    assert_eq!(events["events"].as_array().unwrap().len(), 1, "only b's comment");
}

#[tokio::test]
async fn long_poll_wakes_on_submit() {
    let t = T::new();
    let (a, tok) = t.artifact("ses_x").await;
    let waiter = {
        let r = t.router.clone();
        tokio::spawn(async move {
            let req = Request::get("/api/events?since=0&timeout=10&session=ses_x&author=user")
                .header("authorization", format!("Bearer {ADMIN}"))
                .body(Body::empty())
                .unwrap();
            let start = Instant::now();
            let resp = r.oneshot(req).await.unwrap();
            let v: Value = serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap();
            (start.elapsed(), v)
        })
    };
    tokio::time::sleep(Duration::from_millis(100)).await;
    let waiting = t.get("/api/status", Some(ADMIN)).await.1["waiting"].clone();
    assert_eq!(waiting, json!({"session:ses_x": 1}), "status shows the open long-poll");
    t.comment(&a, &tok, "wake up").await;
    let (elapsed, v) = waiter.await.unwrap();
    assert!(elapsed < Duration::from_secs(5), "woke after {elapsed:?}");
    assert_eq!(v["events"][0]["thread"]["messages"][0]["body"], "wake up");
    assert_eq!(v["next"], v["events"][0]["seq"]);
    assert_eq!(t.get("/api/status", Some(ADMIN)).await.1["waiting"], json!({}), "uncounted when done");
}

#[tokio::test]
async fn long_poll_times_out_empty() {
    let t = T::new();
    let (st, v) = t.get("/api/events?since=0&timeout=1", Some(ADMIN)).await;
    assert_eq!((st, v), (StatusCode::OK, json!({"events": [], "next": 0})));
}

mod inputs;
mod update;

#[tokio::test]
async fn cors_preflight_names_authorization() {
    let t = T::new();
    let req = Request::builder()
        .method(Method::OPTIONS)
        .uri("/api/artifacts/x/threads")
        .header("origin", "http://localhost:5173")
        .header("access-control-request-method", "GET")
        .header("access-control-request-headers", "authorization")
        .body(Body::empty())
        .unwrap();
    let resp = t.router.clone().oneshot(req).await.unwrap();
    let h = resp.headers();
    assert_eq!(h["access-control-allow-origin"], "*");
    let allowed = h["access-control-allow-headers"].to_str().unwrap().to_ascii_lowercase();
    assert!(allowed.contains("authorization") && !allowed.contains('*'), "{allowed}");
}
