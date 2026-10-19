#!/usr/bin/env python3
"""Regenerate redline's logo SVGs with the letters outlined, so the files need no font.

    # Newsreader is OFL: https://github.com/google/fonts/tree/main/ofl/newsreader
    curl -sSfLo /tmp/Newsreader.ttf "https://github.com/google/fonts/raw/main/ofl/newsreader/Newsreader%5Bopsz,wght%5D.ttf"
    python3 -m pip install fonttools   # if missing
    python3 .design/logo/build.py /tmp/Newsreader.ttf

It writes wordmark.svg, wordmark-dark.svg, mark.svg, mark-light.svg and favicon.svg next to this file.
Change the strokes, tiles or colors here, never by hand-editing the SVGs. Keep colors in sync with
../tokens.css.
"""
import os
import sys

from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont

HERE = os.path.dirname(os.path.abspath(__file__))
INK, PAPER, RED, RED_DARK = "#1B1A17", "#F3EFE7", "#D7261E", "#FF5A4E"

# The hand-drawn strokes. Wordmark: across the x-height, rising right, overshooting both ends.
WORD_STROKE = "M3 56 C 62 51, 150 60, 214 54 S 290 50, 313 46"
MARK_STROKE = "M7 37 C 22 33, 40 38, 57 31"


def outline(font, text, size, track):
    """SVG path data for `text` with its baseline at y=0 (y down)."""
    gs, cmap, hmtx = font.getGlyphSet(), font.getBestCmap(), font["hmtx"]
    s, x, pen = size / font["head"].unitsPerEm, 0.0, SVGPathPen(None)
    for ch in text:
        g = cmap[ord(ch)]
        gs[g].draw(TransformPen(pen, (s, 0, 0, -s, x, 0)))
        x += hmtx[g][0] * s + track
    return pen.getCommands()


def wordmark(d, ink, red, title):
    return f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 316 100" width="316" height="100" role="img" aria-label="redline">
  <title>{title}</title>
  <path transform="translate(10 78)" fill="{ink}" d="{d}"/>
  <path d="{WORD_STROKE}" fill="none" stroke="{red}" stroke-width="6" stroke-linecap="round"/>
</svg>
"""


def mark(d, tile, ink, red, title, stroke_width=5):
    letter = f'\n  <path transform="translate(18.5 48)" fill="{ink}" d="{d}"/>' if d else ""
    return f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" width="64" height="64" role="img" aria-label="redline">
  <title>{title}</title>
  <rect width="64" height="64" rx="14" fill="{tile}"/>{letter}
  <path d="{MARK_STROKE}" fill="none" stroke="{red}" stroke-width="{stroke_width}" stroke-linecap="round"/>
</svg>
"""


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    font = instantiateVariableFont(TTFont(sys.argv[1]), {"wght": 600, "opsz": 72})
    word, r = outline(font, "redline", 92, -2), outline(font, "r", 60, 0)
    files = {
        "wordmark.svg": wordmark(word, INK, RED, "redline wordmark, for light backgrounds"),
        "wordmark-dark.svg": wordmark(word, PAPER, RED_DARK, "redline wordmark, for dark backgrounds"),
        "mark.svg": mark(r, INK, PAPER, RED_DARK, "redline mark (ink tile)"),
        "mark-light.svg": mark(r, PAPER, INK, RED, "redline mark (paper tile), for dark UIs"),
        "favicon.svg": mark(None, INK, PAPER, RED_DARK, "redline favicon (16-24 px)", stroke_width=10),
    }
    for name, svg in files.items():
        with open(os.path.join(HERE, name), "w") as f:
            f.write(svg)
        print(name)


if __name__ == "__main__":
    main()
