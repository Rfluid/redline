/* redline anchors: where a comment points, and finding that place again. Protocol helpers only;
 * no UI, no DOM writes. Served in front of client.js (one /client.js), so it must not depend on it.
 *
 * Text anchors follow the W3C TextQuoteSelector idea: exact quote + some context, scoped by a CSS
 * selector. Region anchors are a point (on charts, canvases, images), relative to an element.
 */
(function () {
  "use strict";

  function cssPath(el) {
    if (!(el instanceof Element)) return "body";
    const parts = [];
    while (el && el !== document.body && el.nodeType === 1) {
      if (el.id) { parts.unshift(`#${CSS.escape(el.id)}`); break; }
      const tag = el.tagName.toLowerCase();
      const sibs = Array.from(el.parentNode ? el.parentNode.children : []).filter((s) => s.tagName === el.tagName);
      parts.unshift(sibs.length > 1 ? `${tag}:nth-of-type(${sibs.indexOf(el) + 1})` : tag);
      el = el.parentElement;
    }
    return parts.length ? parts.join(" > ") : "body";
  }

  // Text the user can see: skip <script>/<style>/... so inline code can't match a quote.
  const HIDDEN = new Set(["SCRIPT", "STYLE", "NOSCRIPT", "TEMPLATE"]);
  function textNodes(root) {
    const w = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
      acceptNode: (n) => (n.parentElement && HIDDEN.has(n.parentElement.tagName) ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_ACCEPT),
    });
    const out = [];
    while (w.nextNode()) out.push(w.currentNode);
    return out;
  }
  function flatten(root) {
    const nodes = textNodes(root);
    return { nodes, full: nodes.map((n) => n.data).join("") };
  }

  function fromSelection(sel = window.getSelection(), ctx = 32) {
    if (!sel || sel.isCollapsed || !sel.rangeCount) return null;
    const range = sel.getRangeAt(0);
    const quote = range.toString();
    if (!quote.trim()) return null;
    let root = range.commonAncestorContainer;
    if (root.nodeType !== 1) root = root.parentElement;
    const { nodes, full } = flatten(root);
    let start = -1, pos = 0;
    for (const n of nodes) {
      if (n === range.startContainer) { start = pos + range.startOffset; break; }
      pos += n.data.length;
    }
    if (start === -1) { // selection starts at an element boundary
      const pre = document.createRange();
      pre.selectNodeContents(root);
      pre.setEnd(range.startContainer, range.startOffset);
      start = pre.toString().length;
    }
    return {
      kind: "text",
      page: location.pathname,
      selector: cssPath(root),
      quote,
      prefix: full.slice(Math.max(0, start - ctx), start),
      suffix: full.slice(start + quote.length, start + quote.length + ctx),
    };
  }

  function fromElement(el, extra = {}) {
    return { kind: "element", page: location.pathname, selector: cssPath(el), ...extra };
  }

  /**
   * Region anchor for viewport point (x, y): the element there plus the point's position inside it
   * (ex, ey as fractions), with page coordinates as a fallback. Hide your own UI first.
   */
  function fromPoint(x, y) {
    const el = document.elementFromPoint(x, y) || document.body;
    const r = el.getBoundingClientRect();
    const frac = (v, len) => (len > 0 ? Math.min(1, Math.max(0, v / len)) : 0);
    return {
      kind: "region", page: location.pathname, selector: cssPath(el),
      ex: frac(x - r.left, r.width), ey: frac(y - r.top, r.height),
      x: frac(x, innerWidth), y: frac(y, innerHeight), px: Math.round(x + scrollX), py: Math.round(y + scrollY),
    };
  }

  // Screenshot crop around a region point, in CSS pixels.
  const CROP_W = 480, CROP_H = 320;

  /**
   * Best-effort crop around viewport point (x, y) of the canvas, SVG or image under it, as
   * `{mime: "image/png", data: <base64>}`. Null for plain HTML (browsers can't screenshot the DOM
   * without a permission prompt) or tainted (cross-origin) pixels.
   */
  async function snapshot(el, x, y) {
    const target = el.closest("svg") || el;
    let src = null;
    if (target instanceof HTMLCanvasElement) src = target;
    else if (target instanceof HTMLImageElement && target.complete) src = target;
    else if (target instanceof SVGSVGElement) src = await svgImage(target).catch(() => null);
    if (!src) return null;
    const r = target.getBoundingClientRect();
    const sw = (src.width || src.naturalWidth) / r.width, sh = (src.height || src.naturalHeight) / r.height;
    const w = Math.min(CROP_W, r.width), h = Math.min(CROP_H, r.height);
    const left = Math.min(Math.max(0, x - r.left - w / 2), r.width - w), top = Math.min(Math.max(0, y - r.top - h / 2), r.height - h);
    const out = document.createElement("canvas");
    out.width = Math.round(w); out.height = Math.round(h);
    try {
      out.getContext("2d").drawImage(src, left * sw, top * sh, w * sw, h * sh, 0, 0, out.width, out.height);
      return { mime: "image/png", data: out.toDataURL("image/png").split(",")[1] };
    } catch { return null; } // tainted canvas
  }

  function svgImage(svg) {
    const r = svg.getBoundingClientRect();
    const copy = svg.cloneNode(true);
    copy.setAttribute("xmlns", "http://www.w3.org/2000/svg");
    copy.setAttribute("width", r.width); copy.setAttribute("height", r.height);
    const url = URL.createObjectURL(new Blob([new XMLSerializer().serializeToString(copy)], { type: "image/svg+xml" }));
    return new Promise((resolve, reject) => {
      const img = new Image(r.width, r.height);
      img.onload = () => { URL.revokeObjectURL(url); resolve(img); };
      img.onerror = (e) => { URL.revokeObjectURL(url); reject(e); };
      img.src = url;
    });
  }

  function toRange(nodes, start, end) {
    const range = document.createRange();
    let pos = 0, started = false;
    for (const n of nodes) {
      const stop = pos + n.data.length;
      if (!started && start < stop) { range.setStart(n, start - pos); started = true; }
      if (started && end <= stop) { range.setEnd(n, end - pos); return range; }
      pos = stop;
    }
    return null;
  }

  // Exact quote; among several occurrences, prefer the one whose prefix matches.
  function findQuote(full, a) {
    const prefix = a.prefix || "";
    let at = -1;
    for (let i = full.indexOf(a.quote); i !== -1; i = full.indexOf(a.quote, i + 1)) {
      if (at === -1) at = i;
      if (full.slice(Math.max(0, i - prefix.length), i) === prefix) return i;
    }
    return at;
  }

  // The quote was edited: take whatever now sits between its old prefix and suffix. Tries the full
  // context, then its nearest characters, so an edit next to the quote doesn't defeat it.
  const CONTEXT_MIN = 4, CONTEXT_NEAR = 12, MAX_DRIFT = 400;
  function findBetween(full, a) {
    for (const n of [Infinity, CONTEXT_NEAR]) {
      const pre = (a.prefix || "").slice(-n), suf = (a.suffix || "").slice(0, n);
      if (pre.length < CONTEXT_MIN || suf.length < CONTEXT_MIN) continue;
      for (let i = full.indexOf(pre); i !== -1; i = full.indexOf(pre, i + 1)) {
        const s = i + pre.length, e = full.indexOf(suf, s);
        if (e > s && e - s <= Math.max(a.quote.length * 4, MAX_DRIFT)) return [s, e];
      }
    }
    return null;
  }

  /**
   * Where an anchor points now. `match`: "exact"; "moved" (quote found outside its selector, or a
   * region whose element is gone: its old page point); "fuzzy" (quote edited, found by its context);
   * "none" (detached: show the original quote). Region anchors also get `point` (page px).
   */
  function locate(a) {
    const none = { target: null, match: "none" };
    if (!a) return none;
    let scoped = null;
    try { scoped = a.selector ? document.querySelector(a.selector) : null; } catch {}
    if (a.kind === "element") return scoped ? { target: scoped, match: "exact" } : none;
    if (a.kind === "region") {
      if (scoped) {
        const r = scoped.getBoundingClientRect();
        return { target: scoped, match: "exact", point: { x: r.left + a.ex * r.width + scrollX, y: r.top + a.ey * r.height + scrollY } };
      }
      return { target: null, match: "moved", point: { x: a.px, y: a.py } };
    }
    if (a.kind !== "text" || !a.quote) return none;
    const roots = [[scoped, "exact"], [document.body, scoped ? "moved" : "exact"]].filter(([r]) => r);
    for (const [root, match] of roots) {
      const { nodes, full } = flatten(root), at = findQuote(full, a);
      if (at !== -1) return { target: toRange(nodes, at, at + a.quote.length), match };
    }
    for (const [root] of roots) {
      const { nodes, full } = flatten(root), between = findBetween(full, a);
      if (between) return { target: toRange(nodes, ...between), match: "fuzzy" };
    }
    return none;
  }

  /** Best-effort: a Range for a text anchor, an Element for an element anchor, or null. */
  function resolve(a) { return locate(a).target; }

  window.Redline = Object.assign(window.Redline || {}, {
    anchors: { fromSelection, fromElement, fromPoint, snapshot, resolve, locate, cssPath },
  });
})();
