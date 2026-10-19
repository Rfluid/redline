---
name: redline
description: Inline review loop between an artifact you built (HTML page, web app, Streamlit app, report) and this session. The user comments inline on the artifact; you receive the comments, change the artifact, and answer inline under each comment. Use when the user says "redline", "let me comment on it", "I'll review it inline", "open it for review", "wait for my comments", or runs /redline [start|wait|status] [artifact]. Handles only the communication; you build and serve the artifact as usual.
---

# redline

Connects an artifact you made to this session. The user selects text in the artifact, comments, and submits one comment at a time or a batch of drafts. Each comment reaches you as a thread. You answer inline, under the comment, and change the artifact.

redline does **not** render or serve the artifact. You do that as usual (a file, a dev server, Streamlit). redline adds a local broker (`127.0.0.1:7465`, started on demand), a small script the page loads, and these tools.

Tools come from the `redline` MCP server. If they aren't available, use the CLI, which does the same thing: `redline` on PATH, or `bin/redline` next to this file (Windows: `bin\redline.cmd`; `bin/redline` from Git Bash).

| MCP tool | CLI |
|---|---|
| `register_artifact(title, uri?, kind?, id?)` | `redline register <title> [--uri] [--kind] [--id] [--session S]` |
| `wait_for_comments(artifact_id?, timeout_s?, since?)` | `redline wait [--artifact A] [--session S] [--since N] [--timeout 300]` |
| `list_threads(artifact_id, status?)` | `redline list <artifact>` |
| `reply(thread_id, body, status?)` | `redline reply <thread> <body> [--status ...]` |
| `get_image(attachment_id)` | `redline image <attachment> [--out PATH]` |
| `set_status(thread_id, status, note?)` | `redline status <thread> open\|working\|answered\|resolved [--note N]` |

## Start a review

1. Build the artifact as you normally would.
2. Call `register_artifact` with a short title, and with `uri` set to the file path or URL. Keep the returned `artifact_id`. If you're re-attaching after a restart, pass `id` to keep the existing threads.
3. Put the returned `embed` snippet into the page, just before `</body>`. It loads `client.js` (the protocol) and `overlay.js` (the commenting UI).
   - **Plain HTML / any framework:** paste it as-is, or add it in the root layout or `index.html`.
   - **Already has its own comment UI:** load only the `client.js` tag and drive `window.Redline.client` (see `docs/PROTOCOL.md`).
   - **React app that should get its own comment UI:** copy `bindings/react/useRedline.js` and `redline-store.js` (next to this file) into the app, load only the `client.js` tag in `index.html`, and use `const rl = useRedline()` (`threads`, `drafts`, `comment(anchor, body)`, `reply`, `resolve`, `addDraft`, `submitDrafts`). Anchor app-level things as `{kind: "app", key}`. Example: `bindings/react/example.html`.
   - **Streamlit:** don't paste the snippet. Use the binding in `bindings/streamlit/` next to this file (see "Streamlit apps" below).
   - **Snippet is temporary:** it holds a token and points at localhost. Before shipping, committing or sharing the artifact, run `redline strip <file or dir>` (it removes every redline snippet and nothing else; `--check` exits 1 if one is left, for CI or hooks).
   - **Clean view:** the user can turn redline off on the page (click the redline mark, or Alt+Shift+R) to see it without markers or highlights. For your own screenshots of the page, call `Redline.overlay.hide()` first (the snippet stays; `redline strip` is still what removes it).
4. Tell the user where to open the artifact and how to comment: select text, click **Comment**, then **Send** now, or **Save draft** and later **Send drafts** in the redline dock (bottom right).

## Streamlit apps

The overlay can't see into Streamlit, so each widget the user should review gets a comment button.

1. `register_artifact(title, kind: "streamlit")`.
2. In the app, after each widget to review:
   ```python
   import redline_streamlit as rl
   rl.setup()                                   # reads REDLINE_EMBED
   st.metric("MRR", "$42k"); rl.comment("mrr", label="MRR metric")
   ```
   Keys must be stable across reruns (a name, not a counter).
3. Launch with the snippet in the environment, not in the source:
   `REDLINE_EMBED='<embed>' PYTHONPATH=<this skill dir>/bindings/streamlit streamlit run app.py`
   (or `pip install -e <this skill dir>/bindings/streamlit`).
