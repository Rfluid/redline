# Typography

redline loads **no web fonts**: the overlay runs on loopback-served pages and must not fetch from the network, or flash. Everything uses system stacks. Newsreader exists only inside the logo SVGs, as outlines.

## Stacks

| Token | Stack | Use |
|---|---|---|
| `--rl-font-ui` | `system-ui, -apple-system, "Segoe UI", Roboto, sans-serif` | All UI |
| `--rl-font-quote` | `"Iowan Old Style", "Charter", "Georgia", "Cambria", serif` | **Only** quotes of the commented text |
| `--rl-font-mono` | `ui-monospace, "SF Mono", Menlo, Consolas, monospace` | Code in agent replies, `<kbd>` |

The serif is the editorial accent. It marks "this is the artifact's text", so never set UI labels or messages in it.

## Scale

| Token | Size / line | Weight | Use |
|---|---|---|---|
| `--rl-text-quote` | 15.5 / 22, italic | 400 | Quote line in popover and drawer |
| `--rl-text-body` | 14 / 20 | 400 | Messages, inputs |
| `--rl-text-ui` | 13 / 18 | 500 | Buttons, dock |
| `--rl-text-label` | 12 / 16 | 600 | Author line, chips, marker numerals |
| `--rl-text-hint` | 11.5 / 16 | 400 | Shortcut hints, timestamps |
| `--rl-text-code` | 12.5 | 400 | Inline code |

Phone (`max-width: 600px`) bumps body to 15 / 22 and UI to 14.

## Rules

- **Two weights:** 400 and 600 (500 only for button labels). No bold-italic.
- **Author line:** name in 600 ("You", or "Agent" in `--rl-agent`), then `· 3 min` in `--rl-muted`.
- **Quotes** are wrapped in curly quotes “…”, truncated to one line with an ellipsis, and the quoted words carry a 1.5 px `--rl-mark` underline.
- **Agent markdown** renders within this scale: headings become 14 / 600 (no big headings inside a popover); lists keep a 18 px indent.
- Keep `font-feature-settings: "tnum"` on counts and marker numerals so the digits don't jump.
