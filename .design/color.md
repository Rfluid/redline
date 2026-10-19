# Color

Tokens are in [tokens.css](tokens.css) (CSS) and [tokens.json](tokens.json). Use the **semantic** names in code, never the palette hex.

## Palette

| Name | Light | Dark | Role |
|---|---|---|---|
| Red pen | `#D7261E` | `#FF5A4E` | Marks on the artifact, the logo stroke. Nothing else. |
| Blue pencil | `#2B4C7E` | `#8DB0E8` | The agent: replies, working, answered, focus ring |
| Ink | `#1B1A17` | `#F3EFE7` | Text and primary buttons (they swap in dark) |
| Graphite | `#5E5A53` | `#B9B3A8` | Secondary text, labels, icons |
| Rule | `#E4E0D8` | `#3A3832` | Borders and dividers |
| Paper | `#FBFAF7` | `#1A1916` | Overlay surfaces: popover, dock, drawer |
| Raised | `#FFFFFF` | `#23221E` | Inputs, cards inside surfaces |
| Note (amber) | `#B26A00` | `#E0A84A` | Reconnecting, detached, errors. Glyphs only in light (4.06:1); text uses `--rl-note-fg` |
| Done (green) | `#2F6B3F` | `#7FC08F` | Resolved, connected |

The palette is warm on purpose (paper, not screen grey), so redline reads as *ours* on top of cool-grey app UIs.

## Semantic tokens

| Token | Means |
|---|---|
| `--rl-fg`, `--rl-muted`, `--rl-hint` | text: main, secondary, hint/shortcut |
| `--rl-surface`, `--rl-raised`, `--rl-line`, `--rl-line-soft`, `--rl-code` | surfaces and borders |
| `--rl-primary`, `--rl-on-primary` | the one primary button per surface |
| `--rl-mark`, `--rl-mark-fg` | red-pen marks; red text on a red tint (chips) |
| `--rl-mark-tint-draft`, `--rl-mark-tint`, `--rl-mark-tint-focus` | highlight fills: draft, open, focused |
| `--rl-agent`, `--rl-agent-tint` | blue pencil; its highlight and chip fill |
| `--rl-note`, `--rl-note-fg`, `--rl-note-tint`, `--rl-note-line` | warnings and errors |
| `--rl-done`, `--rl-done-tint` | resolved, live |
| `--rl-focus` | focus ring |

## Choosing the theme

The overlay sets `data-theme="light" | "dark"` on its host from the **page**, not the OS:

1. Walk `document.body`, then `document.documentElement`. Take the first computed `background-color` with alpha > 0.5.
2. If none is found, the page is on the browser canvas. Use dark only if the page opts into dark (`color-scheme: dark`, or `light dark` together with `prefers-color-scheme: dark`); otherwise use light.
3. Relative luminance < 0.18 → `dark`, else `light`. (0.18 is where black and white text have equal contrast: darker than that, light UI text reads better.)
4. Run it again after the page settles (the existing MutationObserver debounce), and on `prefers-color-scheme` changes.

The `::highlight()` sheet lives in the document and can't read our host attribute. Rebuild that adopted sheet whenever the theme changes, reading the values from the host's computed custom properties (`web/theme.js`), so the tokens exist in one place.

## Highlights

The overlay registers one CSS highlight per state (`redline-draft`, `redline-open`, `redline-answered`, `redline-focus`), styled as a translucent fill plus an underline:

| State | Fill | Underline |
|---|---|---|
| Draft | `--rl-mark-tint-draft` | dashed, 1.5 px, `--rl-mark` |
| Open / working | `--rl-mark-tint` | solid, 2 px, `--rl-mark` |
| Answered | `--rl-agent-tint` | solid, 2 px, `--rl-agent` |
| Focused (marker hovered, popover open) | `--rl-mark-tint-focus` (agent tint if answered) | solid, 3 px |
| Resolved | none: drop the range | |

`text-decoration` in `::highlight()` is Chromium-first. The fill carries the meaning on its own, and the underline only reinforces it.

## Contrast (checked)

`python3 .design/check.py` checks every pair the UI uses: text ≥ 4.5:1, glyphs ≥ 3:1, translucent fills composited over `--rl-surface`. For example `--rl-muted` on `--rl-surface` is 6.57:1 in light and `--rl-hint` is 5.23:1. Red `#D7261E` on white ≈ 5.0:1, which is enough for the marker numerals. Open vs answered differ in **shape and lightness** (ring vs filled), not hue alone. See accessibility.md.
