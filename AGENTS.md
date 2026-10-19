# AGENTS.md

**The source of truth for agents working on this repo is [`.agent/`](.agent/README.md).** Read `.agent/README.md` first, then follow it.

Short version:
1. Read `.agent/context.md` (what redline is, invariants) and `.agent/rules.md` (hard rules).
2. Work from `docs/ROADMAP.md`, using `.agent/workflows/implement-task.md`.
3. `just check` must be green before you say "done".
4. Write a `.agent/log/` entry at the end.

Never touch `~/.local/state/redline`; use `REDLINE_HOME=$(mktemp -d)`. Never run `redline install`/`uninstall`, commit or push without the user's OK.

(`SKILL.md` and `INSTALL.md` in this folder are for agents *using* redline, not developing it.)
