# Protocol

Everything here is served by `redline serve` at `http://127.0.0.1:7465`. The port can be overridden with `REDLINE_PORT` or `--port`.

## Auth

`Authorization: Bearer <token>`. The WebSocket takes `?token=` instead, because browsers can't set headers on it.

| Token | Where it comes from | Speaks as | Scope |
|---|---|---|---|
| admin | `$REDLINE_HOME/admin.token` | `agent` | everything |
| artifact | returned by register, embedded in the page | `user` | that artifact |

## HTTP API

| Method | Path | Token | Body / query | Returns |
|---|---|---|---|---|
| GET | `/health` | none | | `{name:"redline", version, pid, started}` |
| GET | `/client.js`, `/overlay.js` | none | | JavaScript |
| POST | `/api/artifacts` | admin | `{title, uri?, kind?, session?, id?}` | `Artifact` (with `token`) |
| GET | `/api/artifacts` | admin | | `Artifact[]` |
| GET | `/api/artifacts/{id}` | admin, artifact | | `Artifact` (`token` only for admin) |
| GET | `/api/artifacts/{id}/threads` | admin, artifact | | `Thread[]` |
| POST | `/api/artifacts/{id}/submit` | admin, artifact | `{threads:[{anchor, body, image?}], replies:[{thread_id, body, status?}]}`; `image` = `{mime, data}` (base64) | `Event[]`, all or nothing |
| GET | `/api/attachments/{id}` | admin, that artifact's token | | the image bytes (`Content-Type` png/jpeg/webp) |
| GET | `/api/artifacts/{id}/ws?token=` | artifact, admin | | WebSocket of `Event` JSON text frames |
| POST | `/api/threads/{id}/messages` | admin, artifact | `{body, status?}` | `Event` |
| POST | `/api/threads/{id}/status` | admin, artifact | `{status, note?}` (`note` ≤ 1 KiB: progress shown under `working`) | `Event` |
| GET | `/api/artifacts/{id}/update` | admin, artifact | | `UpdateInfo` |
| POST | `/api/artifacts/{id}/update/dismiss` | admin, artifact | `{version: "X.Y.Z"}` (skip that release) or `{all: true}` (checks and notices off) | `UpdateInfo`; writes `config.toml` |
| GET | `/api/events` | admin | `since, timeout (s, ≤600), artifact?, session?, author?` | `{events: Event[], next}` |
| GET | `/api/status` | admin | | `{version, build, pid, started, uptime_s, artifacts, threads, by_status, events, head, waiting}` (`build` = git sha, `head` = latest event seq, `waiting` = open long-polls by filter: `artifact:<id>`, `session:<id>`, `all`) |

- **Errors:** `{error: "..."}`, with status 400, 401, 404, 413 or 422 (malformed JSON).
- **Limits** (`src/daemon/limits.rs`):

  | What | Limit | Status |
  |---|---|---|
  | request body | 1 MiB | 413 |
  | one message (comment or reply) | 64 KiB, non-empty | 413 / 400 |
  | one anchor, serialized | 16 KiB | 413 |
  | status `note` | 1 KiB | 413 |
  | `image` (decoded) | 512 KiB; `image/png`, `image/jpeg`, `image/webp` | 413 / 400 |
  | threads + replies per submit | 1–100 | 400 |
  | artifact `title`, `uri`, `kind`, `id`, `session` | 2 KiB each | 413 |
- **Submit is atomic.** A reply to a thread of another artifact rejects the whole batch (404) and writes nothing.
- **Version handshake.** Before each call, clients read `/health`. They replace the daemon (SIGTERM to the `pid` it reports, `taskkill /F` on Windows, then start a new one) when it runs an older version, or the same version but `started` is before the client binary's mtime (a rebuild). A newer daemon is left alone.
- **Update notices.** `UpdateInfo` = `{current, latest, available, notify, check, skipped, checked, command, notes}`: built from the cached release check (`update.json`) and `[update]` in `config.toml`, never from the network. `notify` = `available && check && !skipped`; show a notice only then. Any token for the artifact may dismiss: it only hides a notice. `version` must be `X.Y.Z`.
- **`/api/events` is a long-poll.** It returns as soon as there are events with `seq > since` (at most 100), or `{events: [], next: since}` after `timeout`.

## Types

```jsonc
// Artifact
{ "id": "art_…", "title": "Q3 summary", "uri": "file:///…/q3.html", "kind": "html",
  "session": "ses_…", "token": "…", "created": 1760550000000 }
// Thread
{ "id": "thr_…", "artifact_id": "art_…", "anchor": { … }, "status": "working",
  "note": "Rebuilding the chart",           // or null; cleared by the next message or status change
  "created": 1760550000000, "updated": 1760550000000, "messages": [Message] }
// Message
{ "id": "msg_…", "thread_id": "thr_…", "author": "user", "body": "make this bigger", "created": 1760550000000 }
// Event (thread = current state at read time)
{ "seq": 42, "artifact_id": "art_…", "kind": "message", "author": "user", "at": 1760550000000, "thread": Thread }
```

