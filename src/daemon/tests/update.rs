//! Update notices over HTTP: read with the artifact's token, dismiss one release or all of them.

use super::*;
use crate::update::{config, release};

#[tokio::test]
async fn overlay_reads_and_dismisses_update_notices() {
    let t = T::new();
    let (art, tok) = t.artifact("ses_u").await;
    let (other, _) = t.artifact("ses_u").await;
    let cache = release::Cache { checked: 1, latest: Some("99.0.0".into()), error: None };
    release::write(&t.dir.path().join("update.json"), &cache).unwrap();
    let path = format!("/api/artifacts/{art}/update");

    let (st, u) = t.get(&path, Some(&tok)).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!((u["latest"].as_str(), u["notify"].as_bool()), (Some("99.0.0"), Some(true)));
    assert_eq!(t.get(&path, None).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(t.get(&format!("/api/artifacts/{other}/update"), Some(&tok)).await.0, StatusCode::UNAUTHORIZED);

    let dismiss = format!("{path}/dismiss");
    assert_eq!(t.post(&dismiss, Some(&tok), json!({})).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(t.post(&dismiss, Some(&tok), json!({"version": "../../x"})).await.0, StatusCode::BAD_REQUEST);
    let (st, u) = t.post(&dismiss, Some(&tok), json!({"version": "99.0.0"})).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!((u["skipped"].as_bool(), u["notify"].as_bool()), (Some(true), Some(false)));

    let (_, u) = t.post(&dismiss, Some(ADMIN), json!({"all": true})).await;
    assert_eq!(u["check"].as_bool(), Some(false));
    let cfg = config::load(&t.dir.path().join("config.toml")).unwrap();
    assert_eq!((cfg.check, cfg.skip), (false, vec!["99.0.0".to_string()]));
}
