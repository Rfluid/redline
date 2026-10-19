//! Registering the `redline` MCP server in each agent's config.
//! Configs are backed up once (`<file>.redline.bak`); JSONC is refused rather than mangled.

use super::NAME;
use super::agents::{Agent, McpConfig, expand, on_path, which};
use crate::paths;
use anyhow::{Context, Result, bail};
use serde_json::{Map, Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Command agents launch, with `mcp`: `<repo>/bin/redline` (Windows: `<repo>\target\bin\redline.exe`).
#[must_use]
pub fn mcp_command() -> PathBuf {
    paths::entry_point(&paths::repo_dir())
}

fn server_json() -> Value {
    json!({ "command": mcp_command(), "args": ["mcp"] })
}

fn cursor_json() -> Value {
    json!({ "type": "stdio", "command": mcp_command(), "args": ["mcp"] })
}

fn opencode_json() -> Value {
    json!({ "type": "local", "command": [mcp_command(), "mcp"], "enabled": true })
}

fn claude(dir: Option<&Path>, args: &[&str]) -> Result<Output> {
    // Full path: on Windows, `claude` may be `claude.cmd` (npm), which a bare name wouldn't find.
    let mut c = Command::new(which("claude").unwrap_or_else(|| "claude".into()));
    c.args(args);
    if let Some(d) = dir {
        c.env("CLAUDE_CONFIG_DIR", d);
    }
    c.output().context("running `claude`")
}

fn claude_label(dir: Option<&Path>) -> String {
    dir.map_or_else(|| "claude".into(), |d| format!("claude ({})", d.display()))
}

fn label(a: &Agent, p: &str) -> String {
    format!("{} {p}", a.name)
}

fn registered(changed: bool) -> String {
    if changed { "registered" } else { "already registered" }.into()
}

/// Adds the server for `a`. One `(label, outcome)` per config touched.
pub fn register(a: &Agent, claude_dirs: &[Option<PathBuf>]) -> Vec<(String, Result<String>)> {
    match &a.mcp {
        McpConfig::ClaudeCli => claude_dirs
            .iter()
            .map(|d| {
                let d = d.as_deref();
                (claude_label(d), claude_add(d))
            })
            .collect(),
        McpConfig::CodexToml(p) => vec![(label(a, p), edit_toml(&expand(p), true).map(registered))],
        McpConfig::ServersJson(p) => {
            vec![(
                label(a, p),
                edit_json(&expand(p), |m| set_entry(m, "mcpServers", Some(server_json()))).map(registered),
            )]
        }
        McpConfig::CursorJson(p) => {
            vec![(
                label(a, p),
                edit_json(&expand(p), |m| set_entry(m, "mcpServers", Some(cursor_json()))).map(registered),
            )]
        }
        McpConfig::OpencodeJson(p) => {
            vec![(label(a, p), edit_json(&expand(p), |m| set_entry(m, "mcp", Some(opencode_json()))).map(registered))]
        }
    }
}

/// The file `claude mcp add --scope user` edits: `<CLAUDE_CONFIG_DIR>/.claude.json`, else `~/.claude.json`.
fn claude_json(dir: Option<&Path>) -> PathBuf {
    dir.map_or_else(|| expand("~/.claude.json"), |d| d.join(".claude.json"))
}

fn claude_add(dir: Option<&Path>) -> Result<String> {
    if claude(dir, &["mcp", "get", NAME])?.status.success() {
        return Ok("already registered".into());
    }
    // `claude` edits its own config, but keep the same promise as for the other agents: a backup first.
    backup_once(&claude_json(dir))?;
    let cmd = mcp_command();
    let out = claude(dir, &["mcp", "add", "--scope", "user", NAME, "--", &cmd.to_string_lossy(), "mcp"])?;
    if !out.status.success() {
        bail!("{}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok("registered (user scope)".into())
}

/// Removes the server for `a`. `Ok(true)` if something was removed.
pub fn unregister(a: &Agent, claude_dirs: &[Option<PathBuf>]) -> Vec<(String, Result<bool>)> {
    match &a.mcp {
        McpConfig::ClaudeCli if on_path("claude") => claude_dirs
            .iter()
            .map(|d| {
                let d = d.as_deref();
                let res = claude(d, &["mcp", "remove", "--scope", "user", NAME]).map(|o| o.status.success());
                (claude_label(d), res)
            })
            .collect(),
        McpConfig::CodexToml(p) if expand(p).exists() => vec![(label(a, p), edit_toml(&expand(p), false))],
        McpConfig::ServersJson(p) | McpConfig::CursorJson(p) if expand(p).exists() => {
            vec![(label(a, p), edit_json(&expand(p), |m| set_entry(m, "mcpServers", None)))]
        }
        McpConfig::OpencodeJson(p) if expand(p).exists() => {
            vec![(label(a, p), edit_json(&expand(p), |m| set_entry(m, "mcp", None)))]
        }
        _ => vec![],
    }
}

/// Whether the server is registered, per config.
pub fn present(a: &Agent, claude_dirs: &[Option<PathBuf>]) -> Vec<(String, bool)> {
    let in_json = |p: &str, key: &str| {
        fs::read_to_string(expand(p))
            .ok()
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
            .is_some_and(|v| v[key][NAME].is_object())
    };
    match &a.mcp {
        McpConfig::ClaudeCli => claude_dirs
            .iter()
            .map(|d| {
                let d = d.as_deref();
                (claude_label(d), claude(d, &["mcp", "get", NAME]).is_ok_and(|o| o.status.success()))
            })
            .collect(),
        McpConfig::CodexToml(p) => {
            let found = fs::read_to_string(expand(p))
                .ok()
                .and_then(|s| s.parse::<toml_edit::DocumentMut>().ok())
                .is_some_and(|d| d.get("mcp_servers").and_then(|t| t.get(NAME)).is_some());
            vec![((*p).to_string(), found)]
        }
        McpConfig::ServersJson(p) | McpConfig::CursorJson(p) => vec![((*p).to_string(), in_json(p, "mcpServers"))],
        McpConfig::OpencodeJson(p) => vec![((*p).to_string(), in_json(p, "mcp"))],
    }
}

/// Sets (`Some`) or removes (`None`) `root[key]["redline"]`. Returns whether anything changed.
pub(crate) fn set_entry(root: &mut Map<String, Value>, key: &str, val: Option<Value>) -> bool {
    let section = root.entry(key).or_insert_with(|| json!({}));
    let Some(section) = section.as_object_mut() else { return false };
    match val {
        Some(v) if section.get(NAME) == Some(&v) => false,
        Some(v) => {
            section.insert(NAME.into(), v);
            true
        }
        None => {
            let removed = section.remove(NAME).is_some();
            // We created the section if it held only us; don't leave `"mcpServers": {}` behind.
            if removed && section.is_empty() {
                root.remove(key);
            }
            removed
        }
    }
}

/// Edits a JSON config in place via `f`; writes (after a one-time backup) only if `f` changed it.
pub(crate) fn edit_json(path: &Path, f: impl FnOnce(&mut Map<String, Value>) -> bool) -> Result<bool> {
    // Missing and empty files both mean "no config yet" (Antigravity ships an empty mcp_config.json).
    let text = fs::read_to_string(path).ok().filter(|t| !t.trim().is_empty()).unwrap_or_else(|| "{}".into());
    let mut v: Value = serde_json::from_str(&text).with_context(|| {
        format!("{} isn't plain JSON (comments?); add the redline server by hand: {}", path.display(), server_json())
    })?;
    let m = v.as_object_mut().context("config root isn't an object")?;
    if !f(m) {
        return Ok(false);
    }
    // Back to exactly what the backup holds (uninstall of an untouched config): restore it byte for byte.
    let restored = read_backup(path).filter(|b| serde_json::from_str::<Value>(b).is_ok_and(|bv| bv == v));
    write_with_backup(path, &restored.unwrap_or(serde_json::to_string_pretty(&v)? + "\n"))?;
    Ok(true)
}

fn backup_path(path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.redline.bak", path.display()))
}

pub(super) fn read_backup(path: &Path) -> Option<String> {
    fs::read_to_string(backup_path(path)).ok()
}

/// Copies `path` to `<path>.redline.bak` unless a backup exists already (the first one is the original).
fn backup_once(path: &Path) -> Result<()> {
    let bak = backup_path(path);
    if path.exists() && !bak.exists() {
        fs::copy(path, bak).with_context(|| format!("backing up {}", path.display()))?;
    }
    Ok(())
}

pub(super) fn write_with_backup(path: &Path, contents: &str) -> Result<()> {
    backup_once(path)?;
    fs::create_dir_all(path.parent().context("bad path")?)?;
    fs::write(path, contents)?;
    Ok(())
}

mod toml_file;
pub(crate) use toml_file::edit_toml;

#[cfg(test)]
mod tests;
