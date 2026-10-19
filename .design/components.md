# Components

Every value here is a token from `tokens.css`. Sizes are CSS px. On phones every interactive target grows to 44 px (see accessibility.md).

## Icons

Inline stroke SVG, 24-unit viewBox, `stroke="currentColor"`, 2 px stroke, round caps and joins, drawn at 14–16 px. The set:

| Name | Path | Used for |
|---|---|---|
| pen | `M4 20l4-1 11-11-3-3L5 16z` | Comment chip, agent avatar |
| spot | `circle 12,12 r7` + `M12 2v4M12 18v4M2 12h4M18 12h4` | Comment on a spot (replaces 📍) |
| check | `M5 12.5l4.5 4.5L19 7.5` | Resolve, resolved |
| close | `M6 6l12 12M18 6L6 18` | Close (replaces ×) |
| send | `M5 12h14M13 6l6 6-6 6` | Send reply, send drafts |

Decorative icons get `aria-hidden="true"`. Icon-only buttons get `aria-label`.

## Margin marker

A 26 px circle with the thread's number (label type, `tnum`). It sits in the margin beside its highlight (see layout.md) and is a real `<button>` labelled "Thread 2, answered".

| State | Look |
|---|---|
| Draft | transparent, 2 px **dashed** `--rl-mark` ring, red numeral |
| Open | `--rl-raised` fill, 2 px solid `--rl-mark` ring, red numeral |
| Working | `--rl-raised` fill, 2 px `--rl-agent` ring, blue numeral, plus the working note pill |
| Waiting (any thread whose next move is the agent's: `working`, or `open` with your message last) | the state look plus a **comet**: a 1.5 px arc 6 px outside the ring, fading in from transparent over 65% of the turn, in the marker's color (red while it waits, blue once the agent works), turning once per 1.1 s. Static under reduced motion. Labelled "Thread 2, waiting for the agent" while open |
| Answered | solid `--rl-agent` fill, white numeral |
| Resolved | 18 px `--rl-line` dot with a check; hidden unless the drawer's Resolved tab is open |
| Spot pin (region anchor) | same as its state, with the bottom-left corner squared off (4 px radius) so it points at the spot |
| Moved / detached | the state look plus a 2 px gap in the ring at 45°; the tag in the popover explains it |

Hover or focus: a 4 px halo in the state's tint (`box-shadow: 0 0 0 4px`), and the highlight switches to *focused*. The **working note pill** sits right of the marker (left of it when there is less than 200 px of room): 22 px tall, `--rl-surface`, `--rl-line` border, three 4 px blue dots pulsing in sequence, then `Agent: "<note>"` truncated to 40 characters.

## Highlight

See color.md › Highlights. It is the only thing redline draws in the page's own text.

## Selection chip

Appears 8 px above the end of a text selection. Height 32, radius 8, `--rl-primary` background, `--rl-on-primary` text, pen icon in `--rl-mark`, label "Comment" and a `<kbd>⌥C</kbd>`. Clicking it opens the compose popover. It hides as soon as the selection collapses.

## Compose popover

Width 380 (`--rl-pop-w`), radius 12, `--rl-surface`, 1 px `--rl-line`, `--rl-shadow-pop`.

1. **Quote line:** the selected text (typography.md). For a spot, the spot icon plus the element name instead.
2. **Textarea:** min 84 px, `--rl-raised`, radius 9, placeholder "Comment for the agent…". Focused: border `--rl-focus` and a 3 px ring in `--rl-agent-tint`.
3. **Footer:** hint "⌘↵ send · Esc cancel" (left, `--rl-hint`), then **Save draft** (outline) and **Send** (primary).

Screenshot thumbnails for spots sit under the quote line: max-height 96, radius 6, 1 px `--rl-line`.

## Thread popover

Same frame as compose, width 360. It opens from a marker click, from a drawer row, or by itself when the agent replies to a thread you have open (it never opens by itself for other threads: the dock badge tells you instead).

1. **Header** (border-bottom): `#2` in label/muted, status chip, spacer, **Resolve** (outline, check icon), close (ghost, icon only).
2. **Quote line**, then an anchor tag if `match ≠ exact`.
3. **Messages:** a 22 px avatar (You = `--rl-mark` disc with "Y"; Agent = `--rl-agent` disc with the pen icon), the author line, then the body. 14 px gap between messages. Agent bodies render the sanitized markdown.
4. **Where it stands** (a waiting thread, see Margin marker), below the last message, like a chat's typing row: the 22 px agent avatar, a 22 px bubble (radius 11, bottom-left 4) with three pulsing dots, then 13 px italic text. Open: "Sent · waiting for the agent" in `--rl-muted`, the avatar hollow (1.5 px `--rl-agent` ring, 55% opacity), bubble `--rl-code`, grey dots. Working: the agent's note, else "Agent is working on it", in `--rl-agent` with the solid avatar and an `--rl-agent-tint` bubble. Gone once the agent replies. Sending a comment or reply also announces "Comment sent. Waiting for the agent." to screen readers.
5. **Reply row:** a textarea that starts one line tall and grows to 120 px (radius 9, `--rl-raised`), with a 30 px **send** icon button (primary) inside it on the right. Under it, the hint "⌘↵ send · ⌥↵ save as draft" (hidden on touch screens). Reply drafts show above it, dashed, with Discard / Send.

**Resolve** closes the popover and shows a toast "Thread N resolved." with **Undo**. Resolved threads (opened from the drawer) show **Reopen** (outline) in place of Resolve.

## Status chip

Pill: 11.5–12 px label type, padding 3×8 (4×9 on phone), full radius, a 6 px glyph before the word.

| Status | Text / fill | Glyph |
|---|---|---|
| Open | `--rl-mark-fg` on `--rl-mark-tint-draft` | hollow ring |
| Working | `--rl-agent` on `--rl-agent-tint` | hollow ring |
| Answered | `--rl-agent` on `--rl-agent-tint` | filled dot |
| Resolved | `--rl-done` on `--rl-done-tint` | check |

## Anchor tag

Square-ish (radius 6), 12 px. *Edited*, *Moved* and *Text changed*: `--rl-muted` text, 1 px `--rl-line` border. *Detached*: `--rl-note-fg` on `--rl-note-tint`, `--rl-note-line` border. The title attribute keeps today's explanations.

## Buttons

| Kind | Look | Use |
|---|---|---|
| Primary | `--rl-primary` fill, `--rl-on-primary` text, 600 | One per surface: Send, Send N drafts |
| Outline | `--rl-raised` fill, 1 px `--rl-line` | Save draft, Resolve, Reopen, Retry |
| Ghost | transparent, `--rl-muted` text | Discard, Close |
| Icon | 28–34 square, outline or ghost | Spot, close, send |

Height 34 (28 in dense headers), radius 8, padding 0 12–14. Hover: primary lightens 8%, outline and ghost get a `--rl-line-soft` fill. Pressed: translateY(0.5px). Disabled: 45% opacity, no pointer. Focus: see accessibility.md. **No red buttons.**

## Dock

The always-there entry point, replacing the vertical "redline" tab. A pill fixed 24 px from the bottom-right: `--rl-surface`, 1 px `--rl-line`, `--rl-shadow-dock`, padding 6, items 36 tall.

`[mark] N threads · M new replies` · `[spot]` · `[Send K drafts →]` · `● Live`

- The mark is `mark.svg` on light pages and `mark-light.svg` on dark ones, 24 px.
- The summary is a button that opens the drawer, styled as a ghost button so it sits in the dock rather than on it: the count in `--rl-fg` 600, the noun in `--rl-muted`, symmetric 12 px padding, and a `--rl-line-soft` fill only on hover or while the drawer is open. Show "· M new replies" only when there are unseen agent replies (seen = the thread was opened; remembered per artifact in localStorage), in `--rl-agent` 600.
- **Spot** (ghost icon button): comment on a spot (a chart, an image). It starts the crosshair picker.
- **Send K drafts** (primary) appears only when K > 0.
- **Connection:** "Live" with a `--rl-done` dot. Disconnected: "Reconnecting… comments are kept" in `--rl-note-fg`, with a hollow `--rl-note` dot. Never a bare unlabeled dot.
- **Empty** (no threads, no drafts): mark plus "Select text to comment, or press ⌥C" in `--rl-muted`. After the first comment it collapses to the summary.
- **Off (clean view):** clicking the mark, or Alt+Shift+R, turns redline off: markers, highlights, popovers, chip, drawer and toast go; the dock shrinks to the mark at 35% opacity (100% on hover or focus), with a 9 px `--rl-agent` dot when replies arrive meanwhile. Remembered per artifact in localStorage. `Redline.overlay.hide()/show()/toggle()` do the same from code.

### Update notice

When a newer release is known and not dismissed, the dock gets an **Update** chip before the connection label: 28 tall, pill, `--rl-note-tint` fill, `--rl-note-fg` 600 12 px text, a 6 px `--rl-note` dot. Amber, not red or blue: it's about redline, not the review. Hidden in the clean view.

The chip opens a card where the toast sits (24 px right, 84 px bottom; above the dock on phones, full width): `--rl-surface`, 1 px `--rl-line`, radius 12, `--rl-shadow-pop`, width 320.

`redline 0.2.0 is available  [×]` · "You have 0.1.0. Run `redline update`, or ask your agent to update redline." · `[What's new]` … `[Later] [Skip 0.3.0]` · "Turn off update notices" (12 px muted underlined text button)

- **What's new** links to the release notes (new tab). **Later** hides the chip for a day in this browser. **Skip** and **Turn off** are saved for the CLI and agents too, then confirmed with a toast ("Skipped 0.3.0. A newer release will still show." / "Update notices are off. To turn them back on: redline update on").
- `role="dialog"`, focus moves to its first button, Esc closes it.

## Drawer (all threads)

Opens from the dock summary. A 360 px sheet on the right, full height, `--rl-surface`, over the page with **no** backdrop (the page stays usable).

- **Header:** "Threads", a segmented control *Open N* / *Resolved N*, spacer, **Send K drafts** (primary, when K > 0), close.
- **Rows:** 22 px marker, the quote (serif italic, one line), then the state on the right ("Draft", "Working", "1 new reply" in `--rl-agent` 600). Clicking a row scrolls the page to the anchor, closes the drawer and opens that thread's popover.
- Drafts list first, then threads in page order.
- **Empty:** "No comments yet. Select any text on the page to start."

## Toast

Replaces `alert()`. It appears above the dock: `--rl-primary` fill (inverted), radius 10, 13 px, a 7 px `--rl-mark` dot, the message, and an optional **Retry** (outline on inverted). `role="status"`; errors use `role="alert"`. It auto-hides after 6 s unless it has an action.

## Scrollbar

Every scrolling area in the overlay (drawer list, popover body, code blocks): a thin bar, no arrows, transparent track, `--rl-line` thumb with full radius (`--rl-hint` on hover). Never the OS default: its grey track breaks the paper surface, worst in dark.

## Phone sheet

At ≤ 600 px, popovers become a bottom sheet: full width, top radius 18, a 40×4 grab handle, `--rl-shadow-sheet`, and an 18% ink scrim over the page. Margin markers become inline superscript markers (18 px) right after the highlighted range. The dock spans the full width minus 16 px gutters.
