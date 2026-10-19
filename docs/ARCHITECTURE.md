# Architecture

## Goal and non-goals

- **Goal:** a two-way channel between something an agent built (the *artifact*) and the agent session that built it. The user comments inline. The agent receives the comments, acts on them, and answers inline.
- **Non-goal: rendering.** redline never builds, serves or displays the artifact. `overlay.js` is an optional UI *for comments*, layered over the artifact.
- **Non-goal for v1: remote access.** Everything binds `127.0.0.1`.
- **Any agent.** The same install and behaviour for Claude Code, Codex, Cursor, Antigravity, opencode and Gemini CLI. MCP is the common interface, and a CLI covers the rest.

## Components

```
            browser (artifact, any origin)                      this device
 ┌────────────────────────────────────────────┐
 │ artifact page                              │     ┌─────────────────────────────────┐
 │  ├─ client.js  (protocol, no UI)  ─────────┼HTTP─►  redline serve  (daemon)        │
 │  └─ overlay.js (optional comment UI) ◄─────┼─WS──┤   axum · 127.0.0.1:7465         │
 └────────────────────────────────────────────┘     │   SQLite (Diesel) state dir     │
                                                    └──────▲───────────────▲──────────┘
                                                           │ HTTP           │ HTTP (long-poll)
                                            ┌──────────────┴───┐    ┌───────┴───────────┐
                                            │ redline mcp      │    │ redline wait|reply│
                                            │ rmcp, stdio, one │    │ CLI: any agent,   │
                                            │ per agent session│    │ background shells │
                                            └────────▲─────────┘    └───────────────────┘
                                                     │ MCP
                                               agent session
```

**Bindings** (`bindings/`) are more clients of the same protocol, for surfaces the overlay can't see into. `bindings/streamlit/` is a Python package whose component (an iframe per widget) loads `client.js` from the daemon and comments with `{kind: "app", key}` anchors. `bindings/react/` is a framework-agnostic store over `Client` (`redline-store.js`) plus `useRedline()` on `useSyncExternalStore`.

**One binary** (`redline`), with these subcommands:

| Subcommand | Role | Module |
|---|---|---|
| `serve` | The broker daemon. One per device, shared by all sessions | `src/daemon/` |
| `mcp` | MCP server on stdio. Each agent session spawns one, with its own *session id* | `src/mcp/` |
| `register`, `wait`, `list`, `reply`, `status`, `snippet`, `stop` | The same operations from a shell | `src/cli/` → `src/api.rs` |
| `install`, `uninstall`, `doctor` | Wire redline into each agent | `src/install/` |
| `update [check\|skip\|off\|on]` | Move the clone to the latest release; manage update notices | `src/update/` |

