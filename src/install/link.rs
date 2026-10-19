//! Links that never clobber something that isn't ours.
//! - Skill dirs: a symlink on Unix, a junction on Windows (symlinks need admin or Developer Mode there;
//!   junctions don't, and agents read through them the same way).
//! - The CLI on PATH: a symlink on Unix; on Windows, small `redline.cmd` (cmd, PowerShell) and `redline`
//!   (Git Bash) shims that run the entry point.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// First line of every CLI shim we write, so we know it's ours.
#[cfg(windows)]
const SHIM_MARK: &str = "redline CLI shim, written by `redline install`";

/// Links `src` at `dst`. An existing non-redline target is skipped unless `force`, in which case it's
/// renamed to `<dst>.bak.<ms>` first. Returns a human-readable outcome.
pub fn link(src: &Path, dst: &Path, force: bool) -> Result<String> {
    if target(dst).is_some_and(|cur| cur == src) {
        return Ok("already linked".into());
    }
    if let Some(skipped) = make_room(dst, force)? {
        return Ok(skipped);
    }
    make_link(src, dst)?;
    Ok("linked".into())
}

/// The files that put `redline` on PATH in `bin_dir`.
pub fn cli_targets(bin_dir: &Path) -> Vec<PathBuf> {
    if cfg!(windows) {
        vec![bin_dir.join("redline.cmd"), bin_dir.join("redline")]
    } else {
        vec![bin_dir.join("redline")]
    }
}

/// Puts `cli` (one of [`cli_targets`]) on PATH, running `entry` (`paths::entry_point`).
pub fn link_cli(entry: &Path, cli: &Path, force: bool) -> Result<String> {
    #[cfg(windows)]
    {
        let body = shim(entry, cli);
        if fs::read_to_string(cli).is_ok_and(|cur| cur == body) {
            return Ok("already linked".into());
        }
        // An older shim of ours (another clone, an older format) is replaced without --force.
        let ours = fs::read_to_string(cli).is_ok_and(|cur| cur.contains(SHIM_MARK));
        if !ours && let Some(skipped) = make_room(cli, force)? {
            return Ok(skipped);
        }
        fs::create_dir_all(cli.parent().context("bad target")?)?;
        fs::write(cli, body)?;
        Ok("linked".into())
    }
    #[cfg(not(windows))]
    link(entry, cli, force)
}

/// A shim that runs `entry` with the caller's arguments.
#[cfg(windows)]
fn shim(entry: &Path, cli: &Path) -> String {
    if cli.extension().is_some_and(|e| e.eq_ignore_ascii_case("cmd")) {
        format!("@echo off\r\nrem {SHIM_MARK}\r\n\"{}\" %*\r\n", entry.display())
    } else {
        // Git Bash takes C:/x paths; it runs extensionless scripts, not .cmd files.
        let entry = entry.display().to_string().replace('\\', "/");
        format!("#!/bin/sh\n# {SHIM_MARK}\nexec \"{entry}\" \"$@\"\n")
    }
}

/// True if `p` is a link (or, on Windows, a CLI shim) pointing inside `root`.
#[must_use]
pub fn points_into(p: &Path, root: &Path) -> bool {
    #[cfg(windows)]
    let shim = || {
        let root = root.to_string_lossy();
        // The Git Bash shim spells the path with forward slashes.
        fs::read_to_string(p)
            .is_ok_and(|s| s.contains(SHIM_MARK) && s.replace('\\', "/").contains(&root.replace('\\', "/")))
    };
    #[cfg(not(windows))]
    let shim = || false;
    target(p).is_some_and(|dst| dst.starts_with(root)) || shim()
}

/// Removes a link or shim (not what it points to).
pub fn remove(p: &Path) -> Result<()> {
    // A junction is a directory: remove_dir unlinks it and leaves the target alone.
    #[cfg(windows)]
    if junction::exists(p).unwrap_or(false) {
        return Ok(fs::remove_dir(p)?);
    }
    Ok(fs::remove_file(p)?)
}

/// Where `p` links to, if it's a link.
fn target(p: &Path) -> Option<PathBuf> {
    #[cfg(windows)]
    if let Ok(t) = junction::get_target(p) {
        return Some(crate::paths::plain(t));
    }
    fs::read_link(p).ok().map(crate::paths::plain)
}

/// Clears `dst` for a new link: nothing there is fine; something else is backed up with `force` and
/// skipped (`Some(outcome)`) without.
fn make_room(dst: &Path, force: bool) -> Result<Option<String>> {
    if dst.symlink_metadata().is_err() {
        return Ok(None);
    }
    if !force {
        return Ok(Some("SKIPPED: exists and isn't redline (use --force to back up and replace)".into()));
    }
    let bak = PathBuf::from(format!("{}.bak.{}", dst.display(), crate::ids::now_ms()));
    fs::rename(dst, &bak)?;
    Ok(None)
}

fn make_link(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst.parent().context("bad target")?)?;
    #[cfg(windows)]
    junction::create(src, dst)?;
    #[cfg(not(windows))]
    std::os::unix::fs::symlink(src, dst)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_is_idempotent_and_respects_foreign_files() {
        let dir = tempfile::tempdir().unwrap();
        let (src, dst) = (dir.path().join("src"), dir.path().join("a/dst"));
        fs::create_dir(&src).unwrap();
        assert_eq!(link(&src, &dst, false).unwrap(), "linked");
        assert_eq!(link(&src, &dst, false).unwrap(), "already linked");
        assert!(points_into(&dst, dir.path()));
        assert!(dst.join(".").is_dir(), "the link resolves to the dir");

        let foreign = dir.path().join("foreign");
        fs::write(&foreign, "x").unwrap();
        assert!(link(&src, &foreign, false).unwrap().starts_with("SKIPPED"));
        assert_eq!(link(&src, &foreign, true).unwrap(), "linked");

        remove(&dst).unwrap();
        assert!(dst.symlink_metadata().is_err());
        assert!(src.is_dir(), "removing the link leaves its target");
    }

    #[test]
    fn cli_link_is_idempotent_and_respects_foreign_files() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("repo");
        let entry = repo.join("bin/redline");
        fs::create_dir_all(entry.parent().unwrap()).unwrap();
        fs::write(&entry, "x").unwrap();
        for cli in cli_targets(&dir.path().join("bin")) {
            assert_eq!(link_cli(&entry, &cli, false).unwrap(), "linked");
            assert_eq!(link_cli(&entry, &cli, false).unwrap(), "already linked");
            assert!(points_into(&cli, &repo));
            remove(&cli).unwrap();
            assert!(!points_into(&cli, &repo));

            fs::write(&cli, "someone else's").unwrap();
            assert!(link_cli(&entry, &cli, false).unwrap().starts_with("SKIPPED"));
            assert!(!points_into(&cli, &repo));
        }
    }
}
