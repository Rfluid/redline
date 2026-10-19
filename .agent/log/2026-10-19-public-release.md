# 2026-10-19 · public-release

**Agent:** claude  **Roadmap:** none (release prep)

## Done
- Squashed the repo into one first commit for the public release: version 0.1.0, one `CHANGELOG.md` entry, ROADMAP reduced to open work (renumbered P1.1–P2.3; DECISIONS drafts follow the new IDs).
- Dropped earlier session logs and the one-shot `implement-roadmap` prompt; their lasting gotchas are in `workflows/implement-task.md`, and the agents' config doc URLs are in `src/install/mcp_config/tests.rs`.
- Folded in the CI fixes that were pending: newer Actions pins, release dry-run concurrency per tag, and the harness waiting for Chrome to exit before removing its profile.
- Fixed Windows auto-start hanging callers: `Command::spawn` passes every inheritable handle, so the daemon held the caller's stdout pipe open (`$(redline register)` never returned; CI's Windows e2e hung for hours). The daemon now starts through `Start-Process` and logs to `REDLINE_LOG_FILE` (`src/lifecycle.rs`, `src/daemon/mod.rs`, DECISIONS "Windows").
- `ci.yml`: the Windows `jq` step runs under `pwsh` (actionlint parsed it as bash).
- Browser harness: waits up to 30s for Chrome's DevTools port (a cold start on the Windows runner took over 10s) and reports Chrome's stderr when it never opens.

## Verified
- `just check` green; clippy clean for `x86_64-pc-windows-msvc` (cargo xwin). CI green on Linux, macOS and Windows (the Windows e2e no longer hangs).

## Open / next
- ROADMAP P1.1 (clean-install test) and P1.3 (Windows on a real machine).
- Dependabot `ureq` 3 needs code changes in `src/api` and `src/lifecycle.rs` (the API changed).
