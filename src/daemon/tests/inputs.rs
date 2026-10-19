//! Input limits and region screenshots (attachments).

use super::{ADMIN, T};
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use tower::ServiceExt;

/// 1×1 PNG.
const PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

#[tokio::test]
async fn input_limits_give_clear_errors() {
    use crate::daemon::limits::{MAX_ANCHOR_BYTES, MAX_MESSAGE_BYTES, MAX_REQUEST_BYTES, MAX_SUBMIT_ITEMS};
    let t = T::new();
    let (a, tok) = t.artifact("s").await;
    let submit = format!("/api/artifacts/{a}/submit");
    let one = |anchor: Value, body: String| json!({"threads": [{"anchor": anchor, "body": body}]});

    let cases = [
        (one(json!({}), "x".repeat(MAX_MESSAGE_BYTES + 1)), StatusCode::PAYLOAD_TOO_LARGE, "message is"),
        (one(json!({"quote": "q".repeat(MAX_ANCHOR_BYTES)}), "x".into()), StatusCode::PAYLOAD_TOO_LARGE, "anchor is"),
        (one(json!({}), "  ".into()), StatusCode::BAD_REQUEST, "empty message"),
        (json!({"threads": [], "replies": []}), StatusCode::BAD_REQUEST, "nothing to submit"),
        (
            json!({"threads": vec![json!({"anchor": {}, "body": "x"}); MAX_SUBMIT_ITEMS + 1]}),
            StatusCode::BAD_REQUEST,
            "submit item count",
        ),
        (json!({"threads": "nope"}), StatusCode::UNPROCESSABLE_ENTITY, "threads"),
    ];
    for (body, want, msg) in cases {
        let (st, v) = t.post(&submit, Some(&tok), body).await;
        assert_eq!(st, want, "{v}");
        assert!(v["error"].as_str().is_some_and(|e| e.contains(msg)), "{msg}: {v}");
    }

    let huge = json!({"threads": [{"anchor": {}, "body": "x".repeat(MAX_REQUEST_BYTES)}]});
    let (st, v) = t.post(&submit, Some(&tok), huge).await;
    assert_eq!(st, StatusCode::PAYLOAD_TOO_LARGE);
    assert!(v["error"].as_str().is_some_and(|e| e.contains("request body over")), "{v}");

    let long_title = json!({"title": "t".repeat(crate::daemon::limits::MAX_FIELD_BYTES + 1)});
    assert_eq!(t.post("/api/artifacts", Some(ADMIN), long_title).await.0, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(t.get(&format!("/api/artifacts/{a}/threads"), Some(&tok)).await.1, json!([]), "nothing written");
}

#[tokio::test]
async fn region_screenshot_is_stored_and_scoped() {
    let t = T::new();
    let (a, tok_a) = t.artifact("s").await;
    let (_b, tok_b) = t.artifact("s").await;
    let submit = format!("/api/artifacts/{a}/submit");
    let thread = |image: Value| json!({"threads": [{"anchor": {"kind": "region", "x": 0.5, "y": 0.5}, "body": "this", "image": image}]});

    let (st, evs) = t.post(&submit, Some(&tok_a), thread(json!({"mime": "image/png", "data": PNG}))).await;
    assert_eq!(st, StatusCode::OK, "{evs}");
    let att = evs[0]["thread"]["anchor"]["image"].as_str().unwrap().to_string();
    assert!(att.starts_with("att_"));
    assert_eq!(evs[0]["thread"]["anchor"]["x"], 0.5, "the rest of the anchor is kept");

    let fetch = |tok: Option<String>| {
        let r = t.router.clone();
        let att = att.clone();
        async move {
            let mut req = Request::get(format!("/api/attachments/{att}"));
            if let Some(tok) = tok {
                req = req.header("authorization", format!("Bearer {tok}"));
            }
            let resp = r.oneshot(req.body(Body::empty()).unwrap()).await.unwrap();
            let ct = resp.headers().get("content-type").map(|v| v.to_str().unwrap().to_string());
            (resp.status(), ct, to_bytes(resp.into_body(), usize::MAX).await.unwrap().len())
        }
    };
    let (st, ct, len) = fetch(Some(tok_a.clone())).await;
    assert_eq!((st, ct.as_deref()), (StatusCode::OK, Some("image/png")));
    assert!(len > 60);
    assert_eq!(fetch(Some(ADMIN.into())).await.0, StatusCode::OK);
    assert_eq!(fetch(Some(tok_b)).await.0, StatusCode::UNAUTHORIZED, "another artifact's token");
    assert_eq!(fetch(None).await.0, StatusCode::UNAUTHORIZED);

    let bad = [
        (json!({"mime": "text/html", "data": PNG}), StatusCode::BAD_REQUEST),
        (
            json!({"mime": "image/png", "data": "A".repeat(crate::daemon::limits::MAX_IMAGE_BYTES * 2)}),
            StatusCode::PAYLOAD_TOO_LARGE,
        ),
        (json!({"mime": "image/png", "data": "not base64!"}), StatusCode::BAD_REQUEST),
    ];
    for (image, want) in bad {
        assert_eq!(t.post(&submit, Some(&tok_a), thread(image)).await.0, want);
    }
    let not_object = json!({"threads": [{"anchor": "x", "body": "b", "image": {"mime": "image/png", "data": PNG}}]});
    assert_eq!(t.post(&submit, Some(&tok_a), not_object).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(t.get(&format!("/api/artifacts/{a}/threads"), Some(&tok_a)).await.1.as_array().unwrap().len(), 1);
}
