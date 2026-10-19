# Principles

Check every design decision against these. When two conflict, the lower number wins.

1. **The artifact comes first.** redline is a guest on someone else's page. Never cover content the user is reading for longer than they asked. Never mutate the page's DOM. Draw only in our shadow root and in `::highlight()`.
2. **Red belongs to the artifact.** Red is for marks *on* the artifact: highlights, margin markers, region pins, and the stroke in the logo. Buttons, tabs and chrome are ink. Red never means "primary action", and it never means "error" (errors use the note color, plus words).
3. **The agent writes in blue pencil.** Anything the agent wrote or is doing (replies, the working note, answered markers) uses the agent blue. You can tell who said what at a glance.
4. **Match the page, not the OS.** Pick the light (paper) or dark (ink) theme from the page's own background. Highlights are translucent, so they read on any background.
5. **Stay in the margin.** Threads show up as numbered markers beside the text and open as popovers next to it. The full list is one click away in the dock, never in the way.
6. **Quiet until needed.** Hide zero counts. Collapse reply boxes to one line. Secondary actions are outline or ghost buttons. One primary button per surface.

## What this rules out

- A red "Submit" button. Use ink.
- A full-height panel that opens by itself and covers the page. Open the popover for the thread that changed; the drawer opens only when asked.
- `prefers-color-scheme` as the only theme signal.
- Emoji as icons. Use the stroke SVG set in components.md.
- `alert()` for errors. Use the toast.