`status` is one of `open | working | answered | resolved`. Timestamps are Unix milliseconds.

## Anchors

The daemon stores anchors as opaque JSON. Surfaces define them, agents read them. Kinds in use:

```jsonc
{ "kind": "text", "page": "/", "selector": "main > p:nth-of-type(2)",
  "quote": "ship the importer in March", "prefix": "We will ", "suffix": ", then harden" }
{ "kind": "element", "page": "/", "selector": "#revenue-chart" }
{ "kind": "app", "key": "pricing_table", "label": "Plans table", "page": "/" }   // app-defined (the Streamlit binding)
{ "kind": "region", "page": "/", "selector": "#chart", "ex": 0.5, "ey": 0.3,   // point inside the element (fractions)
  "x": 0.42, "y": 0.18, "px": 640, "py": 910,                                  // viewport fractions, page px (fallback)
  "image": "att_…" }                                                           // optional screenshot crop
```

Text anchors follow the W3C *TextQuoteSelector* idea: an exact quote plus context, scoped by a CSS selector. `locate()` tries, in order:

| `match` | How |
|---|---|
| `exact` | the quote in the selector's visible text (not `<script>`/`<style>`), preferring the occurrence whose prefix matches |
| `moved` | the quote elsewhere in `<body>` (the selector changed) |
| `fuzzy` | the quote was edited: the text between its old `prefix` and `suffix` (full context, then the nearest 12 characters; at most 4× the quote length or 400 characters apart) |
| `none` | detached: the overlay shows the thread with its original quote and a `detached` tag (`text changed`, in a neutral colour, once the thread is answered or resolved: the agent usually changed that text on purpose) |

Element anchors are `exact` when the selector matches, else `none`. Region anchors are `exact` at their point inside the element when the selector matches, else `moved` to their old page point (`px`, `py`); both carry `point` in page pixels.

**Images.** A thread submitted with `image` stores it as an attachment in the same transaction and adds `"image": "att_…"` to its anchor (which must then be a JSON object). The overlay crops 480×320 CSS px around the point when it lands on a `<canvas>`, an `<svg>` or an `<img>` (not for cross-origin pixels). Plain HTML has no screenshot: browsers can't capture the DOM without a permission prompt. The overlay re-locates after the page's DOM changes (a `MutationObserver`; it never mutates the page).

## Overlay rendering

Agent messages render as a safe markdown subset (`web/markdown.js`): the body is HTML-escaped first, then paragraphs, line breaks, `**bold**`, `*italic*`/`_italic_`, `` `code` ``, fenced code blocks, `-`/`1.` lists, `#` headings (as bold lines) and `[text](url)` links with `http(s):`/`mailto:` only. User messages are plain text.

## `client.js` API