- **On-demand daemon.** `mcp` and the CLI read `daemon.json` in the state dir. If no healthy daemon answers `/health`, they spawn `redline serve` detached, in its own process group (Windows: no console window, its own Ctrl-C group, and out of the agent's job object when the job allows it), logging to `daemon.log`, and wait up to 5s.
- **Version handshake.** If the daemon is older than the caller, or the same version but started before the caller's binary was rebuilt, the caller stops it (SIGTERM to the pid from `/health`; `taskkill /F` on Windows) and starts a fresh one. So an update or rebuild takes effect on the next call. A newer daemon is never stopped by an older client.
- **Lifecycle commands.** `redline status` reports the daemon (port, pid, version, uptime, counts); `redline stop` stops it. On SIGTERM the daemon removes `daemon.json`; Windows has no SIGTERM, so `stop` kills the process there and removes `daemon.json` itself (SQLite's journal makes a hard stop safe). Every 10s it checks that `daemon.json` still names it, and exits if not (the state dir was deleted, or another daemon took over), so no orphan outlives its state.
- **Updates.** The daemon checks for a release at most once a day (hourly tick; a failed check retries after an hour): `git ls-remote --tags` on the clone's `origin`, highest `vX.Y.Z`, cached in `update.json`. That is redline's only outbound connection; it runs with no terminal (no prompts, 20s timeout) and is skipped when `config.toml` has `check = false`, or `CI` / `REDLINE_NO_UPDATE_CHECK` is set. Notices are computed from the cache and `config.toml` without the network, and shown three ways: the overlay's dock chip (`GET /api/artifacts/{id}/update`), a stderr line for people at a terminal, and `update` in `register`'s result for agents (`SKILL.md` tells them to mention it once). Dismissing (skip a version, or turn notices off) writes `config.toml`, from the overlay, the CLI or by hand; "Later" is browser-only. `redline update` runs `scripts/checkout-release.sh` (inline git steps for clones that predate it), then `bin/redline --version` to rebuild or download (on Windows, their `.ps1` twins under Windows PowerShell); the version handshake replaces the daemon on the next call.
- **Agent side is blocking.** `src/api.rs` uses `ureq`, a blocking HTTP client. The CLI calls it directly. The MCP tools call it inside `tokio::task::spawn_blocking`.
- **The skill is the repo.** `install` symlinks the whole repo as the skill (a directory junction on Windows), because `SKILL.md` is at the root, just like handoff. Agents also call `bin/redline`, a wrapper that builds the release binary when the sources are newer.
- **Windows entry point.** `bin/redline.ps1` (run by `bin/redline.cmd`, or by `bin/redline` under Git Bash) does what `bin/redline` does, then copies the binary to `target\bin\redline.exe`; MCP configs and the CLI shims on PATH (`redline.cmd`, plus an extensionless `redline` for Git Bash) run that exe directly. Windows can't overwrite a running exe but can rename one, so the script moves the old copy aside before putting the new one in place, and cargo's own output is never what runs. `paths::entry_point` and `paths::script` hold the per-OS choice.

## Data model

```
artifact  id, title, uri, kind, session, token, created       token = browser-side secret
  thread  id, artifact_id, anchor (JSON text), status, note (progress, nullable), created, updated
    message  seq, id, thread_id, author (user|agent), body, created
event     seq, artifact_id, thread_id, kind (message|status), author, at     append-only
attachment id, artifact_id, mime, data (BLOB), created                       region screenshots
```

- **Schema** lives in `migrations/` (embedded and run at daemon start) and `src/store/schema.rs`.
- **Attachments** are written with the thread that carries them (same transaction, same single event); the anchor references them as `image: "att_…"`. Readable by the admin token and that artifact's token.
- **Drafts never reach the daemon.** They live in the browser (`localStorage`) until submitted. "Send this comment" and "Send drafts" are the same call: `POST /submit` with any number of new threads and replies.
- **Status:** `open → working → answered → resolved`.
  - A **user** message (re)opens a thread.
  - An **agent** reply sets it to `answered` unless the reply gives a status.
  - A status change can carry a short `note` ("Rebuilding the chart"). The overlay shows it with a spinner while the thread is `working`. The next message or status change clears it.
  - Resolving is the user's call. The skill tells agents not to resolve threads.
- **Event log.** Every write (new thread, message, status change) appends an event in the same transaction. Readers keep a `since` cursor, so a consumer can never miss an event or see them out of order. Events are returned with the thread's *current* state.
- **Session routing.** `register_artifact` stamps the artifact with the calling MCP session. `wait_for_comments` without an artifact waits on every artifact in that session. Re-registering with an existing `id` moves the artifact to the current session, for example after an agent restart.

## Getting comments into the agent

MCP is request/response, so redline offers three delivery paths. Each agent uses the best one it supports:

| Mechanism | Works with | How |
|---|---|---|
| Blocking tool | every MCP agent | `wait_for_comments` long-polls `/api/events` for up to 540s. The skill loops it |
| Background CLI | agents with background shells (Claude Code) | `redline wait --session S --since N` exits when comments arrive, which wakes the agent. `register_artifact` returns `session` and `since` for it |
| Push | Claude Code channels (experimental) | `redline mcp --channel` sends `notifications/claude/channel` per new user event |

All three read the same event log. Agent-authored events are filtered out of agent waits (`author=user`).

**Channels** (checked 2026-10-16 against code.claude.com/docs/en/channels-reference): the server declares `experimental["claude/channel"]: {}` only with `--channel`, and sends `notifications/claude/channel` with `{content, meta}`, where `meta` values are strings with identifier keys. Claude Code won't register a channel on protocol revision 2026-07-28, which rmcp prefers, so `--channel` caps the supported revisions at 2025-11-25. The push loop starts at the current head, so a restart doesn't replay old comments. Claude Code must be started with `--dangerously-load-development-channels server:<name>` (interactive only), and Team/Enterprise orgs must allow channels.

## Security model

- **Loopback only.** The daemon binds `127.0.0.1`.
- **Admin token** (`admin.token`, mode 0600 on Unix; on Windows the profile's user-only ACL covers it; stable across restarts): it lets agent-side callers speak as `agent` on every artifact.
- **Artifact tokens** are embedded in the page. They let the caller speak as `user` on that one artifact. Anyone who can load the page can comment on it. That's why the snippet is for review only.
- **CORS is permissive**, because artifacts load from `file://`, `localhost:5173`, `localhost:8501` and so on. The token is the boundary, not the origin.
- **Tokens are never logged.** Request logs carry the path only (the WebSocket URL has `?token=`); tests grep `daemon.log` for both token kinds.
- **Comment text is untrusted input** to the agent. It's user feedback, but anything that can load the page can write it.
- **Update dismissals accept an artifact token.** A page script could use it to hide update notices or turn them off, nothing more: the body is `{version: X.Y.Z}` or `{all: true}`, and it can't turn checks on or write anything else.
- **`overlay.js` never mutates the artifact's DOM.** Its UI lives in a shadow root, and highlights use the CSS Custom Highlight API.

## State on disk

`$REDLINE_HOME`, by default `~/.local/state/redline/` (Windows: `%LOCALAPPDATA%\redline\`):

| File | |
|---|---|
| `redline.db` | SQLite (WAL) |
| `admin.token` | agent token, 0600 |
| `daemon.json` | `{port, pid, version, started}` of the running daemon, 0600 |
| `update.json` | the last release check: `{checked, latest, error?}` |
| `daemon.log` | daemon log when auto-started: startup, one line per request (method, path, status, latency; never the query string, which can hold a token), failed requests with their error. `RUST_LOG` sets the filter (default `redline=info,tower_http=info`; `debug` adds events and WebSocket open/close) |

Settings live apart from state, in `~/.config/redline/config.toml` (`$XDG_CONFIG_HOME`, or `$REDLINE_CONFIG`): today only `[update] check` and `skip` (see `docs/PROTOCOL.md`).
