# .design: redline's design system

The source of truth for how redline looks and reads, on every surface it draws: `web/overlay*.js`, the Streamlit component (`bindings/streamlit/…/frontend/`), the React example, and anything with the logo (README, release pages, favicons).

> redline never styles the artifact. These rules cover only what redline draws on top of the artifact (invariant 1 in `.agent/context.md`).

## The idea in one line

**Red pen for you, blue pencil for the agent, ink on paper for everything else.** Editors mark proofs in red. Blue pencil is the editor's note that doesn't print. Red belongs to the marks on the artifact. Chrome steps back.

## Files

| File | What | Read when |
|---|---|---|
| [principles.md](principles.md) | The 6 rules every design decision is checked against | Always, first |
| [brand.md](brand.md) | Logo, mark, favicon, clear space, misuse, voice and copy | Using the logo or writing UI text |
| [color.md](color.md) | Palette, semantic tokens, theme selection, contrast | Picking any color |
| [typography.md](typography.md) | Stacks, scale, where the serif goes | Setting any text |
| [components.md](components.md) | Markers, highlights, popover, compose, dock, drawer, chips, buttons, toast, sheet | Building or changing UI |
| [layout.md](layout.md) | Margin placement, popover placement, z-order, phone | Positioning anything |
| [accessibility.md](accessibility.md) | Contrast, focus, keyboard, motion, touch, screen readers | Before calling UI done |
| [tokens.css](tokens.css) | Every token as CSS custom properties, light and dark | Implementing |
| [tokens.json](tokens.json) | The same tokens, for tools and bindings | Implementing outside CSS |
| [check.py](check.py) | Contrast of every token pair the UI uses; tokens.css ≡ tokens.json | After changing a token |
| [logo/](logo/) | Outlined SVGs and `build.py`, their generator | Using the logo |

## Rules for changing the design

- **Tokens first.** Change a value in `tokens.css` *and* `tokens.json` in the same change, then run `python3 .design/check.py`. UI code uses tokens, never raw hex (except the `::highlight` sheet, which is generated from tokens; see color.md).
- **A new component or state** gets a section in `components.md` before code.
- **A change to a principle** needs a `docs/DECISIONS.md` entry.
- **Keep the canvas in sync** if you change the look: the reference mockups live on a private claude.ai design canvas (redline redesign) that the maintainer can share. The files in this folder win when they disagree with it.

## Status

Design v2, implemented in `web/` (the overlay), the Streamlit component, the React example and the README. `python3 .design/check.py` keeps every copy of the tokens in sync.
