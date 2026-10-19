//! What the CLI commands do, where it's more than a call: status, strip, update.

use super::UpdateCmd;
use crate::api::Api;
use crate::{lifecycle, strip, update};
use anyhow::Result;
use serde_json::json;
use std::path::PathBuf;

pub(super) fn print(v: &impl serde::Serialize) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(v)?);
    Ok(())
}

/// `redline status`: never starts the daemon, so it can report "not running".
pub(super) fn daemon_status() -> Result<()> {
    if lifecycle::running().is_none() {
        return print(&json!({ "running": false }));
    }
    let api = Api::connect()?;
    let mut v = serde_json::to_value(api.status()?)?;
    v["running"] = json!(true);
    v["port"] = json!(api.port);
    v["update"] = json!(update::info(&update::Files::default()));
    print(&v)
}

pub(super) fn strip_files(paths: &[PathBuf], check: bool) -> Result<()> {
    let mut found = vec![];
    let files = strip::files(paths)?;
    for f in &files {
        let Ok(text) = std::fs::read_to_string(f) else { continue }; // binary or unreadable: not ours
        let (out, n) = strip::strip(&text);
        if n > 0 {
            if !check {
                std::fs::write(f, out)?;
            }
            found.push(json!({ "path": f, "removed": n }));
        }
    }
    let any = !found.is_empty();
    print(&json!({ "files_checked": files.len(), (if check { "found" } else { "stripped" }): found }))?;
    if check && any {
        std::process::exit(1);
    }
    Ok(())
}

pub(super) fn update_cmd(action: Option<UpdateCmd>) -> Result<()> {
    let files = update::Files::default();
    match action {
        None => print(&update::apply::apply(&files)?),
        Some(UpdateCmd::Check) => {
            update::config::load(&files.config)?; // surface a broken config.toml here
            let c = update::check(&files);
            let mut v = serde_json::to_value(update::info(&files))?;
            if let Some(e) = c.error {
                v["error"] = json!(e);
            }
            v["config"] = json!(files.config);
            print(&v)
        }
        Some(UpdateCmd::Skip { version }) => print(&update::skip(&files, version.as_deref())?),
        Some(UpdateCmd::Off) => print(&update::set_check(&files, false)?),
        Some(UpdateCmd::On) => print(&update::set_check(&files, true)?),
    }
}

/// `update` for agents: present only when there's something to tell the user.
pub(super) fn update_field() -> Option<serde_json::Value> {
    let u = update::info(&update::Files::default());
    u.notify.then(|| json!(u))
}
