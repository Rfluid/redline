# Accessibility and motion

## Contrast

- Text ≥ 4.5:1 against its own background; icons and the marker ring ≥ 3:1. The token pairs in color.md are checked. Adding a token pair? Add it to `PAIRS` in `check.py` and run `python3 .design/check.py`.
- Never tell states apart by hue alone. Markers differ by shape (dashed, ring, filled, dot), and chips carry a glyph *and* a word.
- `forced-colors: active`: drop fills and shadows, give the markers and the highlight `Highlight`/`CanvasText` system colors, and keep borders at 1 px `CanvasText`.

## Focus

- Every control is a real `<button>`, `<input>` or `<textarea>` with a label (a visible label, `aria-label`, or a visually-hidden `<span>` inside the `<label>`).
- `:focus-visible` = `outline: 2px solid var(--rl-focus); outline-offset: 2px`. Never remove an outline without replacing it.
- Opening a popover moves focus into it (the textarea for compose, the reply input for a thread). Closing it returns focus to the marker or the page selection.
- Popovers trap Tab while they are open (`role="dialog"`, `aria-modal="false"`, labelled "Thread 2" or "New comment").

## Keyboard

| Key | Does |
|---|---|
| ⌥C / Alt+C | Comment on the selection (skipped while typing in the page's own fields) |
| ⌘↵ / Ctrl+Enter | Send (compose or reply) |
| ⌥↵ / Alt+Enter | Save as draft |
| Esc | Close the popover or drawer (in the drawer's Select mode, leave the mode first) |
| Shift+click (drawer, Select mode) | Select the range from the last row clicked |
| ⌥⇧R / Alt+Shift+R | Turn redline off (clean page, faint mark only) or back on; announced in the live region |
| Tab / Shift+Tab | Move through the markers in page order (they follow the document order of their anchors) |

## Screen readers

- Markers: `aria-label="Thread 2, answered, 1 new reply"`.
- New agent replies are announced through one polite live region: "Agent replied on thread 2".
- The working note updates the same live region, throttled to once per 10 s.
- The highlight is visual only. The marker is what makes the thread reachable.

## Touch

- Targets ≥ 44 × 44 under `(pointer: coarse)` or ≤ 600 px, even if the visual is smaller (pad the hit area).
- The selection chip goes below the selection on touch (layout.md).

## Motion

| What | Motion | Duration |
|---|---|---|
| Popover, chip | fade + 4 px rise | 120 ms ease-out |
| Drawer, sheet | slide | 180 ms cubic-bezier(.2,.8,.2,1) |
| Marker appear | scale .8 → 1 | 120 ms |
| Working dots | opacity pulse, staggered 200 ms | 1.2 s loop |
| Waiting comet (marker) | rotation | 1.1 s loop |
| New agent reply | the marker halo pulses twice | 2 × 600 ms |

Under `prefers-reduced-motion: reduce`: no transforms, crossfades only (≤ 80 ms), the working dots hold steady at full opacity, the waiting comet stands still, and the halo shows without pulsing.
