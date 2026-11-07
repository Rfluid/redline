# 2026-11-07 · markdown-rendering

**Agent:** claude  **Roadmap:** none (user request)

## Done
- `web/markdown.js` rewritten as a line-based block parser plus an inline tokenizer, close to CommonMark/GFM. Before, a chunk was a list only if *every* line was an item, so `Changes:\n- a\n- b` showed literal dashes. Now: lists under a line, nested (2-space under `1.` too), loose, `start=n`, task items, lazy lines, fences inside items; `>` quotes; `---` rules; setext headings; pipe tables with alignment; `~~strike~~`, `***both***`, proper nesting of `**`/`*`, any backtick run, `\` escapes, `<url>` and bare URLs (trailing punctuation trimmed).
- Safety by construction: literal text is escaped as emitted; links still http(s)/mailto only. Linear on hostile input at the 64 KB cap (failure caches for delimiter/backtick searches, bounded link regex, linear `trimUrl`, depth cap 12). Before these fixes, unmatched `*` took 10 s and a 32k-column table took 101 s.
- Overlay layout fix: the unnamed wrapper around `.who`/`.body` in `popover.js` (now `.mtx`, `flex: 1; min-width: 0`) let wide content stretch the popover sideways. Table cells use `overflow-wrap: normal` so they don't break letter by letter; `.tbl` scrolls.
- Docs: PROTOCOL, SKILL, DECISIONS (new entry), `.design/` typography + components, CHANGELOG; manual-browser-test now binds `http.server` to 127.0.0.1.

## Verified
- `tests/bindings/markdown.test.mjs`: 4 new tests (lists, blocks, inline, pathological timing) + more XSS payloads. Browser test step extended (nested list, task, quote/del, table rows, wide table scrolls without widening the popover).
- Live in Chrome (chrome-devtools MCP), sandboxed daemon: a realistic reply and an edge-case reply, light, dark, phone sheet.

## Open / next
- A disabled checkbox reads greyed out; a custom-drawn box in `--rl-agent` would match the design better.
- Once, `tests/browser/overlay.test.mjs` printed every step `ok` and then never exited (`just check` hit an outer timeout). It didn't come back in three standalone runs or the next full `just check` (exit 0). If it returns, look at the harness's Chrome/daemon teardown.
- Lazy continuation follows CommonMark: a line right after a list item (no blank line) joins that item. Correct, but it can surprise.
