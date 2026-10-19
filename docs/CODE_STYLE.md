# Code style and patterns

These conventions apply to all code in this repo. Tooling enforces what it can (`just lint`). The rest is review (`.agent/workflows/self-review.md`).

## Tooling

| Tool | Config | Enforced by |
|---|---|---|
| rustfmt | `rustfmt.toml`: width 120, `use_small_heuristics = "Max"` | `cargo fmt --check` |
| clippy | `[lints]` in `Cargo.toml` (pedantic + hygiene), `clippy.toml` | `cargo clippy --all-targets -- -D warnings` |
| rustc | `unsafe_code = "forbid"`, `unused_must_use = "deny"` | build |
| JS | no build step; syntax only | `node --check web/*.js` |
| editors | `.editorconfig` | |
| toolchain | `rust-toolchain.toml` (pinned version, with rustfmt + clippy) | rustup |

Every pedantic allow in `Cargo.toml` is deliberate:
- `module_name_repetitions`, `must_use_candidate`, `missing_errors_doc`, `missing_panics_doc`: noise for an application crate.
- `needless_pass_by_value`: axum extractors are taken by value.

One local allow: `clippy::unused_async_trait_impl` on the `#[tool_handler]` impl in `src/mcp/mod.rs`. The lint fires on code that rmcp's macro generates.

Tests may `unwrap`/`expect` (`clippy.toml`). Nothing else may.

## Crate layout

- **Library + thin binary.** `src/main.rs` only calls `redline::cli::run()`. Everything else lives in the library, so it's testable and reusable.
- **One concept per module.** When a file passes ~250 lines, or holds two concepts, turn it into a folder module (`foo/mod.rs` + parts). Current shape:

```
daemon/   mod.rs (App, router, serve) · auth.rs · handlers.rs · live.rs (long-poll, WS) · tests.rs
store/    mod.rs (Store API) · rows.rs (Diesel rows) · schema.rs · tests.rs
mcp/      mod.rs (server, channel push) · tools.rs
install/  mod.rs (install/uninstall/doctor) · agents.rs · link.rs · mcp_config.rs (+ tests.rs)
```

- **Long tests go in a sibling `tests.rs`** (`#[cfg(test)] mod tests;`). Short ones stay inline.
- **Visibility is as narrow as works:**
  - `pub(super)` for DB rows
  - `pub(crate)` for helpers shared across modules
  - `pub` only for the library API.

## Layers and boundaries

```
cli ──► api (blocking HTTP) ──► daemon (axum) ──► store (Diesel)
mcp ──► api                      ▲
web/*.js ──────── HTTP/WS ───────┘
```

- **The daemon owns the database.** Nothing else opens it. Agents and the CLI go through HTTP (`api.rs`), so there's one writer and one event fan-out.
- **Wire types** (`model.rs`) are separate from **DB rows** (`store/rows.rs`). Convert with `From`. Never serialize a row directly.
- **Every MCP tool has a CLI twin,** and both go through `Api`. No logic in `cli.rs` or `mcp/tools.rs` beyond argument mapping.

## Errors

- **Application code:** `anyhow::Result`, with `.context("what we were doing")` at I/O boundaries. Use `bail!` for invalid input.
- **HTTP boundary:** `ApiError` (`daemon/auth.rs`) maps to status + `{error}` JSON. `From<anyhow::Error>` → 400. Use `not_found` / `unauthorized` explicitly.
- **MCP boundary:** a tool returns `Result<String, String>`. `Err` becomes an `isError` result the agent can read. Write messages for the agent: say what to do next.
- **No `unwrap`/`expect` outside tests.** For "impossible" cases, return an error with context. Lock poisoning is recovered (`App::store()`), not unwrapped.

## Async and blocking

- **The daemon is async** (tokio/axum). The store is sync (Diesel) behind `std::sync::Mutex`. Take the guard in a statement or a small block, and **never hold it across `.await`**.
- **The agent side is blocking** (`ureq`). Inside async code (MCP), wrap it in `tokio::task::spawn_blocking`. See `mcp/tools.rs::blocking`.
- **Long waits are long-polls with a deadline,** not sleeps. Subscribe to the broadcast before the first read (`daemon/live.rs`).

## Persistence

- **One public write = one transaction = exactly one appended event.** Re-read the full `Event` after commit, and publish it from the handler.
- **Status rules live in the store** (`add_message`, `set_status`), not in handlers or clients.
- **Schema changes only via new migrations** (`.agent/workflows/add-migration.md`). Keep `seq` columns `BigInt`.
- **Tests use `Store::open(Path::new(":memory:"))`,** so migrations run there too.

## Naming and values

