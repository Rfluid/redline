//! Command-line interface: argument types and dispatch. The commands' bodies live in `commands`.

mod commands;

use crate::api::Api;
use crate::model::{Artifact, RegisterArtifact};
use crate::{daemon, install, lifecycle, mcp, paths, update};
use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use commands::{daemon_status, print, strip_files, update_cmd, update_field};
use serde_json::json;
use std::path::PathBuf;
use std::time::Duration;

const READ_TIMEOUT: Duration = Duration::from_secs(10);
/// Grace period for MCP tasks once the client disconnects.
const MCP_SHUTDOWN: Duration = Duration::from_millis(200);

/// `0.2.0 (1a2b3c4d5e)`: the crate version and the commit it was built from (see build.rs).
pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (", env!("REDLINE_GIT_SHA"), ")");

/// redline: inline review comments between artifacts and coding-agent sessions.
#[derive(Parser)]
#[command(version = VERSION, about)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run the local broker daemon (started automatically when needed).
    Serve {
        #[arg(long, env = "REDLINE_PORT", default_value_t = paths::DEFAULT_PORT)]
        port: u16,
    },
    /// Run the MCP server on stdio (what agents launch).
    Mcp {
        /// Also push comments into the session via Claude Code channels.
        #[arg(long, env = "REDLINE_CHANNEL")]
        channel: bool,
    },
    /// Register an artifact; prints its id and the embed snippet.
    Register {
        title: String,
        #[arg(long)]
        uri: Option<String>,
        #[arg(long)]
        kind: Option<String>,
        #[arg(long)]
        id: Option<String>,
        #[arg(long, env = "REDLINE_SESSION")]
        session: Option<String>,
    },
    /// Block until new user comments arrive (for agents without MCP, or background shells).
    Wait {
        #[arg(long)]
        artifact: Option<String>,
        #[arg(long, env = "REDLINE_SESSION")]
        session: Option<String>,
        #[arg(long, default_value_t = 0)]
        since: i64,
        #[arg(long, default_value_t = 300)]
        timeout: u64,
    },
    /// List artifacts, or the threads of one artifact.
    List { artifact: Option<String> },
    /// Reply inline on a thread.
    Reply {
        thread: String,
        body: String,
        #[arg(long)]
        status: Option<String>,
    },
    /// Without arguments: daemon port, pid, version, uptime and counts.
    /// With `<thread> <status>`: set a thread's status (open, working, answered, resolved).
    Status {
        #[arg(requires = "status")]
        thread: Option<String>,
        status: Option<String>,
        /// Progress note shown under the status (with `<thread> <status>`).
        #[arg(long, requires = "thread")]
        note: Option<String>,
    },
    /// Stop the daemon. Any later command starts it again.
    Stop,
    /// Save a region comment's screenshot (anchor `image`) to a file.
    Image {
        attachment: String,
        /// Output path (default: `<attachment>.<ext>` in the current dir).
        #[arg(long, short)]
        out: Option<PathBuf>,
    },
    /// Remove redline embed snippets from files or directories (do this before shipping).
    Strip {
        #[arg(required = true)]
        paths: Vec<PathBuf>,
        /// Only report; exit 1 if any snippet is found.
        #[arg(long)]
        check: bool,
    },
    /// Print the embed snippet for an artifact.
    Snippet { artifact: String },
    /// Link the skill and register the MCP server for coding agents.
    Install(InstallArgs),
    /// Undo `install`. Leaves the comment database alone.
    Uninstall(InstallArgs),
    /// Check this device's setup.
    Doctor(InstallArgs),
    /// Update to the latest release, or manage update notices.
    Update {
        #[command(subcommand)]
        action: Option<UpdateCmd>,
    },
}

#[derive(Subcommand)]
pub(crate) enum UpdateCmd {
    /// Check for a new release now and print what's known.
    Check,
    /// Stop notices for a release (default: the latest). A newer release still shows.
    Skip { version: Option<String> },
    /// Turn update checks and notices off.
    Off,
    /// Turn update checks and notices back on.
    On,
}

