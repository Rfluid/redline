//! Agent presence: is an agent listening for an artifact's comments right now?
//!
//! A listener is an admin long-poll on `/api/events` (`redline wait`, `wait_for_comments`, the
//! channel push loop). Presence is ephemeral: it lives in memory, is pushed to the artifact's
//! `WebSocket`s as a `presence` frame, and never touches the event log (it is not a write).

use super::App;
use std::collections::BTreeMap;
use std::sync::PoisonError;
use std::time::{Duration, Instant};

/// How long an artifact still counts as listened to after its last long-poll ended. Agents poll in
/// a loop but work between polls (read the comment, edit, reply: often tens of seconds), and an
/// MCP `wait_for_comments` returns every 120 s by default. A minute bridges those gaps without
/// claiming for long that a session that went away is still there.
pub(super) const LISTEN_GRACE: Duration = Duration::from_secs(60);

/// Long-polls in flight and when each filter's last one ended. Keys are the poll's filter:
/// `artifact:<id>`, `session:<id>` or `all`.
#[derive(Default)]
pub(super) struct Listeners {
    open: BTreeMap<String, usize>,
    ended: BTreeMap<String, Instant>,
}

/// Whether an agent listens to one artifact, and until when that holds if nothing else happens
/// (`None`: a poll is in flight, so until it ends; or nobody listens).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct Presence {
    pub listening: bool,
    pub until: Option<Instant>,
}

impl Listeners {
    pub(super) fn start(&mut self, key: &str) {
        *self.open.entry(key.to_owned()).or_default() += 1;
    }

    pub(super) fn end(&mut self, key: &str, now: Instant) {
        if let Some(n) = self.open.get_mut(key) {
            *n -= 1;
            if *n == 0 {
                self.open.remove(key);
            }
        }
        self.ended.retain(|_, t| now.duration_since(*t) < LISTEN_GRACE);
        self.ended.insert(key.to_owned(), now);
    }

    /// Long-polls in flight, by filter (for `redline status`).
    pub(super) fn open(&self) -> BTreeMap<String, usize> {
        self.open.clone()
    }

    /// An artifact is listened to when a poll whose filter covers it (that artifact, its session, or
    /// no filter) is in flight, or ended less than [`LISTEN_GRACE`] ago.
    pub(super) fn presence(&self, artifact: &str, session: Option<&str>, now: Instant) -> Presence {
        let keys =
            [Some("all".to_owned()), Some(format!("artifact:{artifact}")), session.map(|s| format!("session:{s}"))];
        let keys = keys.iter().flatten();
        if keys.clone().any(|k| self.open.contains_key(k)) {
            return Presence { listening: true, until: None };
        }
        let until = keys.filter_map(|k| self.ended.get(k)).map(|t| *t + LISTEN_GRACE).max().filter(|u| *u > now);
        Presence { listening: until.is_some(), until }
    }
}

/// Counts one long-poll for its lifetime; dropping it (return or client gone) ends it.
pub(super) struct Waiting<'a> {
    app: &'a App,
    key: String,
}

impl Drop for Waiting<'_> {
    fn drop(&mut self) {
        self.app.listeners().end(&self.key, Instant::now());
        self.app.presence_tx.send_replace(());
    }
}

impl App {
    fn listeners(&self) -> std::sync::MutexGuard<'_, Listeners> {
        self.listeners.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(super) fn wait_guard(&self, key: String) -> Waiting<'_> {
        self.listeners().start(&key);
        self.presence_tx.send_replace(());
        Waiting { app: self, key }
    }

    pub(super) fn waiting(&self) -> BTreeMap<String, usize> {
        self.listeners().open()
    }

    /// Presence for one artifact now (its session read from the store: re-registering moves it).
    pub(super) fn presence(&self, artifact: &str) -> Presence {
        let session = self.store().artifact(artifact).ok().flatten().and_then(|a| a.session);
        self.listeners().presence(artifact, session.as_deref(), Instant::now())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ART: &str = "art_1";
    const SES: Option<&str> = Some("ses_1");

    #[test]
    fn a_matching_poll_in_flight_listens() {
        let now = Instant::now();
        for key in ["all", "artifact:art_1", "session:ses_1"] {
            let mut l = Listeners::default();
            l.start(key);
            assert_eq!(l.presence(ART, SES, now), Presence { listening: true, until: None }, "{key}");
        }
    }

    #[test]
    fn other_filters_dont_count() {
        let now = Instant::now();
        let mut l = Listeners::default();
        l.start("artifact:art_2");
        l.start("session:ses_2");
        assert!(!l.presence(ART, SES, now).listening);
        l.start("session:ses_1");
        assert!(!l.presence(ART, None, now).listening, "an artifact without a session has no session listener");
        assert_eq!(l.presence(ART, None, now).until, None);
    }

    #[test]
    fn listening_lasts_the_grace_window_after_the_last_poll() {
        let t0 = Instant::now();
        let mut l = Listeners::default();
        l.start("session:ses_1");
        l.start("session:ses_1");
        l.end("session:ses_1", t0);
        assert_eq!(l.presence(ART, SES, t0).until, None, "one poll still in flight");
        l.end("session:ses_1", t0);
        assert!(l.open().is_empty());
        let p = l.presence(ART, SES, t0 + Duration::from_secs(1));
        assert_eq!(p, Presence { listening: true, until: Some(t0 + LISTEN_GRACE) });
        let later = t0 + LISTEN_GRACE;
        assert_eq!(l.presence(ART, SES, later), Presence { listening: false, until: None });
    }

    #[test]
    fn the_latest_end_wins_and_old_ends_are_dropped() {
        let t0 = Instant::now();
        let mut l = Listeners::default();
        l.start("all");
        l.end("all", t0);
        l.start("artifact:art_1");
        l.end("artifact:art_1", t0 + Duration::from_secs(30));
        assert_eq!(
            l.presence(ART, SES, t0 + Duration::from_secs(31)).until,
            Some(t0 + Duration::from_secs(30) + LISTEN_GRACE)
        );
        l.start("session:x");
        l.end("session:x", t0 + LISTEN_GRACE * 3);
        assert_eq!(l.ended.len(), 1, "ends older than the grace window are forgotten");
    }
}
