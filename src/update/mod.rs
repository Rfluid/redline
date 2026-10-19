//! Updates: a daily check for a newer release, notices (overlay, CLI, agents) the user can turn off or
//! skip per version, and `redline update`. See `docs/DECISIONS.md` › Updates.
//!
//! - [`release`]: finds the newest release tag and caches it in `update.json`.
//! - [`config`]: `[update]` in `config.toml` (`check`, `skip`).
//! - [`apply`]: `redline update` itself.

pub mod apply;
pub mod config;
mod git;
pub mod release;

use crate::ids::now_ms;
use crate::lifecycle::version_cmp;
use crate::model::UpdateInfo;
use crate::paths;
use anyhow::{Result, bail};
use config::UpdateConfig;
use release::Cache;
use std::cmp::Ordering;
use std::path::PathBuf;

pub const RELEASES_URL: &str = "https://github.com/Rfluid/redline/releases";
pub const COMMAND: &str = "redline update";

/// Where the update settings and the cached check live. Tests point these at a temp dir.
#[derive(Clone, Debug)]
pub struct Files {
    pub config: PathBuf,
    pub cache: PathBuf,
}

impl Default for Files {
    fn default() -> Self {
        Self { config: paths::config_file(), cache: paths::state_dir_path().join("update.json") }
    }
}

pub fn notes_url(version: &str) -> String {
    format!("{RELEASES_URL}/tag/v{version}")
}

/// What's known about a newer release, from the cache and the config. No network.
pub fn info(files: &Files) -> UpdateInfo {
    let cfg = config::load(&files.config).unwrap_or_else(|e| {
        tracing::warn!(error = %format!("{e:#}"), "ignoring config.toml");
        UpdateConfig::default()
    });
    info_from(env!("CARGO_PKG_VERSION"), release::read(&files.cache).as_ref(), &cfg)
}

pub fn info_from(current: &str, cache: Option<&Cache>, cfg: &UpdateConfig) -> UpdateInfo {
    let latest = cache.and_then(|c| c.latest.clone());
    let available = latest.as_deref().is_some_and(|l| version_cmp(l, current) == Ordering::Greater);
    let skipped = latest.as_ref().is_some_and(|l| cfg.skip.contains(l));
    UpdateInfo {
        current: current.into(),
        notes: latest.as_deref().map(notes_url),
        latest,
        available,
        notify: available && cfg.check && !skipped,
        check: cfg.check,
        skipped,
        checked: cache.map(|c| c.checked),
        command: COMMAND.into(),
    }
}

/// Background checks are on unless `config.toml` says `check = false`, `$REDLINE_NO_UPDATE_CHECK` is
/// set, or we run in CI (`$CI`). An explicit `redline update check` always runs.
pub fn auto_check_enabled(files: &Files) -> bool {
    let set = |k: &str| std::env::var_os(k).is_some_and(|v| !v.is_empty() && v != "0" && v != "false");
    !set("REDLINE_NO_UPDATE_CHECK") && !set("CI") && config::load(&files.config).map_or(true, |c| c.check)
}

/// Checks now if background checks are on and the last one is old enough (the daemon calls this hourly).
pub fn auto_check(files: &Files) -> Option<Cache> {
    let due = release::read(&files.cache).is_none_or(|c| c.due(now_ms()));
    (due && auto_check_enabled(files)).then(|| check(files))
}

/// Checks the remote now.
pub fn check(files: &Files) -> Cache {
    release::check(&paths::repo_dir(), &files.cache)
}

/// Stops notices for `version` (default: the newest known release, checking first if none is known).
pub fn skip(files: &Files, version: Option<&str>) -> Result<UpdateInfo> {
    let version = match version {
        Some(v) => v.trim_start_matches('v').to_string(),
        None => match info(files).latest.or_else(|| check(files).latest) {
            Some(v) => v,
            None => bail!("no release known yet; run `redline update check`, or name the version"),
        },
    };
    if !release::is_release(&version) {
        bail!("{version:?} isn't a release version (X.Y.Z)");
    }
    config::skip(&files.config, &version)?;
    Ok(info(files))
}

/// Turns checks and notices on or off.
pub fn set_check(files: &Files, on: bool) -> Result<UpdateInfo> {
    config::set_check(&files.config, on)?;
    Ok(info(files))
}

/// The CLI's stderr notice, like `gh`'s: what's new, how to update, how to stop seeing it.
pub fn notice(u: &UpdateInfo) -> Option<String> {
    let latest = u.latest.as_deref().filter(|_| u.notify)?;
    Some(format!(
        "\nA new release of redline is available: {} → {latest}\n\
         To update, run: {COMMAND}   (skip this one: redline update skip; no notices: redline update off)\n{}\n",
        u.current,
        notes_url(latest)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cache(latest: &str) -> Cache {
        Cache { checked: 1, latest: Some(latest.into()), error: None }
    }

    #[test]
    fn newer_release_notifies_unless_skipped_or_off() {
        let on = UpdateConfig::default();
        let u = info_from("0.2.2", Some(&cache("0.3.0")), &on);
        assert!(u.available && u.notify && !u.skipped);
        assert_eq!(u.notes.as_deref(), Some("https://github.com/Rfluid/redline/releases/tag/v0.3.0"));
        assert!(notice(&u).is_some_and(|n| n.contains("0.2.2 → 0.3.0")));

        let skip = UpdateConfig { check: true, skip: vec!["0.3.0".into()] };
        let u = info_from("0.2.2", Some(&cache("0.3.0")), &skip);
        assert!(u.available && u.skipped && !u.notify);
        assert!(info_from("0.2.2", Some(&cache("0.3.1")), &skip).notify, "a newer release than the skipped one shows");

        let off = UpdateConfig { check: false, skip: vec![] };
        assert!(!info_from("0.2.2", Some(&cache("0.3.0")), &off).notify);
        assert_eq!(notice(&info_from("0.2.2", Some(&cache("0.3.0")), &off)), None);
    }

    #[test]
    fn same_or_older_release_is_not_an_update() {
        let on = UpdateConfig::default();
        assert!(!info_from("0.3.0", Some(&cache("0.3.0")), &on).available);
        assert!(!info_from("0.10.0", Some(&cache("0.9.0")), &on).available);
        assert!(!info_from("0.3.0", None, &on).available);
    }

    #[test]
    fn skip_validates_and_records_the_version() {
        let dir = tempfile::tempdir().unwrap();
        let files = Files { config: dir.path().join("config.toml"), cache: dir.path().join("update.json") };
        release::write(&files.cache, &cache("9.0.0")).unwrap();
        assert!(skip(&files, Some("../x")).is_err());
        let u = skip(&files, None).unwrap();
        assert!(u.skipped && !u.notify);
        assert_eq!(config::load(&files.config).unwrap().skip, ["9.0.0"]);
        assert!(!set_check(&files, false).unwrap().check);
    }
}
