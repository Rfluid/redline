//! The newest release: the highest `vX.Y.Z` tag on the clone's remote (pre-releases are ignored).
//! Checked at most once a day; the result is cached in `update.json` in the state dir.

use super::git::git;
use crate::ids::now_ms;
use crate::lifecycle::version_cmp;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::Duration;

/// How often to look for a release, and how soon to retry after a failed check (offline, no auth).
const CHECK_EVERY_MS: i64 = 24 * 60 * 60 * 1000;
const RETRY_EVERY_MS: i64 = 60 * 60 * 1000;
const LS_REMOTE_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
pub struct Cache {
    /// Unix ms of the last check.
    pub checked: i64,
    /// Newest release found (kept from an earlier check when this one failed).
    pub latest: Option<String>,
    /// Why the last check failed, if it did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Cache {
    /// Whether it's time to check again.
    pub fn due(&self, now: i64) -> bool {
        now - self.checked >= if self.error.is_some() { RETRY_EVERY_MS } else { CHECK_EVERY_MS }
    }
}

pub fn read(path: &Path) -> Option<Cache> {
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

pub fn write(path: &Path, c: &Cache) -> Result<()> {
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(c)?)?;
    fs::rename(tmp, path)?;
    Ok(())
}

/// `X.Y.Z`, digits only: a release, not a pre-release or build.
pub fn is_release(v: &str) -> bool {
    let parts: Vec<_> = v.split('.').collect();
    parts.len() == 3 && parts.iter().all(|p| !p.is_empty() && p.len() <= 9 && p.bytes().all(|b| b.is_ascii_digit()))
}

/// Highest release among `git ls-remote --tags --refs` lines (`<sha>\trefs/tags/vX.Y.Z`).
pub fn newest(ls_remote: &str) -> Option<String> {
    ls_remote
        .lines()
        .filter_map(|l| l.split_once("refs/tags/v").map(|(_, v)| v.trim()))
        .filter(|v| is_release(v))
        .max_by(|a, b| version_cmp(a, b))
        .map(str::to_string)
}

/// The remote to look at: `$REDLINE_UPDATE_REMOTE` (tests, forks), else the clone's `origin`.
pub fn remote() -> String {
    std::env::var("REDLINE_UPDATE_REMOTE").ok().filter(|r| !r.is_empty()).unwrap_or_else(|| "origin".into())
}

/// Asks the remote for its tags and caches the answer. A failure is cached too (with the last known
/// `latest`), so an offline machine retries hourly rather than on every call.
pub fn check(repo: &Path, cache: &Path) -> Cache {
    let before = read(cache).unwrap_or_default();
    let c = match git(repo, &["ls-remote", "--tags", "--refs", &remote(), "v*"], LS_REMOTE_TIMEOUT) {
        Ok(out) => Cache { checked: now_ms(), latest: newest(&out), error: None },
        Err(e) => Cache { checked: now_ms(), latest: before.latest, error: Some(format!("{e:#}")) },
    };
    if let Err(e) = write(cache, &c) {
        tracing::warn!(error = %e, "couldn't write the update cache");
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newest_release_is_numeric_and_skips_prereleases() {
        let out = "a\trefs/tags/v0.2.2\nb\trefs/tags/v0.10.0\nc\trefs/tags/v0.9.9\nd\trefs/tags/v1.0.0-rc.1\ne\trefs/tags/vnext\n";
        assert_eq!(newest(out).as_deref(), Some("0.10.0"));
        assert_eq!(newest(""), None);
    }

    #[test]
    fn release_versions_only() {
        assert!(is_release("0.3.0"));
        for v in ["0.3", "0.3.0-rc.1", "0.3.0+x", "a.b.c", "", "1..2", "1.2.3.4"] {
            assert!(!is_release(v), "{v}");
        }
    }

    #[test]
    fn failed_checks_retry_sooner() {
        let ok = Cache { checked: 0, latest: None, error: None };
        let failed = Cache { error: Some("offline".into()), ..ok.clone() };
        assert!(!ok.due(RETRY_EVERY_MS) && ok.due(CHECK_EVERY_MS));
        assert!(failed.due(RETRY_EVERY_MS));
    }
}
