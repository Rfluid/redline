# Decisions

Each entry: the choice, what else was considered, and why.

## Name: `redline`

*Redline* is editorial markup on a draft, which is exactly the workflow. It's short to type (`redline`, `/redline`) and works as a verb ("redline this"). Alternatives considered: marginalia (evocative but long), gloss (ambiguous), backchannel (stresses the pipe, not the comments).

## Shape: daemon + MCP + CLI in one Rust binary

- **One daemon per device** gives the browser a stable origin (`127.0.0.1:7465`) and lets every agent session share state.
- **Each session's `redline mcp`** is a thin client.
- **The CLI duplicates every MCP tool,** so agents without MCP, or with background shells, work the same way.
- **One binary** means one install step and one thing to version.

## Comment UI: optional overlay, separate from the protocol

`client.js` is protocol-only, and `overlay.js` is an opt-in UI. This keeps the "communication only, no rendering" line clean:
- Surfaces with their own UI (React apps, Streamlit components) use `client.js` directly.
- Plain HTML gets a working UI for free.

Considered:
- protocol only: unusable out of the box
- a browser extension: works on any page without embedding, but has separate distribution and more work. It could come later as another client of the same protocol.

## Network scope: localhost only (v1)

Remote review (a phone, a teammate) needs a relay, auth and TLS. That's deferred. Tokens and a loopback bind are enough locally.

## Dependencies

| Area | Choice | Considered | Why |
|---|---|---|---|
| CLI | `clap` 4 (derive, env) | argh, bpaf, lexopt | Standard; env fallbacks (`REDLINE_PORT`, `REDLINE_SESSION`) and help for free |
| HTTP/WS server | `axum` 0.8 (`ws`) + `tower-http` (CORS) | actix-web, raw hyper, tiny_http + tungstenite | Tokio-native, built-in WebSocket, middleware ecosystem |
| Async runtime | `tokio` | none | Required by axum and rmcp |
| MCP | `rmcp` 3.x (`server`, `macros`, `transport-io`) + `schemars` | hand-rolled JSON-RPC (the first scaffold), rust-mcp-sdk | Official SDK; tracks the spec; `#[tool]` generates input schemas from Rust types. Custom notifications (Claude channels) still work via `CustomNotification` |
| Storage | `diesel` 2.3 (sqlite) + `diesel_migrations` + `libsqlite3-sys` (`bundled`) | rusqlite (+ rusqlite_migration), sqlx, redb, JSON files | See the evaluation below |
| Agent-side HTTP client | `ureq` 2 (blocking) | reqwest | Small, sync. The CLI is sync, and MCP wraps calls in `spawn_blocking` |
| Config editing | `toml_edit` (Codex), `serde_json` with `preserve_order` (others) | text append | `toml_edit` preserves user formatting and comments; `preserve_order` keeps the user's key order (without it, the first real install re-sorted every JSON config) |
| Logging | `tracing` + `tracing-subscriber` (`fmt`, `env-filter`), `tower-http` `trace` | `log` + `env_logger`, `eprintln!` | axum and tower-http already emit `tracing` spans; `RUST_LOG` filtering for free. A custom span keeps query strings (tokens) out |
| Misc | `serde`/`serde_json`, `anyhow`, `uuid` (ids, tokens), `dirs` | | |
| Windows skill links | `junction` (Windows only) | `std::os::windows::fs::symlink_dir`, `cmd /c mklink /J` | Symlinks need admin or Developer Mode; junctions don't. The crate does the reparse-point calls we can't (`unsafe_code = "forbid"`), and shelling out to `cmd` would bring its quoting rules into paths |
| Tests (dev only) | `tempfile`, `tower` (`util`, for `ServiceExt::oneshot`) | spawning a real server per test | In-process HTTP tests are fast and need no ports; axum already depends on tower |

### Evaluation: Diesel vs rusqlite

The first scaffold used `rusqlite` with hand-written SQL. Diesel was evaluated as an ORM to ease development, and adopted.

Diesel gives:
- **A typed schema:** column names and types are checked at compile time. A typo or type change breaks the build, not a query at runtime.
- **Embedded migrations** with a CLI (`diesel migration generate/run`). rusqlite would need `rusqlite_migration` or a hand-rolled `user_version` scheme.
- **Less mapping code:** `Queryable`/`Selectable`/`Insertable` derives replace row-to-struct closures.
- **Clean optional filters:** boxed queries (`into_boxed()`) express the event filters (artifact, session, author) without string-building SQL.
- **Ergonomic transactions** (`conn.transaction(|c| …)`).

