# Development

## Layout

```
src/main.rs                 thin binary: redline::cli::run()
src/lib.rs                  module tree
src/cli.rs                  clap commands and dispatch
src/daemon/                 axum: mod.rs (App, router, serve) · auth.rs · handlers.rs · live.rs · tests.rs
src/store/                  Diesel: mod.rs (Store API) · rows.rs · schema.rs · tests.rs
src/mcp/                    rmcp: mod.rs (server, channel push) · tools.rs
src/install/                mod.rs · agents.rs (agent table) · link.rs · mcp_config.rs (+ tests.rs)
src/api.rs                  blocking agent-side client
src/lifecycle.rs            find, start, replace (version handshake) and stop the daemon
src/model.rs                wire types
src/strip.rs                `redline strip`: remove embed snippets
src/paths.rs, src/ids.rs    state dir/tokens; ids, tokens, timestamps
migrations/                 Diesel SQL migrations (embedded)
web/anchors.js, client.js   anchors + protocol client, served together as /client.js
web/overlay-style.js, regions.js, markdown.js, overlay.js
                            styles (light/dark/phone), region picker/pins, safe markdown, comment UI: served together as /overlay.js
tests/e2e.sh                end-to-end test
tests/browser/              headless-Chrome tests (no npm deps): overlay, Streamlit binding
bindings/streamlit/         Python package + dependency-free component frontend; example_app.py
bindings/react/             redline-store.js (framework-agnostic) + useRedline.js; example.html (React via CDN + htm)
tests/bindings/             node:test unit tests: the React store, web/markdown.js (incl. XSS payloads)
bin/redline                 wrapper: rebuilds the release binary when sources are newer, or downloads a release binary without cargo
bin/redline.ps1, .cmd       the Windows twin (Windows PowerShell 5.1), and its cmd launcher; keeps target\bin\redline.exe current
build.rs                    stamps the git sha into --version
scripts/prepare-release.sh  bumps every crate + CHANGELOG for the next release (just prepare-release)
scripts/checkout-release.sh moves a clone to the latest release tag (install and update); .ps1 is the Windows twin
.github/workflows/          ci.yml (PRs to main, main, manual, called by release), release.yml (tags vX.Y.Z[-pre]), audit.yml (cargo-deny: dependency changes, daily), pr.yml (Conventional Commits, actionlint)
.github/dependabot.yml      weekly dependency PRs (cargo, GitHub Actions)
.github/ISSUE_TEMPLATE/     issue forms: bug, feature, agent support; PULL_REQUEST_TEMPLATE.md; CODEOWNERS
.github/rulesets/main.json  branch ruleset for main, to import once the repo is public (see Repository settings)
CONTRIBUTING.md, SECURITY.md, CODE_OF_CONDUCT.md, LICENSE   community files
deny.toml                   cargo-deny policy: advisories, allowed licenses, sources
CHANGELOG.md                release notes (Keep a Changelog)
examples/demo.html          page for manual overlay testing
.agent/                     workspace for coding agents developing redline
```

`web/*.js` are compiled into the binary with `include_str!`. Rebuild after editing them; `bin/redline` does that automatically.

## Tooling

```sh
just            # list recipes
just fmt        # cargo fmt
just lint       # fmt --check, clippy -D warnings (pedantic), node --check web/*.js, .design/check.py
just test       # cargo test (unit)
just e2e        # release build + tests/e2e.sh
just browser    # release build + headless-Chrome overlay test
just screenshots # release build + regenerate the README screenshots in docs/screenshots/
just check      # lint + test + e2e + browser: what CI runs (.github/workflows/ci.yml)
just deny       # cargo-deny (deny.toml): advisories, licenses, sources (cargo install cargo-deny)
just dev        # foreground daemon in a temp REDLINE_HOME on :7499
```

On Windows, `just` runs recipes with `sh`: have Git for Windows' `bin` dir on PATH (GitHub's runners do). `just lint` needs `python3`; use the Linux job for it.

From Linux, check the Windows build (clippy, and the unit tests under wine) without a Windows machine. Wine has no junctions, so the two link tests fail there; CI covers them on real Windows:

