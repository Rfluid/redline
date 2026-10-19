//! `SQLite` persistence via Diesel. Every write appends one event in the same transaction;
//! readers follow the event `seq`.

mod rows;
mod schema;
mod writes;

use crate::ids::{new_id, new_token, now_ms};
use crate::model::{Artifact, Event, RegisterArtifact, Submit, Thread};
use anyhow::{Context, Result, anyhow, bail};
use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use rows::{ArtifactRow, EventRow, MessageRow, ThreadRow};
use schema::{artifacts, attachments, events, messages, threads};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;
use writes::{append_event, check_status, insert_reply, insert_thread, set_thread_status};

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

/// Max events returned by one [`Store::events`] call.
pub const EVENT_PAGE: i64 = 100;

/// Which events to read. `None` fields don't filter.
#[derive(Default)]
pub struct EventFilter<'a> {
    pub since: i64,
    pub artifact: Option<&'a str>,
    pub session: Option<&'a str>,
    pub author: Option<&'a str>,
}

pub struct Store {
    conn: SqliteConnection,
}

impl Store {
    /// Opens (or creates) the database at `path` and runs pending migrations.
    /// `":memory:"` gives a throwaway database.
    pub fn open(path: &Path) -> Result<Self> {
        let url = path.to_str().context("non-UTF-8 state path")?;
        let mut conn = SqliteConnection::establish(url)?;
        conn.batch_execute("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        conn.run_pending_migrations(MIGRATIONS).map_err(|e| anyhow!("migrations: {e}"))?;
        Ok(Self { conn })
    }

    /// Registers a new artifact, or re-attaches an existing one (by `req.id`) to `req.session`.
    pub fn register(&mut self, req: &RegisterArtifact) -> Result<Artifact> {
        if let Some(id) = &req.id
            && self.artifact(id)?.is_some()
        {
            diesel::update(artifacts::table.find(id))
                .set((artifacts::session.eq(&req.session), artifacts::title.eq(&req.title)))
                .execute(&mut self.conn)?;
            if let Some(uri) = &req.uri {
                diesel::update(artifacts::table.find(id)).set(artifacts::uri.eq(uri)).execute(&mut self.conn)?;
            }
            return self.artifact(id)?.context("artifact vanished during update");
        }
        let row = ArtifactRow {
            id: req.id.clone().unwrap_or_else(|| new_id("art")),
            title: req.title.clone(),
            uri: req.uri.clone(),
            kind: req.kind.clone().unwrap_or_else(|| "html".into()),
            session: req.session.clone(),
            token: new_token(),
            created: now_ms(),
        };
        diesel::insert_into(artifacts::table).values(&row).execute(&mut self.conn)?;
        Ok(row.into())
    }

    pub fn artifact(&mut self, id: &str) -> Result<Option<Artifact>> {
        let row = artifacts::table.find(id).select(ArtifactRow::as_select()).first(&mut self.conn).optional()?;
        Ok(row.map(Into::into))
    }

    pub fn artifacts(&mut self) -> Result<Vec<Artifact>> {
        let rows =
            artifacts::table.order(artifacts::created.desc()).select(ArtifactRow::as_select()).load(&mut self.conn)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub fn thread(&mut self, id: &str) -> Result<Option<Thread>> {
        let row = threads::table.find(id).select(ThreadRow::as_select()).first(&mut self.conn).optional()?;
        row.map(|r| self.hydrate(r)).transpose()
    }

    pub fn threads(&mut self, artifact_id: &str) -> Result<Vec<Thread>> {
        let rows = threads::table
            .filter(threads::artifact_id.eq(artifact_id))
            .order(threads::created)
            .select(ThreadRow::as_select())
            .load(&mut self.conn)?;
        rows.into_iter().map(|r| self.hydrate(r)).collect()
    }

    /// New thread with its first message. Status starts `open`.
    pub fn create_thread(&mut self, artifact_id: &str, anchor: &Value, author: &str, body: &str) -> Result<Event> {
        let seq = self.conn.transaction(|c| insert_thread(c, artifact_id, anchor, author, body, None))?;
        self.event(seq)
    }

    /// Adds a message. A user message reopens the thread; an agent reply defaults to `answered`.
    pub fn add_message(&mut self, thread_id: &str, author: &str, body: &str, status: Option<&str>) -> Result<Event> {
        let seq = self.conn.transaction(|c| insert_reply(c, None, thread_id, author, body, status))?;
        self.event(seq)
    }

    /// New threads and replies on one artifact, all or nothing: one transaction, one event each.
    /// A reply to a thread on another artifact fails the whole batch.
    pub fn submit(&mut self, artifact_id: &str, author: &str, req: &Submit) -> Result<Vec<Event>> {
        let seqs = self.conn.transaction(|c| -> Result<Vec<i64>> {
            let mut seqs = Vec::with_capacity(req.threads.len() + req.replies.len());
            for t in &req.threads {
                seqs.push(insert_thread(c, artifact_id, &t.anchor, author, &t.body, t.image.as_ref())?);
            }
            for r in &req.replies {
                seqs.push(insert_reply(c, Some(artifact_id), &r.thread_id, author, &r.body, r.status.as_deref())?);
            }
            Ok(seqs)
        })?;
        seqs.into_iter().map(|seq| self.event(seq)).collect()
    }

    /// Sets the status and the progress note (`None` clears it).
    pub fn set_status(&mut self, thread_id: &str, status: &str, author: &str, note: Option<&str>) -> Result<Event> {
        check_status(status)?;
        let Some(t) = self.thread(thread_id)? else { bail!("no thread {thread_id}") };
        let seq = self.conn.transaction(|c| -> Result<i64> {
            set_thread_status(c, thread_id, status, note)?;
            append_event(c, &t.artifact_id, thread_id, "status", author)
        })?;
        self.event(seq)
    }

    /// Events after `f.since`, oldest first, at most [`EVENT_PAGE`].
    pub fn events(&mut self, f: &EventFilter) -> Result<Vec<Event>> {
        let mut q = events::table
            .inner_join(artifacts::table)
            .filter(events::seq.gt(f.since))
            .select(EventRow::as_select())
            .order(events::seq)
            .limit(EVENT_PAGE)
            .into_boxed();
        if let Some(a) = f.artifact {
            q = q.filter(events::artifact_id.eq(a));
        }
        if let Some(s) = f.session {
            q = q.filter(artifacts::session.eq(s));
        }
        if let Some(a) = f.author {
            q = q.filter(events::author.eq(a));
        }
        let rows = q.load(&mut self.conn)?;
        rows.into_iter().map(|r| self.load_event(r)).collect()
    }

    /// Row counts for `redline status`: `(artifacts, threads per status, events)`.
    pub fn counts(&mut self) -> Result<(i64, BTreeMap<String, i64>, i64)> {
        let n_artifacts = artifacts::table.count().get_result(&mut self.conn)?;
        let n_events = events::table.count().get_result(&mut self.conn)?;
        let by_status: Vec<(String, i64)> = threads::table
            .group_by(threads::status)
            .select((threads::status, diesel::dsl::count_star()))
            .load(&mut self.conn)?;
        Ok((n_artifacts, by_status.into_iter().collect(), n_events))
    }

    /// An attachment: `(artifact_id, mime, bytes)`.
    pub fn attachment(&mut self, id: &str) -> Result<Option<(String, String, Vec<u8>)>> {
        Ok(attachments::table
            .find(id)
            .select((attachments::artifact_id, attachments::mime, attachments::data))
            .first(&mut self.conn)
            .optional()?)
    }

    /// Latest event `seq` (0 when empty): a cursor meaning "from now on".
    pub fn head(&mut self) -> Result<i64> {
        let max: Option<i64> = events::table.select(diesel::dsl::max(events::seq)).first(&mut self.conn)?;
        Ok(max.unwrap_or(0))
    }

    fn event(&mut self, seq: i64) -> Result<Event> {
        let row = events::table.find(seq).select(EventRow::as_select()).first(&mut self.conn)?;
        self.load_event(row)
    }

    fn load_event(&mut self, r: EventRow) -> Result<Event> {
        let thread = self.thread(&r.thread_id)?.context("event for missing thread")?;
        Ok(Event { seq: r.seq, artifact_id: r.artifact_id, kind: r.kind, author: r.author, at: r.at, thread })
    }

    fn hydrate(&mut self, r: ThreadRow) -> Result<Thread> {
        let msgs = messages::table
            .filter(messages::thread_id.eq(&r.id))
            .order(messages::seq)
            .select(MessageRow::as_select())
            .load(&mut self.conn)?;
        Ok(Thread {
            id: r.id,
            artifact_id: r.artifact_id,
            anchor: serde_json::from_str(&r.anchor).unwrap_or(Value::Null),
            status: r.status,
            created: r.created,
            updated: r.updated,
            note: r.note,
            messages: msgs.into_iter().map(Into::into).collect(),
        })
    }
}

#[cfg(test)]
mod tests;
