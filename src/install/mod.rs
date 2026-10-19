//! `redline install|uninstall|doctor`: link the skill and register the MCP server for each
//! coding agent. Mirrors handoff's installer, plus MCP registration.

mod agents;
mod link;
mod mcp_config;

pub use agents::expand;

use crate::paths;
use agents::{AGENTS, Agent};
use anyhow::{Result, bail};
use std::path::PathBuf;

pub const NAME: &str = "redline";

pub struct Opts {
    /// `None`/`"all"` = detected agents, else a comma list.
    pub agents: Option<String>,
    /// Extra Claude config dirs (second profiles).
    pub claude_config_dirs: Vec<PathBuf>,
    /// Extra skills dirs to link into.
    pub skill_dirs: Vec<PathBuf>,
    pub bin_dir: PathBuf,
    pub force: bool,
    pub no_mcp: bool,
}

/// Every Claude config dir to touch: default (`None`), `$CLAUDE_CONFIG_DIR`, and `--claude-config-dir`.
fn claude_dirs(extra: &[PathBuf]) -> Vec<Option<PathBuf>> {
    let mut v = vec![None];
    v.extend(std::env::var_os("CLAUDE_CONFIG_DIR").map(|d| Some(PathBuf::from(d))));
    v.extend(extra.iter().cloned().map(Some));
    v.dedup();
    v
}

fn skill_targets(agents: &[&Agent], o: &Opts) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = agents.iter().flat_map(|a| a.skill_dirs.iter().map(|d| expand(d).join(NAME))).collect();
    if agents.iter().any(|a| a.name == "claude") {
        out.extend(claude_dirs(&o.claude_config_dirs).into_iter().flatten().map(|d| d.join("skills").join(NAME)));
    }
    out.extend(o.skill_dirs.iter().map(|d| d.join(NAME)));
    out.sort();
    out.dedup();
    out
}

/// Our links in dirs earlier versions used (`Agent::legacy_skill_dirs`).
fn legacy_targets(agents: &[&Agent]) -> Vec<PathBuf> {
    agents.iter().flat_map(|a| a.legacy_skill_dirs.iter().map(|d| expand(d).join(NAME))).collect()
}

/// Our links in dirs `a` also loads without de-duplicating by name.
fn duplicate_links(a: &Agent, repo: &std::path::Path) -> Vec<PathBuf> {
    a.dup_skill_dirs.iter().map(|d| expand(d).join(NAME)).filter(|t| link::points_into(t, repo)).collect()
}

pub fn install(o: &Opts) -> Result<()> {
    let agents = agents::select(o.agents.as_deref())?;
    if agents.is_empty() {
        bail!("no agents detected; pass --agents claude,codex,...");
    }
    let repo = paths::repo_dir();
    let entry = paths::entry_point(&repo);
    if !entry.is_file() {
        // Windows: bin\redline.ps1 creates it; a binary run straight from target\release didn't.
        bail!(
            "{} is missing; run install through {}",
            entry.display(),
            paths::script(&repo.join("bin/redline")).0.display()
        );
    }
    let cdirs = claude_dirs(&o.claude_config_dirs);
    println!("agents: {}", agents.iter().map(|a| a.name).collect::<Vec<_>>().join(", "));
    let targets = skill_targets(&agents, o);
    for t in &targets {
        println!("  skill {}: {}", t.display(), link::link(&repo, t, o.force)?);
    }
    for t in legacy_targets(&agents).into_iter().filter(|t| !targets.contains(t) && link::points_into(t, &repo)) {
        link::remove(&t)?;
        println!("  skill {}: removed (old location)", t.display());
    }
    for cli in link::cli_targets(&o.bin_dir) {
        println!("  cli   {}: {}", cli.display(), link::link_cli(&entry, &cli, o.force)?);
    }
    if o.no_mcp {
        return Ok(());
    }
    for a in &agents {
        for (label, res) in mcp_config::register(a, &cdirs) {
            match res {
                Ok(s) => println!("  mcp   {label}: {s}"),
                Err(e) => println!("  mcp   {label}: FAILED: {e:#}"),
            }
        }
    }
    Ok(())
}

pub fn uninstall(o: &Opts) -> Result<()> {
    let all: Vec<&Agent> = AGENTS.iter().collect();
    let repo = paths::repo_dir();
    let cdirs = claude_dirs(&o.claude_config_dirs);
    for t in skill_targets(&all, o).into_iter().chain(legacy_targets(&all)).chain(link::cli_targets(&o.bin_dir)) {
        if link::points_into(&t, &repo) {
            link::remove(&t)?;
            println!("  removed {}", t.display());
        }
    }
    for a in &all {
        for (label, res) in mcp_config::unregister(a, &cdirs) {
            match res {
                Ok(true) => println!("  mcp   {label}: removed"),
                Ok(false) => {}
                Err(e) => println!("  mcp   {label}: FAILED: {e:#}"),
            }
        }
    }
    Ok(())
}

pub fn doctor(o: &Opts) -> Result<()> {
    let repo = paths::repo_dir();
    let cdirs = claude_dirs(&o.claude_config_dirs);
    println!("repo     {}", repo.display());
    println!("state    {}", paths::state_dir()?.display());
    match crate::lifecycle::running() {
        Some((p, h)) => println!("daemon   running on 127.0.0.1:{p} (v{}, pid {})", h.version, h.pid),
        None => println!("daemon   not running (starts on first use)"),
    }
    for cli in link::cli_targets(&o.bin_dir) {
        println!("cli      {}: {}", cli.display(), if link::points_into(&cli, &repo) { "linked" } else { "missing" });
    }
    for a in agents::detected() {
        let skills: Vec<_> = skill_targets(&[a], o)
            .into_iter()
            .map(|t| format!("{}={}", t.display(), if link::points_into(&t, &repo) { "ok" } else { "missing" }))
            .collect();
        println!("{:<9}skill {}", a.name, skills.join(" "));
        let dups = duplicate_links(a, &repo);
        if !dups.is_empty() {
            let dups: Vec<_> = dups.iter().map(|d| d.display().to_string()).collect();
            println!("{:<9}note  also loads {}: redline is listed twice there (harmless)", "", dups.join(", "));
        }
        for (label, ok) in mcp_config::present(a, &cdirs) {
            println!("{:<9}mcp   {label}: {}", "", if ok { "ok" } else { "missing" });
        }
    }
    Ok(())
}
