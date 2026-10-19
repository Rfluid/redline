//! Supported coding agents: where they read skills, how to detect them, where their MCP config lives.

use anyhow::{Context, Result};
use std::path::PathBuf;

/// How an agent registers MCP servers.
pub enum McpConfig {
    /// `claude mcp add --scope user` (honours `CLAUDE_CONFIG_DIR`).
    ClaudeCli,
    /// `[mcp_servers.redline]` in a TOML file.
    CodexToml(&'static str),
    /// `{"mcpServers": {"redline": {command, args}}}` in a JSON file.
    ServersJson(&'static str),
    /// Cursor: like `ServersJson`, plus `"type": "stdio"` (its field table lists it as required).
    CursorJson(&'static str),
    /// opencode: `{"mcp": {"redline": {type: local, command: [...]}}}`.
    OpencodeJson(&'static str),
}

pub struct Agent {
    pub name: &'static str,
    pub skill_dirs: &'static [&'static str],
    /// Other dirs the agent loads skills from *without* de-duplicating by name. A redline link in
    /// one of them makes the skill show up twice (`doctor` says so).
    pub dup_skill_dirs: &'static [&'static str],
    /// Dirs earlier versions linked into; install and uninstall remove our link there.
    pub legacy_skill_dirs: &'static [&'static str],
    pub bins: &'static [&'static str],
    pub dirs: &'static [&'static str],
    pub mcp: McpConfig,
}

pub const AGENTS: &[Agent] = &[
    Agent {
        name: "claude",
        skill_dirs: &["~/.claude/skills"],
        bins: &["claude"],
        dirs: &["~/.claude"],
        dup_skill_dirs: &[],
        legacy_skill_dirs: &[],
        mcp: McpConfig::ClaudeCli,
    },
    Agent {
        name: "codex",
        skill_dirs: &["~/.agents/skills"],
        bins: &["codex"],
        dirs: &["~/.codex"],
        dup_skill_dirs: &[],
        legacy_skill_dirs: &[],
        mcp: McpConfig::CodexToml("~/.codex/config.toml"),
    },
    Agent {
        name: "opencode",
        // Also reads ~/.claude/skills and ~/.config/opencode/skills, but keys skills by name.
        skill_dirs: &["~/.agents/skills"],
        dup_skill_dirs: &[],
        legacy_skill_dirs: &[],
        bins: &["opencode"],
        dirs: &["~/.config/opencode", "~/.opencode"],
        mcp: McpConfig::OpencodeJson("~/.config/opencode/opencode.json"),
    },
    Agent {
        name: "cursor",
        // Cursor reads ~/.agents/skills, ~/.cursor/skills and, for compatibility, ~/.claude/skills and
        // ~/.codex/skills, without de-duplicating. The shared dir keeps it at one copy unless Claude's
        // dir is linked too (see DECISIONS.md, "Skill dirs").
        skill_dirs: &["~/.agents/skills"],
        dup_skill_dirs: &["~/.claude/skills", "~/.codex/skills", "~/.cursor/skills"],
        legacy_skill_dirs: &["~/.cursor/skills"],
        bins: &["cursor", "cursor-agent"],
        dirs: &["~/.cursor"],
        mcp: McpConfig::CursorJson("~/.cursor/mcp.json"),
    },
    Agent {
        name: "agy",
        // App/IDE and CLI read different global skills dirs.
        skill_dirs: &["~/.gemini/config/skills", "~/.gemini/antigravity-cli/skills"],
        dup_skill_dirs: &[],
        legacy_skill_dirs: &[],
        bins: &["agy", "antigravity"],
        dirs: &["~/.gemini/config", "~/.gemini/antigravity-cli", "~/.gemini/antigravity"],
        mcp: McpConfig::ServersJson("~/.gemini/config/mcp_config.json"),
    },
    Agent {
        // Gemini CLI is still maintained, but Antigravity CLI replaced it for unpaid users (2026-06-18).
        // agy reads neither settings.json nor ~/.gemini/skills, so both stay separate agents.
        name: "gemini",
        // Reads ~/.gemini/skills and the ~/.agents/skills alias, and warns ("Skill conflict detected")
        // when both hold one; the shared dir is enough (checked with gemini 0.38.2, 2026-10-16).
        skill_dirs: &["~/.agents/skills"],
        dup_skill_dirs: &[],
        legacy_skill_dirs: &["~/.gemini/skills"],
        bins: &["gemini"],
        dirs: &[],
        mcp: McpConfig::ServersJson("~/.gemini/settings.json"),
    },
];

/// Agent home dirs that an env var can relocate: `(default, var)`.
const HOME_OVERRIDES: &[(&str, &str)] = &[("~/.codex", "CODEX_HOME")];

/// Expands a leading `~/`, honouring `HOME_OVERRIDES` (e.g. `~/.codex/x` → `$CODEX_HOME/x`).
#[must_use]
pub fn expand(p: &str) -> PathBuf {
    expand_with(p, |var| std::env::var_os(var).filter(|v| !v.is_empty()).map(PathBuf::from))
}

