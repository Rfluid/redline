# Layout

## Margin markers

Markers go in the right margin of the **text block** that holds the highlight, not at the viewport edge.

1. `block` = the nearest ancestor of the range's end container whose computed `display` is block, flex or grid and whose width ≥ 240 px.
2. `x` = `block.getBoundingClientRect().right + 16`. If that leaves less than 40 px to the viewport's right edge, use `innerWidth - 40`.
3. `y` = the top of the range's **first** client rect, minus 3 (so the marker centers on the first line).
4. **Collisions:** sort the markers by `y`, then push each one down to at least `previous.y + 30`. If a marker is pushed more than 48 px off its line, draw a 1 px `--rl-line` leader from the marker to the line's right end.
5. Region pins stay at their point. They are already in the content.
6. Recompute on the existing debounced MutationObserver, on resize and on scroll (rAF-throttled). Markers live in our shadow root, positioned `absolute` in page coordinates so they scroll with the page for free.

## Popovers

- Anchor to the marker: the left edge at `marker.right + 14`, the top at `marker.top - 12`.
- If it overflows on the right, flip to the left of the **highlight** (`range.left - 14 - width`). If it still doesn't fit, place it under the first line, clamped to an 8 px viewport gutter.
- Clamp vertically into the viewport with 8 px gutters. Max height `min(560px, 100vh - 16px)`; the message list scrolls, while the header and reply row stay fixed.
- Only one popover at a time. Opening another closes the first and keeps its typed text (today's typed-text preservation applies).
- Close on Esc, the close button, or a click outside both the popover and its marker.

## Compose popover

Straight under the selection (or the clicked spot), left-aligned to it and clamped to the viewport; above it if it doesn't fit below. That keeps the selected words and the margin markers visible while typing.

## Selection chip

8 px above the end of the selection's last rect, right-aligned to it, clamped to the viewport. On touch, put it 12 px **below** the selection, clear of the OS selection menu.

## Dock and drawer

- The dock is fixed `right: 24px; bottom: 24px`. The toast sits above it (`bottom: 84px`).
- The drawer is fixed on the right, 360 wide (`--rl-drawer-w`). When it is open, the dock moves left to sit beside it.

## Z-order (inside our shadow host)

| Layer | z-index |
|---|---|
| Markers, pins, leaders | 2147483640 |
| Dock | 2147483643 |
| Drawer | 2147483644 |
| Popover, selection chip | 2147483646 |
| Toast | 2147483647 |

## Spacing and shape

A 4 px base grid. Tokens: `--rl-space-1` = 4 up to `--rl-space-6` = 24. Radii: 6 (tags, thumbs), 8 (buttons, chip), 9 (inputs), 12 (popover, drawer), 18 (sheet top), full (markers, chips, dock).

## Phone (≤ 600 px)

- Markers become 16 px superscripts at the end of the range's last line (`right - 4`, `top - 11`), so no margin is needed.
- Popover, compose and drawer are all bottom sheets, max height 80vh.
- The dock is full width, 16 px gutters, 48 tall.
