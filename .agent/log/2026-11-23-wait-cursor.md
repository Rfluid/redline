# 2026-11-23 · wait-cursor

**Agent:** claude (Nexus desktop)  **Roadmap:** none (user request after a manual test)

## Done
- Manual test in the Nexus desktop browser (sandboxed daemon, `examples/demo.html`). The user's comments reached the daemon but not the agent, twice. Causes: (1) the session had no redline MCP tools, because Nexus desktop runs Claude Code with its own `CLAUDE_CONFIG_DIR`, whose `.claude.json` had no `redline` server; (2) Nexus's terminal tool doesn't notify on command exit, so a background `redline wait` can't wake the agent either; (3) a bare `redline wait` replayed every comment because `--since` defaulted to 0.
- Fix for (3): `redline wait` saves `next` per `--artifact`/`--session` filter in `<state>/cursors/` and starts there when `--since` is omitted (`src/cli/cursor.rs`). A saved cursor past the log head (log reset) starts over at 0. Docs: PROTOCOL (CLI), ARCHITECTURE (delivery table), SKILL (background recipe).

## Verified
- Unit tests in `src/cli/cursor.rs`; `tests/e2e.sh` step "bare wait resumes from the saved cursor". `just check`.

## Open / next
- (1) is install-side: `redline install --claude-config-dir <dir>` per Nexus desktop account fixes it. `install` could detect those accounts itself (`~/.local/share/xyz.nexus.desktop/accounts/claude/*` on Linux), but that's agent-specific detection for a host app; not done.
- (2) is covered by a SKILL note ("When nothing can wake you") and by the dock's agent-listening indicator, in separate PRs.

## Gotchas
- Run `cargo build --release` before `bash tests/e2e.sh` (or use `just e2e`). Otherwise `bin/redline` rebuilds during the first call and the test fails with "daemon did not start".
- zsh's `echo` turns `\n` in the register JSON into real newlines and breaks `jq`. Use `printf '%s'` (the manual-browser-test workflow already says so).
