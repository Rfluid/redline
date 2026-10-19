# Prompt: test a clean install of redline

Give this to an agent **after** the user has installed redline with the README sentence:

> Install redline from https://github.com/Rfluid/redline for all my coding agents. Follow its `INSTALL.md`.

````
Test the redline install on this machine. The user just installed it from github.com/Rfluid/redline with its INSTALL.md (`redline install` for every detected agent). Verify it really works end to end, for every agent, and report. This is the user's real install: don't run `redline install` / `uninstall`, don't edit agent configs, and don't edit the redline clone. If something is broken, diagnose it and propose the fix; don't apply it. Use temp dirs (`mktemp -d`) for anything you create.

1. Static checks
   - `which redline`, `redline --version` (prints `X.Y.Z (<git sha>)`), `redline doctor`. Every detected agent must show `skill …=ok` and `mcp …: ok`. Note anything `missing`. A Cursor `note` that redline is also loaded from `~/.claude/skills` is expected and harmless.
   - How was it built? If `cargo` isn't installed, `bin/redline` downloaded a release binary into `<clone>/target/prebuilt/` (it needs `gh` logged in while the repo is private). Say which path was used.
   - `redline status`: `{running: false}` is fine before first use. Run `redline list`, then `redline status` again: it shows port, pid, version, uptime and counts.
   - Each config that `doctor` lists must hold exactly one `redline` entry pointing at `<clone>/bin/redline mcp`, with the other servers intact. Diff each against the `<file>.redline.bak` next to it and report what changed: only the redline entry should differ (key order and comments kept; in Codex's `config.toml` the table comes after any `# END …` marker of another tool). Claude's backups are `~/.claude.json.redline.bak` (and `<profile>/.claude.json.redline.bak`); don't print their contents, they hold secrets.
   - Expected layout:
     - skills: `~/.claude/skills/redline` (Claude), `~/.agents/skills/redline` (shared by Codex, opencode, Cursor, Gemini CLI), `~/.gemini/config/skills/redline` and `~/.gemini/antigravity-cli/skills/redline` (Antigravity app/IDE and `agy` CLI). No `~/.cursor/skills/redline` or `~/.gemini/skills/redline` (older locations).
     - MCP: Claude via `claude mcp` (user scope); Codex `[mcp_servers.redline]`; opencode `mcp.redline` (`type: local`); Cursor `mcpServers.redline` with `"type": "stdio"`; Antigravity `~/.gemini/config/mcp_config.json`; Gemini CLI `~/.gemini/settings.json`. Gemini CLI and Antigravity are separate agents with separate configs.
   - Each agent's own listing must show redline: `claude mcp get redline`, `codex mcp list`, `opencode mcp list`, `agy mcp list`, `cursor-agent mcp list-tools redline` (6 tools), gemini via its config. Kill any `opencode serve` that `opencode mcp list` leaves behind (only ones you started).
   - Second Claude profile: ask the user whether they use one (e.g. `~/.claude-work`). If so, check `CLAUDE_CONFIG_DIR=<dir> claude mcp get redline` and `<dir>/skills/redline`.

2. Full MCP loop per agent: `<clone>/tests/agent-loop.sh`
   - Run it once per installed agent: `tests/agent-loop.sh claude`, then codex, opencode, agy, cursor-agent, gemini. Then `tests/agent-loop.sh claude opencode agy` once, to check that several sessions share one daemon. Run `tests/agent-loop.sh` without arguments for its options.
   - It starts the agent headless, which registers an artifact over MCP and waits; the script waits until `redline status` shows the agent listening, submits a comment the way the browser does, and passes when the agent's MCP reply is in the thread. It cleans up after itself and keeps logs on failure.
   - Second Claude profile: `CLAUDE_CONFIG_DIR=<dir> tests/agent-loop.sh claude`.
   - Known setup issues that aren't redline's fault (report them, don't fix them):
     - Codex fails with "not supported when using Codex with a ChatGPT account": the model pinned in `~/.codex/config.toml` is outdated. Rerun with `CODEX_MODEL=<slug>` from `~/.codex/models_cache.json`.
     - cursor-agent "Press any key to sign in": not logged in, so it can't run headless.
     - Gemini CLI `IneligibleTierError … no longer supported for Gemini Code Assist for individuals`: the account's tier was moved to Antigravity.

3. Browser loop (needs the chrome-devtools MCP)
   - Write a small page in a temp dir: a heading, two paragraphs and a `<canvas>` with a few bars drawn by a script. `redline register "clean-install browser test" --uri http://127.0.0.1:<port>/page.html`, put the returned `embed` just before `</body>`, serve the dir with `python3 -m http.server <port> --bind 127.0.0.1`.
   - Start a headless Claude that waits on it: `claude -p "... loop: wait_for_comments(artifact_id=<id>, timeout_s=300); for each user comment: set_status(thread, working, note 'Looking at it'), run sleep 6, then reply '**browser-ack**: <comment>' plus a two-item list and the text <img src=x onerror=alert(1)> [bad](javascript:alert(1)); if the anchor has an image, call get_image and mention its size; stop on 'done'" --allowedTools mcp__redline "Bash(sleep:*)"`. Wait until `redline status` lists `artifact:<id>` under `waiting`.
   - In Chrome check each of these (select text by evaluating a Range and dispatching `mouseup`; the overlay's UI is in the open shadow root of `[data-redline]`):
     - select → **Comment** → **Send**; the thread shows `working` with a spinner and "Looking at it", then the reply live, `answered`.
     - the reply's markdown renders (bold, list) and the hostile part is plain text: no `img`, `script` or `a` element in the message, no dialog.
     - two comments with **Add to drafts**, then **Submit all (2)**: both reach the agent in one batch.
     - **Alt+C** with a selection opens the comment box; **Esc** closes it; **Ctrl+Enter** sends.
     - **📍**, then click on the canvas: a region comment with a screenshot thumbnail and a pin on the chart; the agent's reply mentions the image.
     - edit the commented text in the DOM (keep its surroundings): the thread is tagged `edited`; remove it: `text changed` (answered) or `detached` (open), with the original quote.
     - **Reply** in a thread sends and empties the box; **Resolve** works.
     - dark mode (emulate `prefers-color-scheme: dark`): the panel turns dark. Phone width (390 px, mobile): the panel is full width.
     - nothing is added to the page's DOM (no `<style>` in `<head>`), and the console has no errors or CORS warnings.
   - `redline image <att_id> --out <tmp>/shot.png` writes a PNG.
   - Send "done" to end the agent.

4. Skills
   - In a fresh Claude session the `redline` skill must be listed. For the others, ask each one headless "Which skills do you have? Answer with names only." and check that redline is listed (once).

5. Snippet hygiene
   - Copy your test page, run `redline strip --check <copy>` (exits 1), `redline strip <copy>` (removes it), `redline strip --check <copy>` (exits 0). Nothing else in the file may change.

6. Clean up what you created
   - Stop the processes your tests started (the http server, the headless agents, any `opencode serve` you started). Delete your temp dirs. Leave `~/.local/state/redline`, the clone and the user's configs alone.

Report a table: agent × (skill, MCP config, MCP listing, agent-loop, notes). Then list the browser checks (pass/fail), and the problems, each with its evidence and a proposed fix (in redline or in the user's setup). Disk: a source build needs about 600 MB free.
````
