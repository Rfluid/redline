//! Daemon lifecycle from the agent side: find a healthy daemon, start one on demand,
//! replace a stale one after an update, stop it.

use crate::model::Health;
use crate::paths;
use anyhow::{Context, Result, bail};
use std::process::Command;
use std::time::{Duration, UNIX_EPOCH};

/// How long to wait for a daemon to come up or go down.
const START_POLLS: u32 = 50;
const POLL: Duration = Duration::from_millis(100);
const HEALTH_TIMEOUT: Duration = Duration::from_millis(500);

/// `/health` of the daemon on `port`, if it's a redline daemon.
pub fn health(port: u16) -> Option<Health> {
    ureq::get(&format!("http://127.0.0.1:{port}/health"))
        .timeout(HEALTH_TIMEOUT)
        .call()
        .ok()
        .and_then(|r| r.into_json::<Health>().ok())
        .filter(|h| h.name == "redline")
}

/// The running daemon's port and health, if any.
pub fn running() -> Option<(u16, Health)> {
    let port = paths::read_daemon_state()?.port;
    health(port).map(|h| (port, h))
}

/// Port of a daemon at least as new as this binary: reuses, replaces or starts one.
pub fn ensure() -> Result<u16> {
    match running() {
        Some((port, h)) if !is_stale(&h, env!("CARGO_PKG_VERSION"), exe_mtime_ms()) => Ok(port),
        Some(_) => {
            stop()?;
            spawn()
        }
        None => spawn(),
    }
}

/// Stops the running daemon (SIGTERM, then SIGKILL; Windows has no SIGTERM, so it's killed outright there).
/// Returns its pid, or `None` if none was running.
pub fn stop() -> Result<Option<u32>> {
    let Some((port, h)) = running() else { return Ok(None) };
    // The pid comes from the daemon itself, so a stale daemon.json can't make us kill a stranger.
    terminate(h.pid, false)?;
    if !wait_until(|| health(port).is_none()) {
        terminate(h.pid, true)?;
        if !wait_until(|| health(port).is_none()) {
            bail!("daemon pid {} did not stop", h.pid);
        }
    }
    // A killed daemon couldn't clean up after itself.
    if paths::read_daemon_state().is_some_and(|s| s.pid == h.pid) {
        paths::remove_daemon_state()?;
    }
    Ok(Some(h.pid))
}

#[cfg(unix)]
fn terminate(pid: u32, force: bool) -> Result<()> {
    // `kill` keeps us free of `unsafe` libc calls; it's on every Unix.
    let sig = if force { "-KILL" } else { "-TERM" };
    Command::new("kill").args([sig, &pid.to_string()]).status().context("running `kill`")?;
    Ok(())
}

#[cfg(windows)]
fn terminate(pid: u32, _force: bool) -> Result<()> {
    // The daemon has no window to send WM_CLOSE to, so `taskkill` without /F can't stop it. SQLite's
    // journal makes a hard stop safe: a write either committed or never happened.
    Command::new("taskkill").args(["/F", "/PID", &pid.to_string()]).output().context("running `taskkill`")?;
    Ok(())
}

fn wait_until(mut done: impl FnMut() -> bool) -> bool {
    (0..START_POLLS).any(|_| {
        std::thread::sleep(POLL);
        done()
    })
}

/// A daemon is stale if it's an older version than ours, or the same version started before our
/// binary was last rebuilt (dev builds keep the version). A newer daemon is left alone, so an
/// old client can't take it down.
fn is_stale(h: &Health, ours: &str, exe_mtime: Option<i64>) -> bool {
    match version_cmp(&h.version, ours) {
        std::cmp::Ordering::Less => true,
        std::cmp::Ordering::Greater => false,
        std::cmp::Ordering::Equal => exe_mtime.is_some_and(|m| m > h.started),
    }
}

/// Numeric dotted-version comparison; anything unparseable compares as `0`.
pub(crate) fn version_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |v: &str| -> Vec<u64> { v.split(['.', '-', '+']).take(3).map(|p| p.parse().unwrap_or(0)).collect() };
    parse(a).cmp(&parse(b))
}

/// Our binary's mtime in Unix ms. `None` when it was replaced under us (Linux reports `… (deleted)`).
fn exe_mtime_ms() -> Option<i64> {
    let m = std::env::current_exe().ok()?.metadata().ok()?.modified().ok()?;
    i64::try_from(m.duration_since(UNIX_EPOCH).ok()?.as_millis()).ok()
}

fn spawn() -> Result<u16> {
    use std::process::Stdio;
    let log_path = paths::state_dir()?.join("daemon.log");
    let log = std::fs::OpenOptions::new().create(true).append(true).open(&log_path)?;
    let mut cmd = Command::new(std::env::current_exe()?);
    cmd.arg("serve").stdin(Stdio::null()).stdout(Stdio::null()).stderr(log);
    detach(&mut cmd).context("starting daemon")?;
    let mut port = None;
    wait_until(|| {
        port = running().map(|(p, _)| p);
        port.is_some()
    });
    port.with_context(|| format!("daemon did not start; see {}", log_path.display()))
}

/// Starts the daemon outside our process group, so the agent stopping its MCP server (and its group)
/// leaves the daemon running for other sessions.
#[cfg(unix)]
fn detach(cmd: &mut Command) -> std::io::Result<()> {
    use std::os::unix::process::CommandExt;
    cmd.process_group(0).spawn().map(drop)
}

/// Windows: no console window, its own Ctrl-C group, and out of the agent's job object when the job
/// allows it, since a job set to kill-on-close would take the daemon down with the MCP server.
#[cfg(windows)]
fn detach(cmd: &mut Command) -> std::io::Result<()> {
    use std::os::windows::process::CommandExt;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let flags = CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP;
    match cmd.creation_flags(flags | CREATE_BREAKAWAY_FROM_JOB).spawn() {
        Ok(_) => Ok(()),
        // The job doesn't allow breakaway: start inside it.
        Err(_) => cmd.creation_flags(flags).spawn().map(drop),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(version: &str, started: i64) -> Health {
        Health { name: "redline".into(), version: version.into(), pid: 1, started }
    }

    #[test]
    fn older_daemon_is_stale_newer_is_not() {
        assert!(is_stale(&h("0.1.0", 0), "0.2.0", None));
        assert!(is_stale(&h("0.9.9", 0), "0.10.0", None), "numeric, not lexical");
        assert!(!is_stale(&h("0.3.0", 0), "0.2.0", Some(i64::MAX)));
    }

    #[test]
    fn same_version_restarts_only_after_rebuild() {
        assert!(is_stale(&h("0.1.0", 1000), "0.1.0", Some(2000)));
        assert!(!is_stale(&h("0.1.0", 2000), "0.1.0", Some(1000)));
        assert!(!is_stale(&h("0.1.0", 1000), "0.1.0", None), "replaced binary: can't tell, keep it");
    }
}