Diesel costs:
- **Compile time:** more proc macros, slower first build. Not noticeable next to axum + rmcp.
- **`src/store/schema.rs` must match the migrations.** `print-schema` maps `INTEGER PRIMARY KEY` to `Integer`; we keep `BigInt` (see DEVELOPMENT.md).
- **Harder errors:** macro error messages are worse than rusqlite's.
- **Sync API**, the same as rusqlite. sqlx would be async, but our load is one user on localhost, and a `Mutex<Store>` is fine.
- **`libsqlite3-sys` must be pinned** to the version Diesel supports. Add `bundled` explicitly so no system SQLite is needed.

**Verdict:** adopt Diesel. The schema will grow (anchors, attachments, sessions, maybe users for the remote relay), and typed queries plus first-class migrations pay off then. rusqlite remains the fallback if compile time or macro friction becomes a problem. The store API (`Store` methods) hides the choice from the rest of the code.

## Code quality: pedantic clippy as errors, lib + thin bin

`clippy::pedantic` warns, and CI runs with `-D warnings`. The few allows are documented in CODE_STYLE.md. `unsafe_code` is forbidden. The crate is a library with a thin binary, so every module is unit-testable. Files are split by concept at ~250 lines. A `justfile` gives humans, agents and CI one entry point (`just check`).

**The toolchain is pinned** to an exact version in `rust-toolchain.toml`. With a floating `stable`, a stale local toolchain passed while CI's newer clippy failed on a new lint. Pinning keeps local and CI identical, and a bump is a deliberate change.

## Agent workspace: `.agent/`

Agent instructions live in one agent-agnostic folder: context, rules, workflows, prompts, skills and a session log. `CLAUDE.md`, `AGENTS.md` and `AGENT.md` only point to it. This avoids drift between per-agent instruction files, and lets any agent pick up where another left off (via `.agent/log/`).

## Delivery to agents: three paths over one event log

There's a blocking tool (universal), a background CLI (Claude Code style) and push (Claude channels, experimental). All three read the same `since`-cursored log, so they can coexist and none can miss events. See ARCHITECTURE.md.

## Drafts live in the browser

"Draft" is a UI concept. Keeping drafts client-side means the agent only ever sees what the user deliberately submitted, and the daemon needs no draft state.

## Agent list (checked 2026-10-16)

**Gemini CLI stays a separate agent.** The Gemini CLI README and geminicli.com/docs say: "Unpaid tier and Google One users: Gemini CLI was replaced by Antigravity CLI on June 18th, 2026." The repo still ships weekly releases, and paid users keep it. Antigravity's migration guide (antigravity.google/docs/cli/gcli-migration) says agy defines servers in `~/.gemini/config/mcp_config.json` and skills in `~/.gemini/antigravity-cli/skills`; `agy plugin import gemini` is a one-time import, not a live read of `settings.json`. So installing for `agy` doesn't cover `gemini`, and vice versa. Considered: aliasing `gemini` to `agy` (wrong config files), dropping it (breaks paid users).

**Skill dirs: prefer the shared `~/.agents/skills`.** How each agent treats the same skill name in several dirs:
- Cursor (cursor.com/docs/context/skills) reads `~/.agents/skills`, `~/.cursor/skills`, `~/.claude/skills` and `~/.codex/skills`. Cursor staff on the forum (2026-01-28): "Cursor loads Skills from different directories and treats them as separate entities"; de-duplication is a logged bug.
- opencode reads `~/.config/opencode/skills`, `~/.claude/skills` and `~/.agents/skills`, and keys skills by folder name (`skills.set(skill.id, skill)` in `packages/core/src/config/plugin/skill.ts`), so a duplicate is harmless.
- Codex reads only `~/.agents/skills` (user level); Gemini CLI reads `~/.gemini/skills` and the `~/.agents/skills` alias, with precedence, and prints "Skill conflict detected" when both hold the same skill (seen on gemini 0.38.2 after the first install). So Gemini uses `~/.agents/skills` too, and the old `~/.gemini/skills/redline` link is removed.

So Cursor now links into `~/.agents/skills` (shared with Codex and opencode) instead of `~/.cursor/skills`; install and uninstall remove the old `~/.cursor/skills/redline` link. Cursor still sees two copies when Claude Code is installed too (`~/.claude/skills` is the only dir Claude reads). That's unavoidable and harmless (same files), and `doctor` prints a note. Considered: skipping `~/.claude/skills` when Cursor is present (breaks Claude).