- **Ids are prefixed by type:** `art_`, `thr_`, `msg_`, `ses_` (`ids::new_id`). Tokens are 32 hex chars.
- **Timestamps** are `i64` Unix milliseconds (`ids::now_ms`).
- **No magic numbers.** Use named constants next to their use, with a comment when the value matters (`MAX_WAIT_SECS`, `EVENT_PAGE`).
- **Statuses** come from `model::STATUSES`. Validate at the store and at the MCP edge.

## Logging

- **`tracing` macros in the daemon** (`tracing::info!`, `debug!`), with fields, not formatted strings: `debug!(seq = ev.seq, artifact = %id, "event")`.
- **Never log a token or a full URI.** Log `uri().path()`, ids and statuses. The WebSocket URL's query holds a token.
- **The agent side doesn't log;** it returns errors with context.

## Comments and docs

- **Every file starts with a `//!` header:** what the module is for and how it fits. One to three lines.
- **`///` on public items** says behaviour and edge cases, not types.
- **Comment the *why*:** a non-obvious ordering, a lint allow, an "unverified" assumption. Don't narrate code.
- **User-facing text** (`README.md`) has no internals. **Agent-facing text** (`SKILL.md`, `INSTALL.md`, tool descriptions) is imperative and short.

## Installer safety patterns

- **Detect, don't assume.** Use `agents::detected()`, and ask the user about anything you can't detect.
- **Symlinks:** `link::link` never clobbers a foreign target without `force`, and backs it up when it does. On Windows it makes a junction, and the CLI gets shim files (`link::link_cli`); remove either with `link::remove`.
- **Platform code:** keep the split as small as possible: a `#[cfg(unix)]` / `#[cfg(windows)]` pair of small functions (`lifecycle::terminate`, `lifecycle::detach`), or `cfg!(windows)` when both branches compile everywhere (`paths::entry_point`). Paths a user or agent sees go through `paths::plain` (no `\\?\` prefix). Shell out to a clone script with `paths::script`, which picks the `.ps1` twin on Windows. A change to `bin/redline` or `scripts/checkout-release.sh` needs the same change in its `.ps1` twin.
- **Config edits:**
  - back up once (`*.redline.bak`)
  - write only when something changed
  - preserve formatting (`toml_edit`) and key order (`serde_json` `preserve_order`); keep trailing comments (managed-block markers) above our TOML table
  - on removal, restore the backup verbatim if the result equals it semantically
  - refuse unparseable/JSONC files and print the manual entry instead.
- **Uninstall removes only what points into this repo.**

## Browser code (`web/`)

- **No dependencies, no build step:** one IIFE per file, `"use strict"`, served with `include_str!`. A served URL may concatenate files (`/client.js` = `anchors.js` + `client.js`), so a file must not assume it's the only script; share through `window.Redline` with `Object.assign`.
- **`client.js` is protocol only:** no DOM writes except what the caller asks for. It exposes `window.Redline`.
- **`overlay.js` never mutates the artifact's DOM.** Its UI lives in a shadow root; highlights use the CSS Custom Highlight API, styled with a constructed sheet in `document.adoptedStyleSheets` (CSSOM, no element). The browser test asserts no `<style>` or redline element is added to the page. Escape every user string before it goes into `innerHTML` (`esc`).
- **Overlay layout:** `/overlay.js` concatenates small files that share through `window.Redline.ui`: `overlay-style.js` (CSS from `.design/` tokens), `icons.js`, `theme.js`, `regions.js`, `markdown.js`, `markers.js`, `popover.js` (pure HTML builders + placement), `dock.js`, `overlay-view.js` (state, model, painting), `overlay-input.js` (selection, keyboard, outside clicks), then `overlay.js` (popover lifecycle, actions, data). Load order is in `src/daemon/mod.rs`. Visual values come from `.design/`; `python3 .design/check.py` checks `overlay-style.js`'s tokens against it.
- **Storage:** `localStorage` access is wrapped in try/catch, because pages may block it.

## Tests

| Kind | Where | For |
|---|---|---|
| Unit | in the module (`mod tests` or `tests.rs`) | store rules, config editors, pure helpers |
| HTTP | `src/daemon/tests.rs` (`tower::ServiceExt::oneshot` on `router`, in-memory store) | routes, auth, submit rules, long-poll |
| End-to-end | `tests/e2e.sh` | only what crosses processes: daemon auto-start/lifecycle, CLI, MCP stdio |
| Browser | `tests/browser/overlay.test.mjs` (headless Chrome over CDP, `just browser`) | overlay / `client.js` behaviour |
| Manual browser | `.agent/workflows/manual-browser-test.md` | looks, anything the automated test can't judge |

Unit tests use `tempfile::tempdir()` for files and never touch `$HOME`. E2E uses a temp `REDLINE_HOME` and a random port.

## Commits

Conventional Commits (`feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `chore:`). Subject ≤ 72 chars. The body says *why*. Commit only when the user asks.