fn expand_with(p: &str, env: impl Fn(&str) -> Option<PathBuf>) -> PathBuf {
    for (default, var) in HOME_OVERRIDES {
        let Some(rest) = p.strip_prefix(default) else { continue };
        if !(rest.is_empty() || rest.starts_with('/')) {
            continue;
        }
        if let Some(home) = env(var) {
            return join(home, rest);
        }
    }
    match p.strip_prefix("~/") {
        Some(rest) => join(dirs::home_dir().unwrap_or_default(), rest),
        None => PathBuf::from(p),
    }
}

/// `base` + a `/`-separated `rest`, one component at a time, so Windows paths get `\` throughout.
fn join(base: PathBuf, rest: &str) -> PathBuf {
    rest.split('/').filter(|c| !c.is_empty()).fold(base, |p, c| p.join(c))
}

/// `bin`'s full path on PATH. Windows tries each `PATHEXT` extension (`claude.exe`, `codex.cmd`), and
/// only those: an extensionless file there is a Git Bash shim that can't be run directly.
#[must_use]
pub fn which(bin: &str) -> Option<PathBuf> {
    let exts: Vec<String> = if cfg!(windows) {
        let pathext = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into());
        pathext.split(';').filter(|e| !e.is_empty()).map(str::to_string).collect()
    } else {
        vec![String::new()]
    };
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .flat_map(|d| exts.iter().map(move |e| d.join(format!("{bin}{e}"))))
        .find(|p| p.is_file())
}

#[must_use]
pub fn on_path(bin: &str) -> bool {
    which(bin).is_some()
}

/// Agents with a binary on PATH or a config dir on disk.
#[must_use]
pub fn detected() -> Vec<&'static Agent> {
    AGENTS.iter().filter(|a| a.bins.iter().any(|b| on_path(b)) || a.dirs.iter().any(|d| expand(d).exists())).collect()
}

/// `None`/`"all"` → detected agents; otherwise a comma list of names.
pub fn select(spec: Option<&str>) -> Result<Vec<&'static Agent>> {
    match spec {
        None | Some("all") => Ok(detected()),
        Some(list) => list
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|n| {
                AGENTS.iter().find(|a| a.name == n).with_context(|| {
                    let known: Vec<_> = AGENTS.iter().map(|a| a.name).collect();
                    format!("unknown agent {n}; known: {}", known.join(","))
                })
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_by_name() {
        let got = select(Some("claude, codex")).unwrap();
        assert_eq!(got.iter().map(|a| a.name).collect::<Vec<_>>(), ["claude", "codex"]);
        assert!(select(Some("nope")).is_err());
    }

    #[test]
    fn expand_tilde() {
        assert!(expand("~/x").ends_with("x") && expand("~/x").is_absolute());
        assert_eq!(expand("/abs"), PathBuf::from("/abs"));
    }

    #[test]
    fn expand_honours_codex_home() {
        let env = |v: &str| (v == "CODEX_HOME").then(|| PathBuf::from("/ch"));
        assert_eq!(expand_with("~/.codex/config.toml", env), PathBuf::from("/ch/config.toml"));
        assert_eq!(expand_with("~/.codex", env), PathBuf::from("/ch"));
        assert!(expand_with("~/.codexx/a", env).ends_with(".codexx/a"), "prefix must end at a path separator");
        assert!(expand_with("~/.codex/config.toml", |_| None).ends_with(".codex/config.toml"));
    }

    #[test]
    fn agy_uses_current_antigravity_paths() {
        // https://antigravity.google/docs/mcp and /docs/skills (checked 2026-10-15).
        let agy = AGENTS.iter().find(|a| a.name == "agy").unwrap();
        assert!(matches!(agy.mcp, McpConfig::ServersJson("~/.gemini/config/mcp_config.json")));
        assert_eq!(agy.skill_dirs, ["~/.gemini/config/skills", "~/.gemini/antigravity-cli/skills"]);
    }

    #[test]
    fn gemini_uses_the_shared_skills_dir() {
        let gemini = AGENTS.iter().find(|a| a.name == "gemini").unwrap();
        assert_eq!(gemini.skill_dirs, ["~/.agents/skills"]);
        assert_eq!(gemini.legacy_skill_dirs, ["~/.gemini/skills"]);
    }

    #[test]
    fn cursor_uses_the_shared_skills_dir() {
        // https://cursor.com/docs/context/skills (checked 2026-10-16): no de-duplication across dirs.
        let cursor = AGENTS.iter().find(|a| a.name == "cursor").unwrap();
        assert_eq!(cursor.skill_dirs, ["~/.agents/skills"]);
        assert!(cursor.legacy_skill_dirs.contains(&"~/.cursor/skills"));
        assert!(matches!(cursor.mcp, McpConfig::CursorJson("~/.cursor/mcp.json")));
    }
}
