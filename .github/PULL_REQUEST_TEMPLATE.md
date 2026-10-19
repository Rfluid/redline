<!--
This repo merges with "Rebase and merge" only: every commit in this PR lands on main as is.
Keep each commit a Conventional Commit (`feat(overlay): …`, `fix(mcp): …`) that builds and passes on its own.
Clean up fixups before review is done (`git rebase -i main`, then `git push --force-with-lease`).
-->

## What

<!-- What changes, in a few bullets. Link the issue: "Closes #123". -->

## Why

<!-- The problem it solves, or the roadmap task (P1.1). -->

## How I checked

<!-- Tests added or changed; anything checked by hand (browser, agent, OS). -->

- [ ] `just check` passes (fmt, clippy, JS syntax, design check, unit, e2e, browser)
- [ ] A test covers the behaviour change (unit, `tests/e2e.sh`, or `tests/browser/`)

## Docs

<!-- Tick what applies, or delete the list if nothing user- or contributor-facing changed. -->

- [ ] `CHANGELOG.md` (`[Unreleased]`) for anything users notice
- [ ] `docs/PROTOCOL.md` for an HTTP route, WS frame, MCP tool, CLI flag or `client.js` API
- [ ] `docs/ARCHITECTURE.md` / `docs/DECISIONS.md` for boundaries, data model, security, a dependency or a trade-off
- [ ] `README.md` / `SKILL.md` / `INSTALL.md` for how users or agents use and install redline
- [ ] `.design/` (and `python3 .design/check.py`) for how redline looks
- [ ] `docs/ROADMAP.md` task ticked