Load it with data attributes, and `window.Redline.client` is created automatically. (The daemon serves `/client.js` as `web/anchors.js` + `web/client.js`, and `/overlay.js` as the overlay's files concatenated in the order listed in `src/daemon/mod.rs`, ending with `web/overlay.js`.)

```html
<script src="http://127.0.0.1:7465/client.js" data-redline-artifact="art_…" data-redline-token="…"></script>
```

```js
const { Client, anchors, client } = window.Redline;   // also fires window "redline:ready"

await client.threads();                               // Thread[]
await client.comment(anchor, "make this bigger");     // new thread, sent now
await client.comment(regionAnchor, "greener", image); // image: {mime, data} (base64), optional
await client.attachmentURL("att_…");                  // object URL of a screenshot (fetched with the token)
await client.reply(threadId, "also the legend");
await client.setStatus(threadId, "resolved");
await client.update();                                 // UpdateInfo: is a newer redline out?
await client.dismissUpdate({ version: "0.3.0" });      // or { all: true }: notices off
await client.submit({ threads: [...], replies: [...] });

client.addDraft({ anchor, body });                     // drafts: localStorage only
client.addDraft({ thread_id, body });                  // reply draft
client.drafts(); client.removeDraft(id);
await client.submitDrafts();                           // all, or submitDrafts([ids])

client.on("event", (ev) => …);                         // live: replies, status changes
client.on("open" | "close" | "drafts", …);
client.connect();                                      // WebSocket, auto-reconnect with backoff

anchors.fromSelection();                               // text anchor from the current selection
anchors.fromElement(el, { extra: 1 });                 // element anchor
anchors.fromPoint(x, y);                               // region anchor at a viewport point
await anchors.snapshot(el, x, y);                      // {mime, data} crop of a canvas/svg/img, or null
anchors.locate(anchor);                                // {target: Range | Element | null, match: exact|moved|fuzzy|none}
anchors.resolve(anchor);                               // locate(anchor).target
```

With `overlay.js` loaded, `window.Redline.overlay` turns the UI off and on (a clean page for screenshots or reading; remembered per artifact in this browser, like the dock's mark and Alt+Shift+R):

```js
Redline.overlay.hide(); Redline.overlay.show(); Redline.overlay.toggle(); Redline.overlay.hidden; // → boolean
```

`new Client({server, artifact, token})` works too, for custom UIs or bundlers. React: `bindings/react/useRedline.js` wraps a client in a hook (`{threads, drafts, connected, error, comment, reply, setStatus, resolve, addDraft, removeDraft, submitDrafts, refresh}`).

## MCP tools (`redline mcp`)

| Tool | Args | Returns |
|---|---|---|
| `register_artifact` | `title`, `uri?`, `kind?`, `id?` (re-attach) | `{artifact_id, embed, session, since, next, update?, update_hint?}`: `session` and `since` (latest event seq) let a background `redline wait --session S --since N` start from now; `update` (`UpdateInfo`) and `update_hint` only when there's a release to mention |
| `wait_for_comments` | `artifact_id?`, `timeout_s?` (120, ≤540), `since?` | `{events, next, timed_out}` |
| `list_threads` | `artifact_id`, `status?` | `Thread[]` |
| `reply` | `thread_id`, `body`, `status?` | `Event` |
| `set_status` | `thread_id`, `status`, `note?` | `Event` |
| `get_image` | `attachment_id` | an MCP image content block (the region screenshot) |

- **Results** are pretty-printed JSON in a text content block. Errors come back as `isError` results.
- **Waits ride through a daemon restart** (an update replaces the daemon): `wait_for_comments`, `redline wait` and the channel push reconnect, starting a daemon if needed, and keep waiting from the same cursor until the timeout.
- **The server keeps a per-process cursor,** so `wait_for_comments` with no `since` continues where the last call stopped.
- **Session id:** `$REDLINE_SESSION`, or a random `ses_…` per process.
- **Capabilities:** `tools`. With `--channel` / `REDLINE_CHANNEL=1`, also `experimental["claude/channel"]: {}` (and protocol revisions only up to 2025-11-25, which Claude Code needs for channels), and the server sends one notification per new user comment on its session's artifacts:

```json
{"jsonrpc":"2.0","method":"notifications/claude/channel",
 "params":{"content":"New redline comment on \"<quote>\": <latest message>","meta":{"artifact_id":"art_…","thread_id":"thr_…","seq":"42","status":"open"}}}
```

## CLI

```
redline register <title> [--uri U] [--kind K] [--id ID] [--session S]   → {artifact_id, embed, session, since, update?}
redline wait [--artifact A] [--session S] [--since N] [--timeout 300]  → {events, next}
redline list [<artifact>]                                              → artifacts | threads
redline reply <thread> <body> [--status S]
redline status <thread> <status> [--note N]
redline status                                                         → {running, port, pid, version, uptime_s, artifacts, threads, by_status, events, update}
redline stop                                                           → {stopped: pid | null}
redline image <attachment> [--out PATH]                                → {path, mime, bytes}
redline snippet <artifact>                                             → embed HTML
redline strip <path>… [--check]                                        → {files_checked, stripped|found: [{path, removed}]}; --check exits 1 if found
redline update                                                         → {updated, from, to, version, notes, next}: clone → latest release, rebuilt
redline update check                                                   → UpdateInfo + {config, error?}: checks the remote now
redline update skip [X.Y.Z]   ·   redline update off|on                  → UpdateInfo; writes config.toml (skip default: the latest release)
redline serve [--port P]   ·   redline mcp [--channel]
redline install|uninstall|doctor [--agents a,b] [--claude-config-dir D]… [--skill-dir D]… [--bin-dir D] [--force] [--no-mcp]
```

The embed is wrapped in `<!-- redline:begin … -->` / `<!-- redline:end -->`. `strip` removes those blocks and any other `<script>` that loads redline (`data-redline-*`, or `/client.js` / `/overlay.js` from `127.0.0.1`/`localhost`), in html/jsx/tsx/vue/svelte/astro/md/templates; directories are walked, skipping dot-dirs, `node_modules`, `target`, `dist`, `build`.

All output is JSON, except `snippet` and the install commands. On a terminal (stderr is a TTY), commands other than `serve`, `mcp` and `update` print an update notice to stderr after their output when `notify` is true. `status` and `stop` never start the daemon (`status` reports `{running: false}`); every other command starts it if needed. Environment: `REDLINE_HOME`, `REDLINE_PORT`, `REDLINE_SESSION`, `REDLINE_CHANNEL`, `REDLINE_REPO`, `REDLINE_CONFIG` (config file path), `REDLINE_NO_UPDATE_CHECK=1` (no background release check; `CI` does the same), `REDLINE_UPDATE_REMOTE` (where to look for release tags; default the clone's `origin`), and `RUST_LOG` for the daemon log.

### `config.toml`

`$REDLINE_CONFIG`, else `$XDG_CONFIG_HOME/redline/config.toml`, else `~/.config/redline/config.toml`. Missing file or key = defaults. redline edits it with `toml_edit`, so comments stay; the first edit writes a commented template.

```toml
[update]
check = true      # daily release check + notices; false = neither
skip = []         # releases ("X.Y.Z") not to notify about
```
