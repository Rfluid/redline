use super::*;
use serde_json::json;

fn store() -> Store {
    Store::open(Path::new(":memory:")).unwrap()
}

fn artifact(s: &mut Store, session: &str) -> Artifact {
    s.register(&RegisterArtifact { title: "t".into(), session: Some(session.into()), ..Default::default() }).unwrap()
}

#[test]
fn register_then_reattach_keeps_id_and_token() {
    let mut s = store();
    let a = artifact(&mut s, "ses_a");
    let b = s
        .register(&RegisterArtifact {
            id: Some(a.id.clone()),
            title: "renamed".into(),
            session: Some("ses_b".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(b.id, a.id);
    assert_eq!(b.token, a.token);
    assert_eq!(b.title, "renamed");
    assert_eq!(b.session.as_deref(), Some("ses_b"));
}

#[test]
fn status_rules() {
    let mut s = store();
    let a = artifact(&mut s, "ses");
    let ev = s.create_thread(&a.id, &json!({"kind": "text", "quote": "x"}), "user", "fix").unwrap();
    let t = ev.thread.id;
    assert_eq!(ev.thread.status, "open");

    assert_eq!(s.add_message(&t, "agent", "done", None).unwrap().thread.status, "answered");
    assert_eq!(s.add_message(&t, "user", "not quite", None).unwrap().thread.status, "open");
    assert_eq!(s.add_message(&t, "agent", "which part?", Some("open")).unwrap().thread.status, "open");
    assert_eq!(s.set_status(&t, "resolved", "user", None).unwrap().thread.status, "resolved");
    assert!(s.set_status(&t, "bogus", "user", None).is_err());

    let msgs = s.thread(&t).unwrap().unwrap().messages;
    let authors: Vec<_> = msgs.iter().map(|m| m.author.as_str()).collect();
    assert_eq!(authors, ["user", "agent", "user", "agent"]);
}

#[test]
fn event_filters() {
    let mut s = store();
    let a = artifact(&mut s, "ses_a");
    let b = artifact(&mut s, "ses_b");
    let ta = s.create_thread(&a.id, &json!({}), "user", "a1").unwrap().thread.id;
    s.create_thread(&b.id, &json!({}), "user", "b1").unwrap();
    s.add_message(&ta, "agent", "reply", None).unwrap();

    let all = s.events(&EventFilter::default()).unwrap();
    assert_eq!(all.len(), 3);
    assert!(all.windows(2).all(|w| w[0].seq < w[1].seq));

    let ses_a = s.events(&EventFilter { session: Some("ses_a"), ..Default::default() }).unwrap();
    assert_eq!(ses_a.len(), 2);

    let user_a = s.events(&EventFilter { session: Some("ses_a"), author: Some("user"), ..Default::default() }).unwrap();
    assert_eq!(user_a.len(), 1);

    let after = s.events(&EventFilter { since: all[1].seq, ..Default::default() }).unwrap();
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].author, "agent");
}

#[test]
fn unknown_thread_is_an_error() {
    let mut s = store();
    assert!(s.add_message("thr_nope", "user", "x", None).is_err());
    assert!(s.thread("thr_nope").unwrap().is_none());
}

#[test]
fn counts_by_status() {
    let mut s = store();
    let a = artifact(&mut s, "ses");
    let t = s.create_thread(&a.id, &json!({}), "user", "x").unwrap().thread.id;
    s.create_thread(&a.id, &json!({}), "user", "y").unwrap();
    s.set_status(&t, "working", "agent", None).unwrap();
    let (arts, by_status, events) = s.counts().unwrap();
    assert_eq!(arts, 1);
    assert_eq!(by_status.get("open"), Some(&1));
    assert_eq!(by_status.get("working"), Some(&1));
    assert_eq!(events, 3);
    assert_eq!(s.head().unwrap(), 3);
    assert_eq!(store().head().unwrap(), 0);
}

#[test]
fn progress_note_lives_until_the_next_change() {
    let mut s = store();
    let a = artifact(&mut s, "ses");
    let t = s.create_thread(&a.id, &json!({}), "user", "x").unwrap().thread.id;
    let ev = s.set_status(&t, "working", "agent", Some("Rebuilding the chart")).unwrap();
    assert_eq!(ev.thread.note.as_deref(), Some("Rebuilding the chart"));
    assert_eq!(s.add_message(&t, "agent", "done", None).unwrap().thread.note, None, "a reply clears it");
    s.set_status(&t, "working", "agent", Some("again")).unwrap();
    assert_eq!(s.set_status(&t, "open", "user", None).unwrap().thread.note, None);
}