4. The user can Send per widget, or Save draft across widgets and Send drafts. Comments arrive with `anchor: {kind: "app", key, label}`. Change the widget with that key. Without `REDLINE_EMBED`, `rl.comment` renders nothing, so the calls can stay.

Example: `bindings/streamlit/example_app.py`.

## Review loop

Repeat until the user says they're done:

1. Call `wait_for_comments` (no `since`; the server tracks the cursor). It blocks until comments arrive or it times out.
   - **Timed out with nothing new:** call it again. Don't report each timeout to the user.
   - **Background shell instead (Claude Code's background Bash, or any agent that is notified when a background command exits):** see "Waiting in the background" below.
2. For each event, read the thread: `anchor` (what they pointed at: `quote`, `selector`, `key`) and `messages` (the conversation, oldest first). The newest user message is the request.
   - **Region comments** (`anchor.kind: "region"`): the user clicked a spot, at `ex`/`ey` (fractions) inside the element `selector`. If `anchor.image` is set, call `get_image(attachment_id)` (CLI: `redline image <id> --out shot.png`) to see what they pointed at.
3. Classify and act:
   - **Change request:** `set_status(working)`, with a short `note` if the change takes a while ("Rebuilding the chart"); the user sees a spinner and the note. Edit the artifact. Then `reply` with one or two lines on what you changed. The thread becomes `answered`.
   - **Question:** `reply` with the answer. Don't edit unless asked.
   - **Unclear:** `reply` with your question. Leave the thread `open` by passing `status: "open"`.
   - **Several comments in one batch:** handle them all, and reply to each thread separately.
4. The page doesn't refresh itself unless your dev server hot-reloads: after editing a static file, say "reload to see it" in the reply.
   Replies show up live under the user's comment. Keep them short; the change itself is the main answer. Markdown works: **bold**, *italic*, `code`, fenced code, lists, [links](https://…) (http/https/mailto only). No HTML.
5. Don't resolve threads yourself unless the user asked you to. Resolving is their call.

When the artifact is regenerated or reloaded, the threads remain. Text anchors re-attach by quote and context. If you edit the quoted text, the overlay finds it by its surrounding text and tags the thread `edited`; if the text is gone, the thread shows as `detached` with the original quote. Say what you changed in your reply.

## Comments pushed to you (Claude Code channels)

If the redline server runs as a channel (its instructions say so), new comments arrive by themselves as `<channel source="redline" artifact_id="…" thread_id="…">` messages. Don't call `wait_for_comments` then. Handle each like an event from it: `list_threads` if you need the whole thread, `set_status(working)`, make the change, `reply` on that `thread_id`.

## Waiting in the background

Use this when your agent can run a shell command in the background and gets notified when it exits. You stay free to talk to the user while you wait.

1. `register_artifact` returns `session` and `since`. (CLI: `redline register … --session <S>` prints them too.)
2. Run in the background: `redline wait --session <session> --since <since> --timeout 600`.
3. When it exits, read its JSON: `events` (same shape as `wait_for_comments`) and `next`.
   - Events: handle them as in the review loop.
   - No events (timeout): nothing happened.
4. Start the next background wait with `--since <next>`. Repeat until the user is done.

Use one cursor: either this recipe or `wait_for_comments`, not both at once, or each sees only part of the stream.

## Status

`list_threads(artifact_id)` gives the full picture. Use it after a restart, or when the user asks "what's left?". Open threads whose last message is from the user still need you.

## Updates

If `register_artifact` (or `redline register`) returns an `update` field, a newer redline is out. Tell the user once per session, in one line: "redline <latest> is available (you have <current>). Want me to update it?" Don't update without their OK, and don't let it hold up the review.

- **They say yes:** run `redline update`. It moves the clone to the latest release and rebuilds. The daemon restarts by itself; your MCP server stays on the old version until the session restarts, so tell them to restart the session when convenient.
- **Not this version:** `redline update skip` (a newer release still shows).
- **Never:** `redline update off` (`redline update on` undoes it). Both are saved in `~/.config/redline/config.toml`.

`redline update check` checks now and prints what's known.