```sh
cargo install cargo-xwin --locked && rustup target add x86_64-pc-windows-msvc   # needs clang-cl, lld-link
cargo xwin clippy --target x86_64-pc-windows-msvc --all-targets -- -D warnings
CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUNNER=wine cargo xwin test --target x86_64-pc-windows-msvc --lib
```

Conventions and patterns: [CODE_STYLE.md](CODE_STYLE.md).

## Build and run

```sh
cargo build --release
export REDLINE_HOME=$(mktemp -d) REDLINE_PORT=7499   # isolated sandbox; never touches your real state
export REDLINE_CONFIG=$REDLINE_HOME/config.toml REDLINE_NO_UPDATE_CHECK=1   # and not your config.toml
bin/redline serve                                    # foreground daemon, or let any command auto-start it
bin/redline register "Demo" --session dev            # prints artifact_id + embed
```

To try the overlay:
1. Copy `examples/demo.html`.
2. Replace the `redline:embed` comment with the `embed` value.
3. Serve the file (`python3 -m http.server`) and open it.
4. Act as the agent with `bin/redline wait --session dev` and `bin/redline reply <thread> "…"`.

Use `printf '%s'`, not `echo`, when piping JSON in zsh: zsh's `echo` expands the `\n` escapes inside the embed string.

## Tests

```sh
bash tests/e2e.sh
```

It uses a temp `REDLINE_HOME` and a random port, and needs `jq` and `curl`. It covers:
- daemon auto-start and register
- static JS
- batch submit → long-poll wake-up
- agent reply visible to the artifact
- agent events filtered from agent waits
- MCP `initialize` / `tools/list` / `tools/call` over stdio
- two concurrent MCP sessions (FIFO-driven `redline mcp`): no cross-delivery, independent cursors
- `status` → `stop` → auto-restart
- the version handshake: touching the release binary makes the next call replace the daemon

rmcp requires a spec-complete `initialize` (`capabilities` and `clientInfo`); a bare one makes the server exit.

Unit tests (`just test`) cover the daemon's HTTP API in-process (`src/daemon/tests.rs`: the auth matrix, mixed and all-or-nothing submits, long-poll wake-up and timeout), the store (status rules, event filters, re-attach), the install config editors (JSON/TOML, backups, JSONC refusal), symlinks, agent selection and ids.

### Browser test

```sh
just browser        # release build + node tests/browser/overlay.test.mjs
```

It runs `overlay.js` + `client.js` in headless Chrome against a sandboxed daemon, with no npm dependencies: `tests/browser/harness.mjs` drives Chrome over the DevTools protocol (Node 22 has `fetch` and `WebSocket`). It covers select → Comment → Send, the highlight and margin marker, the live agent reply, drafts sent from the dock, reload, resolve/undo, the drawer, the error toast, the page-derived theme, the phone sheet, and a clean console. It needs Chrome on PATH (`google-chrome`, `chromium`, …) or `CHROME=/path/to/chrome`. `tests/browser/streamlit.test.mjs` runs `bindings/streamlit/example_app.py` with `streamlit run` and does the loop through the component iframes; it skips when `python3 -c 'import streamlit'` fails (as in CI). `tests/browser/react.test.mjs` runs `bindings/react/example.html` (React from esm.sh) and skips when the CDN is unreachable. Add new browser steps to `overlay.test.mjs`; in-page helpers (`$rl`, `select`, `comment`, `openThread`, `inPop`, `hlAll`) are defined at its top.

## Releases

Users run releases, not main: `INSTALL.md` clones, then `scripts/checkout-release.sh` checks out the highest `vX.Y.Z` tag (prereleases only when named). Updating is the same script.

