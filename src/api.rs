//! Agent-side HTTP client. Starts the daemon on demand.

use crate::model::{Artifact, DaemonStatus, Event, RegisterArtifact, StatusChange, Thread};
use crate::{lifecycle, paths};
use anyhow::{Context, Result, anyhow};
use serde::Serialize;
use serde_json::Value;
use std::fmt::Write as _;
use std::io::Read as _;
use std::time::{Duration, Instant};

/// Upper bound when reading an attachment body (the daemon caps images well below this).
const ATTACHMENT_READ_LIMIT: u64 = 4 * 1024 * 1024;

/// Reconnect attempts while waiting, and the pause before each (a restart takes well under a second).
const RECONNECT_TRIES: u32 = 5;
const RECONNECT_DELAY: Duration = Duration::from_millis(500);

pub struct Api {
    pub port: u16,
    token: String,
}

impl Api {
    pub fn connect() -> Result<Self> {
        Ok(Self { port: lifecycle::ensure()?, token: paths::admin_token()? })
    }

    pub fn base(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    pub fn get(&self, path: &str, timeout: Duration) -> Result<Value> {
        let req = ureq::get(&format!("{}{path}", self.base())).timeout(timeout);
        decode(req.set("Authorization", &format!("Bearer {}", self.token)).call())
    }

    pub fn post(&self, path: &str, body: &impl Serialize) -> Result<Value> {
        let req = ureq::post(&format!("{}{path}", self.base())).timeout(Duration::from_secs(10));
        decode(req.set("Authorization", &format!("Bearer {}", self.token)).send_json(body))
    }

    /// An attachment's `(mime, bytes)`.
    pub fn attachment(&self, id: &str) -> Result<(String, Vec<u8>)> {
        let url = format!("{}/api/attachments/{}", self.base(), enc(id));
        let resp = ureq::get(&url)
            .timeout(Duration::from_secs(10))
            .set("Authorization", &format!("Bearer {}", self.token))
            .call()
            .map_err(call_error)?;
        let mime = resp.content_type().to_string();
        let mut data = Vec::new();
        resp.into_reader().take(ATTACHMENT_READ_LIMIT).read_to_end(&mut data).context("reading attachment")?;
        Ok((mime, data))
    }

    pub fn register(&self, req: &RegisterArtifact) -> Result<Artifact> {
        Ok(serde_json::from_value(self.post("/api/artifacts", req)?)?)
    }

    /// Long-polls for events. Returns (events, next cursor).
    pub fn wait(
        &self,
        since: i64,
        timeout_s: u64,
        artifact: Option<&str>,
        session: Option<&str>,
    ) -> Result<(Vec<Event>, i64)> {
        let mut q = format!("/api/events?since={since}&timeout={timeout_s}&author=user");
        if let Some(a) = artifact {
            let _ = write!(q, "&artifact={}", enc(a));
        }
        if let Some(s) = session {
            let _ = write!(q, "&session={}", enc(s));
        }
        let v = self.get(&q, Duration::from_secs(timeout_s + 15))?;
        Ok((serde_json::from_value(v["events"].clone())?, v["next"].as_i64().unwrap_or(since)))
    }

    /// [`Api::wait`] that rides through a daemon restart (an update replaces the daemon mid-wait):
    /// it reconnects, starting a daemon if needed, and keeps waiting until the deadline. The `since`
    /// cursor means nothing is missed.
    pub fn wait_through_restarts(
        mut self,
        since: i64,
        timeout_s: u64,
        artifact: Option<&str>,
        session: Option<&str>,
    ) -> Result<(Vec<Event>, i64)> {
        let deadline = Instant::now() + Duration::from_secs(timeout_s);
        let mut tries = 0;
        loop {
            let left = deadline.saturating_duration_since(Instant::now()).as_secs();
            match self.wait(since, left, artifact, session) {
                Err(e) if unreachable(&e) && tries < RECONNECT_TRIES && left > 0 => {
                    tries += 1;
                    std::thread::sleep(RECONNECT_DELAY);
                    self = Api::connect()?;
                }
                r => return r,
            }
        }
    }

    pub fn threads(&self, artifact: &str) -> Result<Vec<Thread>> {
        Ok(serde_json::from_value(
            self.get(&format!("/api/artifacts/{}/threads", enc(artifact)), Duration::from_secs(10))?,
        )?)
    }

    pub fn reply(&self, thread: &str, body: &str, status: Option<&str>) -> Result<Event> {
        let v = self.post(
            &format!("/api/threads/{}/messages", enc(thread)),
            &serde_json::json!({ "body": body, "status": status }),
        )?;
        Ok(serde_json::from_value(v)?)
    }

    pub fn status(&self) -> Result<DaemonStatus> {
        Ok(serde_json::from_value(self.get("/api/status", Duration::from_secs(10))?)?)
    }

    pub fn set_status(&self, thread: &str, status: &str, note: Option<&str>) -> Result<Event> {
        let body = StatusChange { status: status.into(), note: note.map(Into::into) };
        let v = self.post(&format!("/api/threads/{}/status", enc(thread)), &body)?;
        Ok(serde_json::from_value(v)?)
    }

    /// What the agent pastes into the artifact so the page can talk to redline.
    pub fn embed(&self, a: &Artifact) -> String {
        format!(
            "{begin} (review only: remove with `redline strip`) -->\n<script src=\"{b}/client.js\" data-redline-artifact=\"{id}\" data-redline-token=\"{tok}\"></script>\n<script src=\"{b}/overlay.js\"></script>\n{end}",
            begin = crate::strip::BEGIN,
            end = crate::strip::END,
            b = self.base(),
            id = a.id,
            tok = a.token.as_deref().unwrap_or("")
        )
    }
}

fn decode(r: Result<ureq::Response, ureq::Error>) -> Result<Value> {
    Ok(r.map_err(call_error)?.into_json()?)
}

/// The daemon went away (as opposed to answering with an error).
fn unreachable(e: &anyhow::Error) -> bool {
    matches!(e.downcast_ref::<ureq::Error>(), Some(ureq::Error::Transport(_)))
}

/// A failed call as an error the agent can act on: the daemon's `{error}` message, or "unreachable".
fn call_error(e: ureq::Error) -> anyhow::Error {
    match e {
        ureq::Error::Status(code, resp) => {
            let body: Value = resp.into_json().unwrap_or(Value::Null);
            anyhow!("daemon {code}: {}", body["error"].as_str().unwrap_or("error"))
        }
        e @ ureq::Error::Transport(_) => anyhow!(e).context("daemon unreachable"),
    }
}

fn enc(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}
