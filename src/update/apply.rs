//! `redline update`: moves the clone to the newest release with `scripts/checkout-release.sh` (the same
//! script `INSTALL.md` uses), then runs `bin/redline` once so it rebuilds (or downloads) the binary. The
//! daemon handshake swaps in a new daemon on the next call. On Windows both are their `.ps1` twins.

use super::git::git;
use super::release::{self, Cache};
use super::{Files, notes_url};
use crate::ids::now_ms;
use crate::lifecycle::version_cmp;
use crate::paths;
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

const LOCAL: Duration = Duration::from_secs(10);
const FETCH: Duration = Duration::from_secs(120);

pub fn apply(files: &Files) -> Result<Value> {
    let repo = paths::repo_dir();
    git(&repo, &["rev-parse", "--git-dir"], LOCAL).with_context(|| {
        format!("{} isn't a git clone, so it can't update itself; reinstall redline", repo.display())
    })?;
    let before = head(&repo)?;
    checkout_release(&repo)?;
    let tag = git(&repo, &["describe", "--tags", "--exact-match"], LOCAL).unwrap_or_default();
    let latest = tag.trim().trim_start_matches('v').to_string();
    let version = rebuild(&repo)?;
    if release::is_release(&latest) {
        release::write(&files.cache, &Cache { checked: now_ms(), latest: Some(latest.clone()), error: None })?;
    }
    Ok(json!({
        "updated": head(&repo)? != before,
        "from": env!("CARGO_PKG_VERSION"),
        "to": latest,
        "version": version,
        "notes": notes_url(&latest),
        "next": "Restart open agent sessions to load the new MCP server. The daemon restarts on its own.",
    }))
}

/// `scripts/checkout-release.sh` (`.ps1` on Windows): fetches tags, refuses to run over local edits, checks
/// out the highest vX.Y.Z (detached). Its messages go to stderr, so our stdout stays JSON. Clones at v0.2.2
/// or older lack the script; the same steps run inline there.
fn checkout_release(repo: &Path) -> Result<()> {
    let (script, argv) = paths::script(&repo.join("scripts/checkout-release.sh"));
    if script.is_file() {
        let ok = Command::new(&argv[0])
            .args(&argv[1..])
            .env("REMOTE", release::remote())
            .stdin(Stdio::null())
            .stdout(Stdio::from(std::io::stderr()))
            .status()
            .context("running scripts/checkout-release.sh")?
            .success();
        if !ok {
            bail!("couldn't switch {} to the latest release (see above)", repo.display());
        }
        return Ok(());
    }
    git(repo, &["fetch", "--quiet", "--tags", "--force", &release::remote()], FETCH)?;
    if !git(repo, &["status", "--porcelain", "--untracked-files=no"], LOCAL)?.trim().is_empty() {
        bail!("{} has local changes; commit or stash them, then run `redline update` again", repo.display());
    }
    let tags = git(repo, &["tag", "--list", "v*"], LOCAL)?;
    let latest = tags
        .lines()
        .filter_map(|t| t.trim().strip_prefix('v'))
        .filter(|v| release::is_release(v))
        .max_by(|a, b| version_cmp(a, b))
        .context("the remote has no release tags (vX.Y.Z)")?;
    git(repo, &["-c", "advice.detachedHead=false", "checkout", "--quiet", "--detach", &format!("v{latest}")], LOCAL)?;
    Ok(())
}

fn head(repo: &Path) -> Result<String> {
    Ok(git(repo, &["rev-parse", "HEAD"], LOCAL)?.trim().to_string())
}

/// Runs the clone's `bin/redline --version`: it builds or downloads the new binary (progress on our stderr).
fn rebuild(repo: &Path) -> Result<String> {
    let (_, argv) = paths::script(&repo.join("bin/redline"));
    let out = Command::new(&argv[0])
        .args(&argv[1..])
        .arg("--version")
        .stdin(Stdio::null())
        .stderr(Stdio::inherit())
        .output()
        .context("running bin/redline")?;
    if !out.status.success() {
        bail!("bin/redline couldn't build or download the new version (see above)");
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}