**Cursor entries carry `"type": "stdio"`.** cursor.com/docs/context/mcp lists `type` as required in its STDIO field table, but its own example omits it. On this machine cursor-agent 2025.10.02 loaded the server both ways (`cursor-agent mcp list-tools` → 5 tools for each, project-level `.cursor/mcp.json` in a temp dir). Writing it follows the table and costs nothing. Gemini and Antigravity entries stay without it (their docs have no `type` field).

## Browser test: raw DevTools protocol, no npm

`tests/browser/` drives headless Chrome over CDP with Node's built-in `fetch` and `WebSocket` (about 150 lines of harness). Considered: Playwright (an npm install plus a ~150 MB browser download in CI, and a `package.json` in a repo with no JS toolchain), Puppeteer (same). Chrome is preinstalled on GitHub's Ubuntu, macOS and Windows runners, and the harness only needs `Runtime.evaluate`, so a dependency buys little. If the tests grow to need real input events or tracing, revisit Playwright.

## Streamlit: a component per widget, no build step

Streamlit renders into its own DOM and runs components in iframes, so the overlay can't select widget text, and an embedded overlay would only see its own iframe. The binding gives each reviewed widget its own small component (a Comment button that opens its threads) and anchors comments by an app-chosen `key`. The frontend is one HTML file that speaks the component postMessage protocol directly (`streamlit:componentReady` / `render` / `setFrameHeight`), so there's no npm/React toolchain, matching `web/`. The snippet travels in `REDLINE_EMBED` so no token lands in the app's source. Considered: one panel component listing all threads (no per-widget context), and the official React template (a JS build for a 60-line UI). Drafts use the same `localStorage` key as the overlay (the component iframes share the app's origin), and widgets follow each other through `storage` events, so "Send drafts" in any widget sends every draft.

## React: a store plus a hook, copied in

`bindings/react/redline-store.js` is a framework-agnostic store (`subscribe`/`getSnapshot`, immutable snapshots, newer-wins merge of live events and refreshes). `useRedline.js` is ~20 lines on `useSyncExternalStore`. Both are plain ESM with no build. Agents copy them into the app, because bundlers (Vite's `server.fs`, Next) refuse imports from outside the project. Considered: an npm package (publishing and versioning for 80 lines), a context provider (more API, same result). The example uses React from esm.sh with `htm`, so it runs without a toolchain.

## Region screenshots: crop what the page can draw, store with the thread

Browsers can't screenshot the DOM without `getDisplayMedia` (a permission prompt per capture) or a DOM-cloning library like html2canvas (a dependency that re-renders the page, often wrongly). Charts are where a picture matters, and they're usually `<canvas>`, `<svg>` or `<img>`, which a page can draw into a canvas itself. So the overlay crops those (480×320 CSS px around the click) and sends nothing for plain HTML, where the selector and the point say enough.

The image travels inside `submit` and is stored in an `attachments` table in the same transaction as its thread, so "one write = one event" still holds and no orphan uploads exist. A separate upload route was considered (it would need its own event or none). Agents fetch the bytes with `get_image`, which returns an MCP image block the model can see; the CLI twin writes a file. Limits: 512 KiB decoded, png/jpeg/webp only, served with `nosniff`.

Also: the overlay used to add a `<style>` to `document.head` for the `::highlight()` colour, which broke the "never mutates the artifact's DOM" invariant. It now uses a constructed stylesheet in `document.adoptedStyleSheets`.

## Release binaries: zig for Linux, one macOS runner, download in the wrapper

Linux binaries are cross-built on one Ubuntu runner with `cargo-zigbuild`, targeting glibc 2.28, so they run on distros older than the runner (a native build on ubuntu-latest needs glibc 2.39). zig also cross-compiles the bundled SQLite C code for aarch64 without a separate toolchain. Considered: native arm64 runners (not free for private repos), `cross` (Docker), musl (needs a musl C cross-compiler for aarch64). macOS builds both targets on one arm64 runner.

`bin/redline` stays the single entry point: with cargo it builds, without cargo it downloads the release matching `Cargo.toml`'s version, verifies its sha256, and caches it in `target/prebuilt/`. The checksum guards against a truncated or corrupted download, not a compromised release. While the repo is private, downloads go through `gh release download` (the user's GitHub auth).

## Windows

Windows is supported natively (not only under WSL): the same binary, install and update flow, with these differences.

