---
name: redline-dev
description: Develop the redline codebase (Rust daemon + MCP server + CLI + browser client). Use when working inside the redline repo on features, fixes, tests or docs. Sets up a sandboxed dev loop and points to the repo's workflows and rules.
---

# redline-dev

You are working **on** redline, not using it. The source of truth is `.agent/` at the repo root.

1. Read `.agent/context.md` and `.agent/rules.md`.
2. **Sandbox:** every manual run uses `REDLINE_HOME="$(mktemp -d)"` and `REDLINE_PORT=7499`. Never touch `~/.local/state/redline`. Never run `install`/`uninstall` without asking.
3. **Inner loop:**
   - `cargo check`
   - `cargo test <module>` for a focused run
   - `just dev` (foreground daemon in a temp dir)
   - `just check` before you finish
4. **Pick the workflow** in `.agent/workflows/` that matches the task (implement-task, add-mcp-tool, add-agent-support, add-migration, manual-browser-test, self-review).
5. **Conventions** are in `docs/CODE_STYLE.md`; contracts are in `docs/PROTOCOL.md`. Update the docs in the same change (sync table in `rules.md`).
6. **End with** a `.agent/log/` entry.

To make this skill discoverable by an agent that loads project skills, symlink this folder into that agent's project-skills dir (e.g. `.claude/skills/redline-dev`). Ask the user before adding agent-specific config.
