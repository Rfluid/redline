//! Runs `git` with no terminal: it never prompts for a password or a host key, and gives up after a
//! timeout. Update checks run in the background, where a prompt would hang forever.

use anyhow::{Context, Result, bail};
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const POLL: Duration = Duration::from_millis(50);

/// `git -C <repo> <args>`; stdout on success, stderr in the error otherwise.
pub fn git(repo: &Path, args: &[&str], timeout: Duration) -> Result<String> {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(repo).args(args).env("GIT_TERMINAL_PROMPT", "0");
    if std::env::var_os("GIT_SSH_COMMAND").is_none() && std::env::var_os("GIT_SSH").is_none() {
        cmd.env("GIT_SSH_COMMAND", "ssh -o BatchMode=yes -o ConnectTimeout=10");
    }
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("running git (is it installed?)")?;
    // Drain both pipes on threads, so a chatty git can't stall on a full pipe while we poll.
    let out = drain(child.stdout.take());
    let err = drain(child.stderr.take());
    let deadline = Instant::now() + timeout;
    let status = loop {
        if let Some(s) = child.try_wait()? {
            break s;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            // Don't join the readers: an ssh child may still hold the pipes open.
            bail!("git {} timed out after {}s", args.first().unwrap_or(&""), timeout.as_secs());
        }
        thread::sleep(POLL);
    };
    let out = out.join().unwrap_or_default();
    let err = err.join().unwrap_or_default();
    if !status.success() {
        bail!("git {}: {}", args.join(" "), err.trim());
    }
    Ok(out)
}

fn drain(pipe: Option<impl Read + Send + 'static>) -> thread::JoinHandle<String> {
    thread::spawn(move || {
        let mut s = String::new();
        if let Some(mut p) = pipe {
            let _ = p.read_to_string(&mut s);
        }
        s
    })
}