1. `just prepare-release` (or `scripts/prepare-release.sh --bump minor`, `--pre beta`, `--version X.Y.Z`, `--dry-run`). It fetches tags, takes the latest stable tag, bumps it (semver; before 1.0 a minor bump may change the protocol), sets that version on every workspace crate (plus `[workspace.package]` and member entries in `[workspace.dependencies]` if they appear), refreshes `Cargo.lock`, and for a stable release moves the `[Unreleased]` notes in `CHANGELOG.md` under the new version and updates the compare links. A prerelease leaves the changelog alone. `bindings/streamlit` has its own version and isn't touched.
2. `just check`, commit (`chore(release): vX.Y.Z`), merge to main through a PR (CI must pass).
3. `git tag -a vX.Y.Z -m vX.Y.Z && git push origin vX.Y.Z`. `.github/workflows/release.yml` checks the tag matches `Cargo.toml`, runs `ci.yml` on the tagged commit, builds `redline-X.Y.Z-<target>.tar.gz` (+ `.sha256`) for `x86_64`/`aarch64-unknown-linux-gnu` (zig, glibc 2.28) and `aarch64`/`x86_64-apple-darwin`, plus `redline-X.Y.Z-<target>.zip` for `x86_64`/`aarch64-pc-windows-msvc`, smoke-tests `--version`, and creates the GitHub Release with that version's changelog section.
   - `vX.Y.Z` is marked **latest**, unless a higher `vX.Y.Z` exists (a backport).
   - `vX.Y.Z-<pre>` (`v0.3.0-beta`, `v1.0.0-rc.1`) is a **prerelease**, never latest; its notes are the `[Unreleased]` section unless it has its own.

**CI** (`.github/workflows/ci.yml`) runs on PRs to main, pushes to main, by hand and from `release.yml`: `lint` (`cargo fetch --locked`, `just lint`, `bash -n` on the scripts) and `test` on Linux, macOS and Windows (`cargo build --locked`, `just test`, `just e2e`, `just browser`). `lint` also parses the `.ps1` scripts with PowerShell's parser. Both use the toolchain pinned in `rust-toolchain.toml`. **Audit** (`.github/workflows/audit.yml`) runs `cargo deny check` when `Cargo.toml`, `Cargo.lock` or `deny.toml` change, and daily for new RustSec advisories; on a PR a new advisory is reported without blocking, since it isn't the PR's fault. A dependency with a license not in `deny.toml` fails it: check the license allows shipping in an MIT binary before adding it there. Dependabot opens weekly PRs for cargo (minor/patch grouped) and GitHub Actions. **PR checks** (`.github/workflows/pr.yml`): every commit in the PR must be a Conventional Commit, with no merge or `fixup!` commits (rebase merge lands them all on main), and `actionlint` checks the workflows.

**Repository settings** (applied with `gh api`, not in git): merge with **Rebase and merge** only (merge commits and squash off), delete branches after merge, "Update branch" button on, Discussions on, Dependabot alerts and security updates on. Branch rulesets need GitHub Pro or a public repo; once public, apply `.github/rulesets/main.json` (no deletion or force-push on main, linear history, PRs with resolved threads, rebase merge only, required CI and commit checks; admins may bypass only by merging a PR):

```sh
gh api -X POST repos/Rfluid/redline/rulesets --input .github/rulesets/main.json
gh api -X PUT repos/Rfluid/redline/private-vulnerability-reporting   # SECURITY.md links to it
```

Renaming a CI job changes its check name: update `required_status_checks` in the ruleset too. To check the pipelines without publishing: `gh workflow run ci.yml --ref <branch>` and `gh workflow run release.yml --ref <branch> [-f tag=v0.3.0-beta]` (a dry run: it plans, tests and builds, then prints the `gh release create` it would run).

`redline --version` prints `X.Y.Z (<git sha>)` (`build.rs`; `REDLINE_GIT_SHA` overrides). The sha is the commit at build time; `bin/redline` only rebuilds when sources change, so a fresh commit with no source change keeps the old sha.

`bin/redline` exports `REDLINE_REPO` (its clone), so a binary built elsewhere (a release) still links this clone. With a stale binary, `redline mcp` starts the current one and rebuilds in the background (`<state>/build.log`): agents give MCP servers about 30s to start, and a rebuild takes longer.

