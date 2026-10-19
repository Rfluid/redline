# Implementation roadmap

This is the plan agents work from. **Take the first unchecked task in the lowest open phase,** unless the user names one. Follow [`.agent/workflows/implement-task.md`](../.agent/workflows/implement-task.md). When a task is done, tick it (`[x]`) and add a log entry. Add newly discovered work as unchecked tasks in the right phase.

Each task has an **ID** (for prompts and logs) and **Done when** criteria. A task isn't done until its criteria hold and `just check` is green.

Legend: 🔒 needs the user's explicit OK (it touches their machine, accounts or configs, or the public surface).

---
## Phase 1: Verification on real setups

- [ ] **P1.1 🔒 Clean-install test.** After the user installs redline with the README sentence, an agent runs [`.agent/prompts/test-clean-install.md`](../.agent/prompts/test-clean-install.md): static checks, `tests/agent-loop.sh` for every agent, the browser checklist, skills, `strip`, cleanup.
  *Done when:* its report shows every installed agent passing (or failing only for a documented setup issue), and any redline problems it found are fixed or added here.
- [ ] **P1.2 Verify Claude Code channels live.** `redline mcp --channel` follows the current channels reference (string `meta`, capability only with `--channel`, protocol ≤ 2025-11-25, push starts at head), and e2e checks the notification on stdio. Still unverified in a live session: Team plans block channels by org policy unless an Owner enables them (`channelsEnabled`), and `--mcp-config` servers aren't accepted as `server:<name>` (a project `.mcp.json` or `claude mcp add` is). Retry on a personal (Pro/Max) login or with channels enabled; fix `src/mcp/mod.rs` (`CHANNEL_*`, `push_loop`) if needed, and document the setup in `INSTALL.md` (optional step) and `SKILL.md`.
  *Done when:* with channels on, a submitted comment shows up in a live Claude session without any tool call.
- [ ] **P1.3 🔒 Windows on a real machine.** CI covers Windows (unit, e2e through `bin/redline.ps1`, browser) and releases build the x86_64 and arm64 zips; a real install hasn't been checked yet.
  *Done when:* an install on a real Windows machine, with and without cargo, passes `redline doctor` and a review loop in one agent.

## Phase 2: Later (needs design first; write a DECISIONS entry before building)

- [ ] **P2.1** Browser extension as an overlay host, so no embed is needed (draft in `docs/DECISIONS.md`).
- [ ] **P2.2** Remote relay: review from a phone or by a teammate. Needs auth, TLS and a threat model (draft in `docs/DECISIONS.md`).
- [ ] **P2.3** Adapter for claude.ai Artifacts' native comments (draft in `docs/DECISIONS.md`).