#[derive(Args)]
struct InstallArgs {
    /// all (default: detected) or a comma list: claude,codex,opencode,cursor,agy,gemini
    #[arg(long)]
    agents: Option<String>,
    /// Extra Claude config dir (a second profile); repeatable.
    #[arg(long)]
    claude_config_dir: Vec<PathBuf>,
    /// Extra skills dir to link into; repeatable.
    #[arg(long)]
    skill_dir: Vec<PathBuf>,
    #[arg(long, default_value = "~/.local/bin")]
    bin_dir: String,
    #[arg(long)]
    force: bool,
    /// Only link the skill and CLI; don't touch agent MCP configs.
    #[arg(long)]
    no_mcp: bool,
}

impl InstallArgs {
    fn into_opts(self) -> install::Opts {
        install::Opts {
            agents: self.agents,
            claude_config_dirs: self.claude_config_dir,
            skill_dirs: self.skill_dir,
            bin_dir: install::expand(&self.bin_dir),
            force: self.force,
            no_mcp: self.no_mcp,
        }
    }
}

/// Parses `std::env::args` and runs the chosen subcommand. People at a terminal get an update notice
/// on stderr afterwards (agents get it in `register`'s output instead).
pub fn run() -> Result<()> {
    use std::io::IsTerminal as _;
    let cmd = Cli::parse().cmd;
    let notify =
        !matches!(cmd, Cmd::Serve { .. } | Cmd::Mcp { .. } | Cmd::Update { .. }) && std::io::stderr().is_terminal();
    let res = dispatch(cmd);
    if notify && let Some(n) = update::notice(&update::info(&update::Files::default())) {
        eprintln!("{n}");
    }
    res
}

fn dispatch(cmd: Cmd) -> Result<()> {
    match cmd {
        Cmd::Serve { port } => tokio::runtime::Runtime::new()?.block_on(daemon::serve(port)),
        Cmd::Mcp { channel } => {
            let rt = tokio::runtime::Runtime::new()?;
            let res = rt.block_on(mcp::run(channel));
            // The client hung up. Don't wait for in-flight blocking long-polls (up to minutes, and they
            // reconnect, which would restart a daemon nobody needs): exit now.
            rt.shutdown_timeout(MCP_SHUTDOWN);
            res
        }
        Cmd::Register { title, uri, kind, id, session } => {
            let api = Api::connect()?;
            let a = api.register(&RegisterArtifact { id, title, uri, kind, session })?;
            let since = api.status()?.head;
            let mut v = json!({ "artifact_id": a.id, "embed": api.embed(&a), "session": a.session, "since": since });
            if let Some(u) = update_field() {
                v["update"] = u;
            }
            print(&v)
        }
        Cmd::Wait { artifact, session, since, timeout } => {
            let (events, next) =
                Api::connect()?.wait_through_restarts(since, timeout, artifact.as_deref(), session.as_deref())?;
            print(&json!({ "events": events, "next": next }))
        }
        Cmd::List { artifact: None } => print(&Api::connect()?.get("/api/artifacts", READ_TIMEOUT)?),
        Cmd::List { artifact: Some(a) } => print(&Api::connect()?.threads(&a)?),
        Cmd::Reply { thread, body, status } => print(&Api::connect()?.reply(&thread, &body, status.as_deref())?),
        Cmd::Status { thread: Some(thread), status: Some(status), note } => {
            print(&Api::connect()?.set_status(&thread, &status, note.as_deref())?)
        }
        Cmd::Status { .. } => daemon_status(),
        Cmd::Stop => print(&json!({ "stopped": lifecycle::stop()? })),
        Cmd::Image { attachment, out } => {
            let (mime, data) = Api::connect()?.attachment(&attachment)?;
            let ext = mime.strip_prefix("image/").unwrap_or("bin");
            let path = out.unwrap_or_else(|| PathBuf::from(format!("{attachment}.{ext}")));
            std::fs::write(&path, &data)?;
            print(&json!({ "path": path, "mime": mime, "bytes": data.len() }))
        }
        Cmd::Strip { paths, check } => strip_files(&paths, check),
        Cmd::Snippet { artifact } => {
            let api = Api::connect()?;
            let a: Artifact = serde_json::from_value(api.get(&format!("/api/artifacts/{artifact}"), READ_TIMEOUT)?)?;
            println!("{}", api.embed(&a));
            Ok(())
        }
        Cmd::Install(a) => install::install(&a.into_opts()),
        Cmd::Uninstall(a) => install::uninstall(&a.into_opts()),
        Cmd::Doctor(a) => install::doctor(&a.into_opts()),
        Cmd::Update { action } => update_cmd(action),
    }
}
