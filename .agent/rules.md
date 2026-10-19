# Rules

## Safety

- **Never touch the user's real redline state.** Use `REDLINE_HOME="$(mktemp -d)"` and a non-default `REDLINE_PORT` for anything you run by hand, plus `REDLINE_CONFIG="$REDLINE_HOME/config.toml"` for anything that touches update settings (`redline update skip|off|on` write `~/.config/redline/config.toml` otherwise). Never run a bare `redline update`: it moves the user's clone.
- **Never run `redline install` / `uninstall` without the user's explicit OK.** They edit the user's agent configs (`~/.claude*`, `~/.codex`, `~/.cursor`, `~/.gemini`, opencode).
- **Never commit or push unless the user asks.** Conventional Commits when you do. PRs merge with rebase only, so every commit lands on main: each one self-contained and passing `just check`, no merge or `fixup!` commits (`pr.yml` checks).
- **Never weaken auth or the loopback bind** to make something easier to test.

## Quality gates (all must pass before you say "done")

```sh
just check     # cargo fmt --check · clippy -D warnings (pedantic) · node --check · cargo test · e2e · browser
```

- **Behaviour change:** add or adjust a unit test (in the module), `tests/e2e.sh` (cross-process) or `tests/browser/overlay.test.mjs` (overlay/client.js).
- **Lints:** no new `#[allow(...)]` without a comment saying why. Prefer fixing the lint.
- **File size:** split any source file that grows past ~250 lines (see `docs/CODE_STYLE.md`).

## Docs stay in sync (same change, not later)

| You changed | Update |
|---|---|
| An HTTP route, WS frame, MCP tool, CLI flag or `client.js` API | `docs/PROTOCOL.md` |
| Component boundaries, the data model, delivery or security | `docs/ARCHITECTURE.md` |
| A dependency or a design trade-off | `docs/DECISIONS.md` |
| A pattern or convention | `docs/CODE_STYLE.md` |
| How redline looks (overlay, bindings' UI, logo, copy) | `.design/` (tokens in both `tokens.css` and `tokens.json`; run `python3 .design/check.py`) |
| Something user-visible | `README.md` (user language, no internals) |
| How agents use redline | `SKILL.md` |
| How agents install redline | `INSTALL.md` |
| Finished a roadmap task | tick it in `docs/ROADMAP.md` |

## Scope

- **Do the task asked.** Note adjacent problems in your log entry or as a new ROADMAP item instead of fixing them silently.
- **Ask the user** when a choice changes the user-facing behaviour, the protocol or security, and the docs don't settle it.
