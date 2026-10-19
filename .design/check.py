#!/usr/bin/env python3
"""Check redline's design tokens: WCAG contrast of the pairs the UI uses, and that every copy of the
tokens (tokens.css, the overlay, the Streamlit component, the React example) matches tokens.json.

    python3 .design/check.py        # prints every pair; exits 1 on a failure or a mismatch

Text pairs need 4.5:1. Glyph-only pairs (marked "glyph") need 3:1. Translucent backgrounds are
composited over `surface` first. Add a pair here when the UI starts using a new combination.
"""
import json, os, re, sys

D = os.path.dirname(os.path.abspath(__file__))
# (fg, bg, kind): kind is "text" (4.5) or "glyph" (3.0)
PAIRS = [("fg", "surface", "text"), ("muted", "surface", "text"), ("hint", "surface", "text"),
         ("muted", "raised", "text"), ("on-primary", "primary", "text"), ("mark", "raised", "text"),
         ("agent", "surface", "text"), ("mark-fg", "mark-tint-draft", "text"), ("agent", "agent-tint", "text"),
         ("done", "done-tint", "text"), ("note-fg", "note-tint", "text"), ("note-fg", "surface", "text"),
         ("done", "surface", "text"), ("note", "surface", "glyph"), ("mark", "surface", "glyph")]
# Files that carry a copy of the tokens: (path from .design/, [(theme, block start, block end)]).
COPIES = [
    ("tokens.css", [("light", ":root, :host {", ":root[data-theme"), ("dark", ':root[data-theme="dark"]', "@media")]),
    ("../web/overlay-style.js", [("light", ":host { all: initial;", ':host([data-theme="dark"])'), ("dark", ':host([data-theme="dark"]) {', "@media")]),
    ("../bindings/streamlit/redline_streamlit/frontend/index.html", [("light", ":root {", ':root[data-theme="dark"]'), ("dark", ':root[data-theme="dark"]', "* {")]),
    ("../bindings/react/example.html", [("light", ":root {", "* {")]),
]


def rgb(s):
    s = s.strip()
    if s.startswith("#"):
        return tuple(int(s[i:i + 2], 16) for i in (1, 3, 5)), 1.0
    m = re.match(r"rgba\((\d+),\s*(\d+),\s*(\d+),\s*([.\d]+)\)", s)
    return tuple(map(int, m.groups()[:3])), float(m.group(4))


def over(fg, bg):
    """Composite a translucent fg over an opaque bg; returns #rrggbb."""
    (c, a), (b, _) = rgb(fg), rgb(bg)
    return "#%02x%02x%02x" % tuple(round(c[i] * a + b[i] * (1 - a)) for i in range(3))


def luminance(h):
    c, _ = rgb(h)
    f = lambda v: v / 12.92 if v <= .04045 else ((v + .055) / 1.055) ** 2.4
    r, g, b = (f(v / 255) for v in c)
    return .2126 * r + .7152 * g + .0722 * b


def ratio(a, b):
    x, y = sorted([luminance(a), luminance(b)])
    return (y + .05) / (x + .05)


def main():
    t = json.load(open(os.path.join(D, "tokens.json")))["color"]
    bad = 0
    for th in ("light", "dark"):
        c = t[th]
        for f, b, kind in PAIRS:
            bg = c[b] if c[b].startswith("#") else over(c[b], c["surface"])
            r, need = ratio(c[f], bg), 4.5 if kind == "text" else 3.0
            ok = r >= need; bad += not ok
            print(f"{'ok  ' if ok else 'FAIL'} {th:5} {f} on {b} ({kind}): {r:.2f}")
    for path, blocks in COPIES:
        text = open(os.path.join(D, path)).read()
        for th, start, end in blocks:
            i = text.index(start)
            blk = text[i:text.index(end, i + len(start))]
            for k, v in re.findall(r"--rl-([a-z-]+):\s*([^;]+);", blk):
                if k in t[th] and t[th][k].replace(" ", "").lower() != v.replace(" ", "").lower():
                    bad += 1; print(f"MISMATCH {path} {th} --rl-{k}: {v.strip()} (tokens.json: {t[th][k]})")
    sys.exit(1 if bad else 0)

if __name__ == "__main__":
    main()
