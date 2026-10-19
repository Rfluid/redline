//! Write helpers that run inside a `Store` transaction. Each public `Store` write calls
//! exactly one `append_event`.

use super::rows::{NewEvent, NewMessage, ThreadRow};
use super::schema::{attachments, events, messages, threads};
use crate::ids::{new_id, now_ms};
use crate::model::{Image, STATUSES};
use anyhow::{Context, Result, bail};
use base64::Engine;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use serde_json::Value;

/// New thread with its first message. An image (a region screenshot) is stored as an attachment
/// and referenced from the anchor as `image: "att_…"`.
pub(super) fn insert_thread(
    c: &mut SqliteConnection,
    artifact_id: &str,
    anchor: &Value,
    author: &str,
    body: &str,
    image: Option<&Image>,
) -> Result<i64> {
    let mut anchor = anchor.clone();
    if let Some(img) = image {
        let id = insert_attachment(c, artifact_id, img)?;
        anchor
            .as_object_mut()
            .context("an anchor with an image must be a JSON object")?
            .insert("image".into(), id.into());
    }
    let t = now_ms();
    let row = ThreadRow {
        id: new_id("thr"),
        artifact_id: artifact_id.into(),
        anchor: anchor.to_string(),
        status: "open".into(),
        created: t,
        updated: t,
        note: None,
    };
    diesel::insert_into(threads::table).values(&row).execute(c)?;
    insert_message(c, &row.id, author, body)?;
    append_event(c, artifact_id, &row.id, "message", author)
}

fn insert_attachment(c: &mut SqliteConnection, artifact_id: &str, img: &Image) -> Result<String> {
    let data = base64::engine::general_purpose::STANDARD.decode(&img.data).context("image data isn't base64")?;
    let id = new_id("att");
    diesel::insert_into(attachments::table)
        .values((
            attachments::id.eq(&id),
            attachments::artifact_id.eq(artifact_id),
            attachments::mime.eq(&img.mime),
            attachments::data.eq(&data),
            attachments::created.eq(now_ms()),
        ))
        .execute(c)?;
    Ok(id)
}

/// Message on an existing thread (optionally required to belong to `artifact`), plus its status change.
pub(super) fn insert_reply(
    c: &mut SqliteConnection,
    artifact: Option<&str>,
    thread_id: &str,
    author: &str,
    body: &str,
    status: Option<&str>,
) -> Result<i64> {
    let owner: Option<String> = threads::table.find(thread_id).select(threads::artifact_id).first(c).optional()?;
    let Some(owner) = owner.filter(|o| artifact.is_none_or(|a| a == o)) else { bail!("no thread {thread_id}") };
    let status = status.unwrap_or(if author == "agent" { "answered" } else { "open" });
    check_status(status)?;
    insert_message(c, thread_id, author, body)?;
    set_thread_status(c, thread_id, status, None)?;
    append_event(c, &owner, thread_id, "message", author)
}

pub(super) fn insert_message(c: &mut SqliteConnection, thread_id: &str, author: &str, body: &str) -> Result<()> {
    let m = NewMessage { id: new_id("msg"), thread_id, author, body, created: now_ms() };
    diesel::insert_into(messages::table).values(&m).execute(c)?;
    Ok(())
}

pub(super) fn set_thread_status(
    c: &mut SqliteConnection,
    thread_id: &str,
    status: &str,
    note: Option<&str>,
) -> Result<()> {
    diesel::update(threads::table.find(thread_id))
        .set((threads::status.eq(status), threads::note.eq(note), threads::updated.eq(now_ms())))
        .execute(c)?;
    Ok(())
}

/// Appends to the event log; returns the new `seq`.
pub(super) fn append_event(
    c: &mut SqliteConnection,
    artifact_id: &str,
    thread_id: &str,
    kind: &str,
    author: &str,
) -> Result<i64> {
    let ev = NewEvent { artifact_id, thread_id, kind, author, at: now_ms() };
    Ok(diesel::insert_into(events::table).values(&ev).returning(events::seq).get_result(c)?)
}

pub(super) fn check_status(s: &str) -> Result<()> {
    if !STATUSES.contains(&s) {
        bail!("bad status {s:?}; expected one of {STATUSES:?}");
    }
    Ok(())
}
