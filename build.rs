//! Stamps the git commit into the binary: `redline --version` → `redline 0.2.0 (1a2b3c4d5e)`.
//! `REDLINE_GIT_SHA` overrides (release builds, tarballs without .git); "unknown" if neither works.

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=REDLINE_GIT_SHA");
    for p in [".git/HEAD", ".git/refs/heads", ".git/packed-refs"] {
        println!("cargo:rerun-if-changed={p}");
    }
    let sha = std::env::var("REDLINE_GIT_SHA").ok().filter(|s| !s.is_empty()).or_else(git_sha);
    println!("cargo:rustc-env=REDLINE_GIT_SHA={}", sha.as_deref().unwrap_or("unknown"));
}

fn git_sha() -> Option<String> {
    let out = Command::new("git").args(["rev-parse", "--short=10", "HEAD"]).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}
