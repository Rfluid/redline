# Brand

## Name

Always lowercase: **redline**, also at the start of a sentence and in titles. The CLI is `redline`. Don't write "Redline", "RedLine" or "RL" in copy.

## Logo

| File | Use on |
|---|---|
| `logo/wordmark.svg` | Light backgrounds: README header, docs |
| `logo/wordmark-dark.svg` | Dark backgrounds |
| `logo/mark.svg` | App icons, the overlay dock on light pages, avatars (ink tile) |
| `logo/mark-light.svg` | The overlay dock on dark pages (paper tile) |
| `logo/favicon.svg` | 16–24 px: the letter drops out and only the stroke stays |

**Wordmark:** "redline" in Newsreader Semibold (opsz 72), tracked −2 at 92 px, with one hand-drawn stroke across the x-height. The stroke rises slightly to the right and overshoots both ends.

**Mark:** a struck lowercase "r" on a 64-unit tile with a 14-unit corner radius.

The letters are outlined paths: the SVGs need no font. Regenerate them with `logo/build.py` (see its header) when the stroke or tile changes. Never retype the wordmark in a live font.

### Rules

- **The stroke is always red** (`#D7261E` on light, `#FF5A4E` on dark). The letters are always ink or paper, never red.
- **Clear space:** at least the height of the "r" (≈ 0.35 × the wordmark's height) on every side.
- **Minimum size:** wordmark 80 px wide; mark 20 px. Below 24 px use `favicon.svg`.
- **Don't:** straighten the stroke, recolor it, put it under the word (it goes *through* it), add effects (shadows, gradients, outlines), rotate the word, or set the wordmark on a busy image.

## Voice and copy

- **Plain and short.** Labels are verbs: *Comment*, *Send*, *Save draft*, *Discard*, *Reply*, *Resolve*.
- **Sentence case** everywhere: "Send 1 draft", not "Send 1 Draft".
- **People:** "You" and "Agent". Not "user", not the model's name.
- **Statuses** use the protocol's words, capitalized as labels: *Draft*, *Open*, *Working*, *Answered*, *Resolved*.
- **Anchor tags:** *Edited* (`fuzzy`), *Moved* (`moved`), *Text changed* (`none` once answered or resolved), *Detached: showing original quote* (`none` while open).
- **Errors say what happened and what's kept:** "Couldn't send. Your comment is kept as a draft." Never just "Error".
- **Counts:** "1 thread", "3 threads", "1 draft". Hide the count when it's 0.
- **Shortcuts** go in the UI as `<kbd>`: ⌥C to comment, ⌘↵ / Ctrl+Enter to send, Esc to cancel, ⌥⇧R to turn redline off or on. Show ⌘/⌥ on macOS and Ctrl/Alt elsewhere.
