# Changelog

All notable changes to redline. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/) (before 1.0, a minor bump may change the protocol).

## [Unreleased]

## [0.1.0] - 2026-10-19

First public release.

### Added
- The review loop: the agent registers an artifact and puts one script tag in the page. You select text (or click a spot on a chart, canvas or image) and comment; the agent receives the comments over MCP (`wait_for_comments`) or the CLI (`redline wait`), changes the artifact, and answers inline under each comment. A spinner and a short note show while it works.
- Drafts stay in your browser until you send them, one at a time or as a batch.
- Region comments carry a cropped screenshot when the page can draw it; agents read it with the `get_image` MCP tool or `redline image`.
- The comment overlay: numbered markers in the margin, threads that open next to the text, and a small dock (thread count, new replies, drafts, connection). It matches the page (light or dark), works on phones, renders agent replies as sanitized markdown, keeps half-typed text across updates, offers Undo on resolve and Retry on a failed send, and tags comments whose text changed. It never changes the page itself.
- Keyboard: Alt+C to comment, Ctrl/⌘+Enter to send, Alt+Enter to save a draft, Alt+Shift+R to turn the overlay off and on (`Redline.overlay.hide()` / `show()` / `toggle()` from code).
- Streamlit binding (`bindings/streamlit/`): a comment button per widget, with drafts, following the app's theme. React binding (`bindings/react/`): `useRedline()` over a framework-agnostic store, plus an example with its own comment UI.
- `redline install` for Claude Code, Codex, opencode, Cursor, Antigravity and Gemini CLI: links the skill and registers the MCP server, backs up each config once, and keeps your key order and comments. `redline uninstall` restores an untouched config byte for byte; `redline doctor` checks it all.
- Every MCP tool has a CLI twin, so agents without MCP can do everything.
- Optional Claude Code channel push (`redline mcp --channel`): new comments reach a live session without a tool call.
- A local daemon on `127.0.0.1` only, with per-artifact tokens and comment history in SQLite. `redline status` and `redline stop`; clients replace a daemon older than them; waits survive a daemon restart. A daemon log (`RUST_LOG`) never contains tokens.
- `redline strip <file|dir> [--check]` removes embed snippets before you ship.
- Update notices: a daily release check, shown in the dock, the terminal and to your agent. `redline update` installs it; skip a version (`redline update skip`) or turn notices off (`redline update off`).
- Linux, macOS and Windows. Release binaries for Linux (x86_64, aarch64), macOS (arm64, x86_64) and Windows (x86_64, arm64); `bin/redline` downloads one when `cargo` isn't installed. Install and update use the latest release, not the development branch.

[Unreleased]: https://github.com/Rfluid/redline/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Rfluid/redline/releases/tag/v0.1.0
