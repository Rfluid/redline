# Workflow: implement a roadmap task

1. **Orient.**
   - Read `.agent/context.md`, `.agent/rules.md` and the newest `.agent/log/` entry.
   - Run `just check`, so you know the baseline is green. If it isn't, fix that first or report it.
2. **Pick.** Take the first unchecked task in the current phase of `docs/ROADMAP.md`, unless the user named one. Read its *Done when* criteria.
3. **Read the code it touches,** and the matching section of `docs/ARCHITECTURE.md` / `docs/PROTOCOL.md`.
4. **Plan briefly.** List the files you'll change and the tests you'll add. If the plan breaks an invariant in `context.md`, stop and ask.
5. **Implement.**
   - Follow `docs/CODE_STYLE.md`.
   - Small steps. Run `cargo check` / `cargo test <module>` as you go.
   - Use a matching workflow if there is one (`add-mcp-tool`, `add-migration`, …).
6. **Test.**
   - Unit tests in the module.
   - `tests/e2e.sh` for anything crossing processes.
   - `workflows/manual-browser-test.md` for UI.
7. **Docs.** Use the sync table in `rules.md`.
8. **Gate.** `just check` must be green, then run `workflows/self-review.md`.
9. **Close.**
   - Tick the task in `docs/ROADMAP.md`. Add follow-ups as new unchecked items.
   - Write a `.agent/log/` entry.
   - Report to the user: what changed, how it was verified, anything left open.

## Gotchas

- Output-filtering shell proxies (such as `rtk`) can hide clippy errors ("No issues found"). Run `just check` unfiltered when lint results matter.
- The toolchain is pinned in `rust-toolchain.toml`. Bump it only on purpose.
- `opencode run` / `opencode mcp list` start a persistent `opencode serve --service`, which keeps the old MCP config. Kill it after config changes and after tests.
- With `agy`, put flags before `-p`.
- zsh `echo` mangles `\n` inside JSON, so use `printf '%s'`.
- `set_var` is unsafe in edition 2024 and `unsafe_code` is forbidden, so env-dependent logic takes an injected lookup (like `expand_with` in `install/agents.rs`) for tests.
- `pkill -f <pattern>` run from an agent's shell tool can match that shell's own command line and kill it. Stop sandbox daemons with `redline stop` and kill other servers by PID.
- Overlay: `ShadowRoot` has no `insertAdjacentHTML` (use a `<template>`), and the document-level `::highlight()` sheet can't see the shadow host's `data-theme` (rebuild the adopted sheet when the theme changes).
- Windows locks running exes, so cargo can't relink a binary an agent is running; that's why `bin/redline.ps1` runs a copy in `target\bin`.
