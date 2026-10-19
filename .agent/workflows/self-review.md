# Workflow: self-review before "done"

Go through the diff (`git diff`) with these questions:

- [ ] **Correctness.**
  - Error paths return useful messages (`.context(...)`).
  - Nothing panics on user input.
  - No `unwrap` outside tests.
- [ ] **Invariants** (`.agent/context.md`).
  - Communication only.
  - Loopback only.
  - Token scopes intact.
  - One event per write.
  - MCP tool ↔ CLI twin.
- [ ] **Async hygiene.**
  - No `std::sync::Mutex` guard held across `.await`.
  - Blocking I/O runs via `spawn_blocking`.
- [ ] **Security.**
  - New routes check auth.
  - Artifact tokens can't reach other artifacts.
  - Nothing logs tokens.
- [ ] **Tests.** New behaviour is covered: unit, e2e, or a manual browser check noted in the log.
- [ ] **Style** (`docs/CODE_STYLE.md`).
  - Module header `//!` present.
  - Files under ~250 lines.
  - Constants for magic numbers.
  - Names say what, not how.
- [ ] **Docs** synced (table in `.agent/rules.md`).
  - README still user-facing.
  - PROTOCOL matches the code.
- [ ] **`just check` green.**
