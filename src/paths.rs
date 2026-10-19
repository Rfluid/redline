//! Where redline keeps its state on this device.

use crate::model::DaemonState;
use anyhow::Result;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_PORT: u16 = 7465;

/// `$REDLINE_HOME`, else `~/.local/state/redline` (the platform equivalent where there is one; on Windows
/// `%LOCALAPPDATA%\redline`). Created if missing.
pub fn state_dir() -> Result<PathBuf> {
    let dir = state_dir_path();
    if dir.as_os_str().is_empty() {
        anyhow::bail!("no home directory");
    }
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// [`state_dir`] without creating it (empty if there's no home directory).
pub fn state_dir_path() -> PathBuf {
    match std::env::var_os("REDLINE_HOME") {
        Some(d) => PathBuf::from(d),
        None => dirs::state_dir()
            .or_else(|| if cfg!(windows) { dirs::data_local_dir() } else { None })
            .or_else(|| dirs::home_dir().map(|h| h.join(".local/state")))
            .map(|d| d.join("redline"))
            .unwrap_or_default(),
    }
}

/// User settings: `$REDLINE_CONFIG`, else `$XDG_CONFIG_HOME/redline/config.toml`, else
/// `~/.config/redline/config.toml` (macOS too: it's what CLI users expect to find and edit).
pub fn config_file() -> PathBuf {
    if let Some(p) = std::env::var_os("REDLINE_CONFIG").filter(|p| !p.is_empty()) {
        return PathBuf::from(p);
    }
    std::env::var_os("XDG_CONFIG_HOME")
        .filter(|d| !d.is_empty())
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
        .unwrap_or_default()
        .join("redline/config.toml")
}

/// The clone this binary belongs to: the skill dir agents link to, and where `bin/redline` lives.
/// `$REDLINE_REPO` (set by `bin/redline`), else the checkout around the binary
/// (`<repo>/target/<dir>/redline`), else where it was built. A downloaded release binary has a
/// build path from CI, so the first two matter.
pub fn repo_dir() -> PathBuf {
    if let Some(r) = std::env::var_os("REDLINE_REPO").filter(|r| !r.is_empty()) {
        return PathBuf::from(r);
    }
    let exe = std::env::current_exe().and_then(|e| e.canonicalize()).ok().map(plain);
    exe.and_then(|e| checkout_of(&e)).unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

/// What agents' MCP configs and the CLI on PATH run.
/// - Unix: `<repo>/bin/redline`, which rebuilds (or downloads) the binary when needed.
/// - Windows: `<repo>\target\bin\redline.exe`, the copy `bin\redline.ps1` keeps current. A script
///   can't sit in an MCP stdio path there (see DECISIONS.md, "Windows").
pub fn entry_point(repo: &Path) -> PathBuf {
    if cfg!(windows) { repo.join("target").join("bin").join("redline.exe") } else { repo.join("bin").join("redline") }
}

/// A clone script (`bin/redline`, `scripts/checkout-release.sh`): the file, and the argv that runs it.
/// On Windows that's its `.ps1` twin under Windows PowerShell, which every Windows has.
pub fn script(unix_script: &Path) -> (PathBuf, Vec<OsString>) {
    if cfg!(windows) {
        let ps1 = unix_script.with_extension("ps1");
        let mut argv: Vec<OsString> =
            ["powershell.exe", "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File"]
                .map(OsString::from)
                .into();
        argv.push(ps1.clone().into_os_string());
        (ps1, argv)
    } else {
        (unix_script.to_path_buf(), vec![unix_script.as_os_str().to_owned()])
    }
}

/// Drops the `\\?\` prefix Windows' `canonicalize` adds (`\\?\C:\x` → `C:\x`), so paths we hand to agents,
/// scripts and junctions are ordinary ones. UNC paths and every path elsewhere pass through.
pub fn plain(p: PathBuf) -> PathBuf {
    match p.to_str().and_then(|s| s.strip_prefix(r"\\?\")) {
        Some(rest) if rest.as_bytes().get(1) == Some(&b':') => PathBuf::from(rest),
        _ => p,
    }
}

/// `<repo>` for `<repo>/target/<dir>/redline`, if `<repo>` is a redline checkout.
fn checkout_of(exe: &Path) -> Option<PathBuf> {
    let target = exe.ancestors().nth(2)?;
    let repo = target.parent()?;
    (target.file_name()? == "target" && repo.join("SKILL.md").is_file() && repo.join("bin/redline").is_file())
        .then(|| repo.to_path_buf())
}

pub fn read_daemon_state() -> Option<DaemonState> {
    let p = state_dir().ok()?.join("daemon.json");
    serde_json::from_slice(&fs::read(p).ok()?).ok()
}

pub fn write_daemon_state(s: &DaemonState) -> Result<()> {
    write_private(&state_dir()?.join("daemon.json"), &serde_json::to_vec_pretty(s)?)
}

pub fn remove_daemon_state() -> Result<()> {
    match fs::remove_file(state_dir()?.join("daemon.json")) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
        _ => Ok(()),
    }
}

/// Agent-side token. Stable across daemon restarts; readable only by this user.
pub fn admin_token() -> Result<String> {
    let p = state_dir()?.join("admin.token");
    if let Ok(t) = fs::read_to_string(&p) {
        let t = t.trim().to_string();
        if !t.is_empty() {
            return Ok(t);
        }
    }
    let t = crate::ids::new_token();
    write_private(&p, t.as_bytes())?;
    Ok(t)
}

/// Writes `p` atomically, readable only by this user. Windows: files under the profile already get a
/// user-only ACL, so there's nothing to set.
fn write_private(p: &Path, data: &[u8]) -> Result<()> {
    use std::io::Write;
    let tmp = p.with_extension("tmp");
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut opts, 0o600);
    let mut f = opts.open(&tmp)?;
    f.write_all(data)?;
    drop(f); // Windows can't rename a file that's still open

    fs::rename(tmp, p)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkout_is_found_from_a_binary_inside_it() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        fs::create_dir_all(repo.join("bin")).unwrap();
        fs::create_dir_all(repo.join("target/prebuilt")).unwrap();
        fs::write(repo.join("SKILL.md"), "x").unwrap();
        fs::write(repo.join("bin/redline"), "x").unwrap();
        assert_eq!(checkout_of(&repo.join("target/prebuilt/redline")).as_deref(), Some(repo));
        assert_eq!(checkout_of(&repo.join("elsewhere/a/redline")), None);
    }

    #[test]
    fn plain_drops_only_the_verbatim_drive_prefix() {
        assert_eq!(plain(PathBuf::from(r"\\?\C:\r\redline")), PathBuf::from(r"C:\r\redline"));
        assert_eq!(plain(PathBuf::from(r"\\?\UNC\host\share")), PathBuf::from(r"\\?\UNC\host\share"));
        assert_eq!(plain(PathBuf::from("/home/u/redline")), PathBuf::from("/home/u/redline"));
    }
}
