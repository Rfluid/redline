# Contributing to redline

Thanks for helping. Bug reports, agent-support requests, docs fixes and code are all welcome.

- **Questions and ideas:** [Discussions](https://github.com/Rfluid/redline/discussions).
- **Bugs and feature requests:** [open an issue](https://github.com/Rfluid/redline/issues/new/choose). For anything bigger than a small fix, open an issue before the PR so we can agree on the approach.
- **Security issues:** never in public. See [SECURITY.md](SECURITY.md).

By taking part you agree to the [code of conduct](CODE_OF_CONDUCT.md).

## Setup

You need Rust (the version in `rust-toolchain.toml` installs itself through rustup), [`just`](https://github.com/casey/just), Node 22, `jq`, Python 3, and Chrome or Chromium for the browser tests.

```sh
git clone https://github.com/Rfluid/redline && cd redline
just check     # everything CI runs: fmt, clippy, JS syntax, design check, unit, e2e, browser
just dev       # a sandboxed daemon on :7499 that never touches your real redline state
```

When you run redline by hand, sandbox it: `REDLINE_HOME="$(mktemp -d)"` and a non-default `REDLINE_PORT`. [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) has the layout, the test suites and the release process.

## Before you write code

Read [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) and the invariants in [.agent/context.md](.agent/context.md). The short version: redline only carries comments; it never renders or serves the artifact, the daemon binds `127.0.0.1` only, and every MCP tool has a CLI twin. A change that breaks an invariant needs a [DECISIONS.md](docs/DECISIONS.md) entry and agreement in the issue first.

Code conventions are in [docs/CODE_STYLE.md](docs/CODE_STYLE.md); visual changes follow [.design/](.design/README.md). Coding agents start at [.agent/README.md](.agent/README.md).

## Pull requests

This repo merges with **Rebase and merge** only. Your commits land on `main` as they are, so:

- **Each commit is a [Conventional Commit](https://www.conventionalcommits.org/):** `<type>(<scope>): <summary>`, type one of `feat fix docs style refactor perf test build ci chore revert`. A PR check enforces it. Look at `git log` for scopes (`overlay`, `mcp`, `daemon`, `install`, …).
- **Each commit stands on its own** and passes `just check`. Fold fixups in before merge (`git commit --fixup`, then `git rebase -i --autosquash main`).
- **No merge commits.** Update your branch with `git rebase main`, then `git push --force-with-lease`.

In the PR:

- A test for the behaviour change: a unit test in the module, `tests/e2e.sh`, or `tests/browser/`.
- Docs updated in the same PR (the PR template lists which doc covers what), and a line in `CHANGELOG.md` under `[Unreleased]` for anything users notice.
- CI green. Dependency changes also run `cargo deny` (`just deny` locally): new dependencies need a license allowed in `deny.toml`.

## Releases

Maintainers cut releases from tags; see [docs/DEVELOPMENT.md#releases](docs/DEVELOPMENT.md#releases).
