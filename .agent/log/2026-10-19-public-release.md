# 2026-10-19 · public-release

**Agent:** claude  **Roadmap:** none (release prep)

## Done
- Squashed the repo into one first commit for the public release: version 0.1.0, one `CHANGELOG.md` entry, ROADMAP reduced to open work (renumbered P1.1–P2.3; DECISIONS drafts follow the new IDs).
- Dropped earlier session logs and the one-shot `implement-roadmap` prompt; their lasting gotchas are in `workflows/implement-task.md`, and the agents' config doc URLs are in `src/install/mcp_config/tests.rs`.
- Folded in the CI fixes that were pending: newer Actions pins, release dry-run concurrency per tag, and the harness waiting for Chrome to exit before removing its profile.

## Verified
- `just check` green.

## Open / next
- Make the repo public, then apply `.github/rulesets/main.json` and private vulnerability reporting (`docs/DEVELOPMENT.md`).
