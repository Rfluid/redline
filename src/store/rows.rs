//! Diesel row types and their conversions to wire types.

use super::schema::{artifacts, events, messages, threads};
use crate::model::{Artifact, Message};
use diesel::prelude::*;

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = artifacts, check_for_backend(diesel::sqlite::Sqlite))]
pub(super) struct ArtifactRow {
    pub id: String,
    pub title: String,
    pub uri: Option<String>,
    pub kind: String,
    pub session: Option<String>,
    pub token: String,
    pub created: i64,
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = threads, check_for_backend(diesel::sqlite::Sqlite))]
pub(super) struct ThreadRow {
    pub id: String,
    pub artifact_id: String,
    pub anchor: String,
    pub status: String,
    pub created: i64,
    pub updated: i64,
    pub note: Option<String>,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = messages, check_for_backend(diesel::sqlite::Sqlite))]
pub(super) struct MessageRow {
    pub id: String,
    pub thread_id: String,
    pub author: String,
    pub body: String,
    pub created: i64,
}

#[derive(Insertable)]
#[diesel(table_name = messages)]
pub(super) struct NewMessage<'a> {
    pub id: String,
    pub thread_id: &'a str,
    pub author: &'a str,
    pub body: &'a str,
    pub created: i64,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = events, check_for_backend(diesel::sqlite::Sqlite))]
pub(super) struct EventRow {
    pub seq: i64,
    pub artifact_id: String,
    pub thread_id: String,
    pub kind: String,
    pub author: String,
    pub at: i64,
}

#[derive(Insertable)]
#[diesel(table_name = events)]
pub(super) struct NewEvent<'a> {
    pub artifact_id: &'a str,
    pub thread_id: &'a str,
    pub kind: &'a str,
    pub author: &'a str,
    pub at: i64,
}

impl From<ArtifactRow> for Artifact {
    fn from(r: ArtifactRow) -> Self {
        Artifact {
            id: r.id,
            title: r.title,
            uri: r.uri,
            kind: r.kind,
            session: r.session,
            token: Some(r.token),
            created: r.created,
        }
    }
}

impl From<MessageRow> for Message {
    fn from(r: MessageRow) -> Self {
        Message { id: r.id, thread_id: r.thread_id, author: r.author, body: r.body, created: r.created }
    }
}