- **Entry point.** Every Windows has Windows PowerShell 5.1, so `bin/redline.ps1` mirrors `bin/redline` (and `scripts/checkout-release.ps1` mirrors the `.sh`); `bin/redline.cmd` runs it from cmd/PowerShell and `bin/redline` hands over to it under Git Bash. Considered: requiring Git Bash (its `bash.exe` is easy to confuse with WSL's in `System32`, and MCP clients spawn commands without a shell), a launcher written in Rust (it would itself need building or downloading first).
- **MCP configs and the PATH shims run `target\bin\redline.exe` directly, not the script.** Windows locks a running exe against writes, so cargo could never rebuild a binary that agents are running; the script builds into `target\release` and copies the result to `target\bin`, renaming the running copy aside first (renaming a running exe is allowed). And a script in the MCP stdio path is fragile there: Node-based clients can't spawn `.cmd` files without a shell, and Windows PowerShell 5.1 re-encodes and mangles native arguments. The cost: a source change reaches agents on the next run of `bin\redline.cmd` (or `redline update`), not on the next MCP start.
- **`-ExecutionPolicy Bypass`** on every `powershell.exe -File` we run: the default policy on Windows clients blocks local scripts. It applies to that one process and doesn't change the user's setting; a Group Policy that enforces a policy still wins.
- **Links.** Skill dirs are directory junctions (no admin rights needed). The CLI on PATH is two small shim files, `redline.cmd` and an extensionless `redline` for Git Bash, each with a marker line so install, uninstall and doctor know they're ours. No `.ps1` shim: PowerShell would refuse to run it under the default execution policy, and finds `redline.cmd` anyway.
- **Stopping the daemon** is `taskkill /F`: there's no SIGTERM, and a windowless process ignores the polite `taskkill`. SQLite's journal makes a hard stop safe; `stop` removes `daemon.json` for it. The daemon starts with `CREATE_BREAKAWAY_FROM_JOB` when the agent's job object allows it, so a kill-on-close job doesn't take it down with the MCP server.
- **State** lives in `%LOCALAPPDATA%\redline` (`dirs::data_local_dir`), the closest thing to `~/.local/state`. The config stays `~/.config/redline/config.toml`, the same path the docs give everywhere. Token files get no mode bits: files under the user profile inherit a user-only ACL.
- **CI** runs the full test job on `windows-latest`, with `tests/e2e.sh` under Git Bash going through `bin/redline.ps1`. Releases add `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc` (cross-compiled on the x64 runner) as `.zip` files.

## Versioning

Semver tags `vX.Y.Z` that must match `Cargo.toml` (the release workflow checks). `CHANGELOG.md` follows Keep a Changelog; the release notes are that version's section. `--version` carries the git sha from `build.rs`, so bug reports name the exact build. The daemon handshake (P1.4) still compares the plain semver version plus the binary's mtime, not the sha, so a rebuild without a commit still restarts the daemon.

## Users run releases, not main; prereleases are opt-in

`INSTALL.md` clones the repo and then runs `scripts/checkout-release.sh`, which checks out the highest stable tag (`vX.Y.Z`, sorted by version). Updating runs the same script; `bin/redline` notices the changed sources and rebuilds (or downloads the matching binary), and the daemon handshake replaces the running daemon. A clone stays a git checkout because agents link to it as the skill dir, so a release is a tag checkout rather than an unpacked tarball. Considered: tracking main (users get unreleased work), downloading only the binary (the skill files, `web/` and `SKILL.md` would still come from main).

Tags with a prerelease suffix (`v0.3.0-beta`, `v1.0.0-rc.1`) become GitHub prereleases with `--latest=false`, and the script skips them unless named, so nobody gets a beta by accident. A stable tag is marked latest only if no higher stable tag exists, so a backport doesn't take over. `release.yml` runs `ci.yml` on the tagged commit (`workflow_call`) before publishing, and a manual run is a dry run that publishes nothing.

`scripts/prepare-release.sh` (adapted from aura's) bumps every workspace crate to one version: redline ships one binary, so per-crate versions (aura's "bump only changed crates") would add bookkeeping without a user. `bindings/streamlit` keeps its own version.

## Dependency audit: cargo-deny, Dependabot

`cargo-deny` (`deny.toml`, `audit.yml`) over `cargo-audit`: one tool checks RustSec advisories *and* licenses and sources, and licenses matter because release binaries bundle every dependency under MIT. The allow list is explicit; `MPL-2.0` (`option-ext`, file-level copyleft, unmodified) and `CDLA-Permissive-2.0` (`webpki-roots`' CA data) were reviewed when added. Advisories run daily and fail main, but only warn on PRs, so a fresh advisory doesn't block unrelated work. Dependabot keeps cargo and the Actions pins current; minor/patch bumps are grouped to keep PR noise to one a week per ecosystem. Not added: CodeQL (Rust support is young and `unsafe_code = "forbid"` plus clippy pedantic cover most of it; revisit for `web/*.js`), `cargo-semver-checks` (redline isn't a library others depend on).

## Rebase and merge only

Every PR lands with **Rebase and merge**: no merge commits, no squash. History on main stays linear (so `git bisect` and `git log` read cleanly, and `prepare-release.sh`'s changelog work isn't muddied by merge noise), while PRs keep their individual commits instead of one squashed summary. The cost is that each commit must stand alone, so `pr.yml` rejects non-Conventional subjects, merge commits and leftover `fixup!` commits. Considered: squash merge (simpler for contributors, but loses the per-commit story agents and reviewers rely on here), merge commits (non-linear history).

## Updates: a daily check, notices you can skip or turn off, `redline update`

The usual CLI pattern (gh, npm's update-notifier): check at most once a day, cache the answer, show a short notice, and make it easy to silence. Dismissal follows Sparkle's three choices: **Later** (hide for a day; browser-only), **Skip this version** (`skip` list; a newer release still shows), and **off** (`check = false`: no checks, no notices).

- **Source: the clone's git remote,** not the GitHub API. `git ls-remote --tags origin` works for a private repo with the user's existing git credentials, needs no token or new dependency, and matches what `redline update` installs (the highest `vX.Y.Z` tag, like `checkout-release.sh`; prereleases never notify). It runs with `GIT_TERMINAL_PROMPT=0`, ssh `BatchMode`, and a 20s timeout, so it can't hang or prompt.
- **The daemon runs it** (hourly tick, daily check, hourly retry after a failure), in the background. CLI calls and the overlay only read the cache, so no command waits on the network. Off in CI and with `REDLINE_NO_UPDATE_CHECK`, which only stop the background check; an explicit `redline update check` always runs.
- **Where the notice shows:** the dock's Update chip (the user is looking at the overlay); a stderr note for people at a terminal (never on stdout, which is JSON for agents; not for `serve`/`mcp`); `update` in `register`/`register_artifact` for agents, with `SKILL.md` telling them to mention it once and ask before updating. Not in MCP `instructions`, which are read once at connect and would go stale.
- **Settings in `~/.config/redline/config.toml`** (XDG; `$REDLINE_CONFIG`), apart from the state dir, because they're user choices. TOML via the existing `toml_edit`, so hand edits and comments survive our writes. The overlay writes through `POST /api/artifacts/{id}/update/dismiss` with the artifact token: the worst a hostile page script can do is hide notices.
- **This is redline's first outbound connection** (the daemon still only *binds* loopback). Considered: no check at all (users miss fixes, as happened with the MCP timeout bug), checking on every CLI call (latency, and it hammers the remote), auto-updating (changes code under running sessions without consent).

## Design system v2: red pen, blue pencil, ink on paper (`.design/`)

The v1 overlay borrowed stock parts (Material red and blue, Bootstrap alert pastels). Red did everything: the primary buttons, the tab, the quotes and the highlights. The theme followed the OS, so a dark OS on a white page drew a dark panel and a dark-red highlight on white. v2 sets a direction and writes it down in `.design/` (principles, tokens, components, layout, accessibility, outlined logo) before any restyle:
- **Red only for marks on the artifact.** Buttons and chrome are ink. The agent is blue pencil (the editor's non-printing note), so who said what reads at a glance.
- **Theme from the page's background**, not `prefers-color-scheme`. Highlights are translucent fill plus underline, so they read on any page.
- **Margin markers and popovers plus a dock** replace the full-height drawer as the default. The drawer stays, opened only from the dock.
- **No web fonts** in the overlay (loopback pages, no network, no flash). The Newsreader wordmark ships as outlined SVG (`.design/logo/build.py`).

Considered: a quiet neutral dev-tool look (safe, forgettable), a friendly collab look (avatars, soft cards: familiar but generic), a terminal look (polarizing). The editorial direction follows from the name (see "Name"). Mockups: a private claude.ai design canvas, "redline redesign"; `.design/` wins when they disagree. Implementation: ROADMAP P7.

---

# Proposals (ROADMAP Phase 2): drafts, not approved

Nothing below is built. Each needs the user's approval first, and a final entry above once decided.

## Draft P2.1: browser extension as the overlay host (recommended first)

**Goal:** review any page without pasting a snippet, so no token ever sits in the artifact and `redline strip` becomes a safety net, not a step.

**Design.**
- A Manifest V3 extension (Chrome/Edge first; Firefox later) that injects `client.js` + `overlay.js` (from its own package, same code as `web/`) into a tab when the tab's URL matches a registered artifact's `uri`, or when the user clicks "Review this page".
- **Pairing:** `redline pair` prints a one-time code; the extension trades it at `POST /api/pair` for an *extension token*. That token is a new, third kind: it can list artifacts and mint/read artifact tokens, never speak as `agent`. Revocable with `redline pair --revoke`.
- The content script talks to the daemon from the extension's origin (`chrome-extension://…`), not the page's, and passes the artifact token to the overlay in its isolated world. The page's own scripts can't read it.
- "Review this page" on an unregistered URL creates an artifact with `session: null`; the agent claims it with `register_artifact(id)`.

**Threat model.**
| Asset | Threat | Mitigation |
|---|---|---|
| extension token | another local app or page steals it to comment as the user on every artifact | stored in `chrome.storage.local` (not page-reachable); daemon only accepts it with the extension's `Origin`; revocable; can't speak as agent |
| artifact token | a hostile page script reads it from the DOM | never in the DOM: lives in the content script's isolated world; overlay UI stays in a closed shadow root |
| user's browsing | extension with `<all_urls>` reads every page | request `activeTab` + optional host permissions per artifact origin (localhost, file://) instead of `<all_urls>` |
| agent session | comment text as prompt injection | unchanged: comments are untrusted input (SKILL says so) |

**Invariants:** 1 (communication only) holds. 3 (two token kinds) changes to three. Needs this entry approved.

**Cost:** an extension package and store listing (or "load unpacked"), plus `/api/pair`. Medium.

## Draft P2.2: remote relay (review from a phone, or by a teammate)

**Goal:** someone not on this machine can comment, without exposing the daemon.

**Options.**
1. *Tunnel the daemon* (`cloudflared`, `tailscale funnel`): zero code, but exposes **every** route, including admin ones, to anyone who finds the URL. Rejected.
2. *Bind to a tailnet/LAN address*: breaks invariant 2, no TLS by default, still exposes admin routes. Rejected.
3. **Outbound relay (recommended):** `redline share <artifact>` makes the daemon open an outbound WebSocket to a relay (self-hostable, a small stateless Rust binary in this repo; optional hosted instance). The relay serves `https://relay/a/<share-id>` and forwards only an allowlist of artifact routes (`threads`, `submit`, `ws`, `attachments`) over that socket. The daemon still binds loopback only.

**Share tokens:** a fourth token kind, per share: scoped to one artifact, speaks as a *named* user (`user:alex`, chosen at join), expiring (default 24h), revocable (`redline share --revoke`), rate-limited at the relay and the daemon.

**Threat model.**
| Actor | Can | Mitigation |
|---|---|---|
| anyone with the link | comment as some user | links are capabilities: unguessable, expiring, revocable; optional join passphrase; names shown to the agent |
| the relay operator | read comments and screenshots in transit | self-host by default; optional end-to-end mode: content encrypted with a key in the URL fragment (`#k=…`, never sent to the relay) |
| network attacker | read/alter traffic | TLS to the relay; the daemon pins the relay's certificate or uses a pre-shared relay key |
| a malicious commenter | prompt-inject the agent; flood | comments stay untrusted input; the agent sees `author: user:<name>` and SKILL says remote comments never authorize destructive actions without the local user; limits from P2.3 plus per-share rate limits |
| a compromised relay | call admin routes | impossible by construction: the daemon only accepts relayed calls on the allowlist, with a share token |

**Invariants:** 2 holds (outbound only). 3 changes (share tokens, named users), and the data model gains `messages.author` values beyond `user|agent`. Needs this entry approved.

**Cost:** large (relay binary, share tokens, E2E mode, docs). Do after P2.1.

## Draft P2.3: adapter for claude.ai Artifacts' native comments (recommend: don't build)

claude.ai Artifacts already have comment threads, and Claude Code sessions can read and answer them (the Artifact comment tools), with notifications. There's no public API to read those comments from outside a Claude session, so an adapter would have to run *inside* the agent: an MCP tool calling another tool, which MCP can't do. The useful part is guidance: SKILL.md can say "for a page published as a claude.ai Artifact, use its native comments, not redline". Revisit if a public comments API appears.