**Without Rust:** `bin/redline` downloads the release binary for `Cargo.toml`'s version into `target/prebuilt/` (via `gh release download` when `gh` is logged in, which a private repo needs, else plain `curl`), checks its sha256, and runs it. `REDLINE_PREBUILT=1` forces this even with cargo installed; `REDLINE_RELEASE_REPO` changes the source repo.

**Windows:** `bin/redline.ps1` does the same (the `.zip` release, `Invoke-WebRequest` instead of `curl`), then copies the binary to `target\bin\redline.exe`, which is what agents and the PATH shims run (see DECISIONS.md, "Windows"). The background rebuild for a stale `redline mcp` logs to `<state>\build.log`; the next run of the script installs it.

## Cross-agent smoke test (`tests/agent-loop.sh`)

Runs against an **installed** redline (each agent's own MCP config), so it's for checking a real install, not for CI:

```sh
tests/agent-loop.sh claude                    # PASS claude: ack-claude: ping from harness
tests/agent-loop.sh claude opencode agy       # in parallel, sharing one daemon
REDLINE=<clone>/bin/redline CODEX_MODEL=<slug> tests/agent-loop.sh codex
```

It starts the agent headless (`claude -p`, `codex exec`, `opencode run --auto`, `agy -p`, `cursor-agent -p`, `gemini -p`) with a prompt to `register_artifact`, `wait_for_comments` and `reply`. Then it finds the artifact with `redline list`, waits until `redline status` shows the agent's long-poll (`waiting`), submits a comment with the artifact token like the browser does, and passes when the agent's reply is in the thread. It kills only the `opencode serve` processes it started. Logs stay in a temp dir on failure (or with `KEEP_LOGS=1`). Agent-specific flags live in the `case` near the top; add a line there for a new agent.

## Database (Diesel)

- **Migrations** are embedded (`embed_migrations!("migrations")`) and run on every daemon start.
- **Driver:** `libsqlite3-sys` with `bundled`. No system SQLite is needed.

Adding a migration:

```sh
cargo install diesel_cli --no-default-features --features sqlite-bundled   # once
diesel migration generate add_something         # edit up.sql / down.sql
diesel migration run --database-url /tmp/rl.db  # scratch DB, to check the SQL
diesel print-schema --database-url /tmp/rl.db > src/store/schema.rs
```

**`print-schema` gotcha:** SQLite `INTEGER PRIMARY KEY AUTOINCREMENT` columns (`messages.seq`, `events.seq`) print as `Integer` (i32). We use `BigInt` (i64). After regenerating, put `BigInt` back for those columns. Alternatively, set up a `patch_file` in `diesel.toml`.

Store conventions:
- Each public write runs in one `conn.transaction`, and appends exactly one event via `append_event`.
- It re-reads the full `Event` after commit.
- The daemon holds the `Store` in a `std::sync::Mutex`. Never hold the guard across an `.await`.

## Adding an MCP tool

1. Add an args struct (`Deserialize + schemars::JsonSchema`; doc comments become field descriptions) and a `#[tool]` method in `src/mcp/tools.rs` returning `Result<String, String>`.
2. Do HTTP through `blocking(|api| …)`.
3. Add the daemon route if needed (`src/daemon/handlers.rs`), and a CLI equivalent (`src/cli.rs`). Every tool has a CLI twin for agents without MCP.
4. Document it in `docs/PROTOCOL.md` and in the `SKILL.md` table.

## Adding an agent

Add an entry to `AGENTS` in `src/install/agents.rs`, with:
- its skills dir
- the binaries and dirs used to detect it
- an `Mcp` target: `ClaudeCli`, `CodexToml`, `McpServersJson` or `OpencodeJson`. Add a new variant if its config format differs.

Then update the lists in `INSTALL.md` and `README.md`. Install must:
- back configs up once (`*.redline.bak`)
- never clobber non-redline files without `--force`
- refuse JSONC rather than mangle it

## Conventions

See [CODE_STYLE.md](CODE_STYLE.md). Step-by-step procedures for agents are in [`.agent/workflows/`](../.agent/README.md).
