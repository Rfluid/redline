/* redline theme: pick the overlay's light or dark theme from the page's own background, not the OS
 * (.design/color.md › Choosing the theme), and build the ::highlight() sheet for the per-state
 * highlights from the overlay's tokens. Reads computed styles only; never touches the page's DOM.
 * Served in front of overlay.js.
 */
(function () {
  "use strict";
  const parse = (c) => {
    const m = /rgba?\(([^)]+)\)/.exec(c || "");
    if (!m) return null;
    const [r, g, b, a = 1] = m[1].split(/[\s,/]+/).filter(Boolean).map(Number);
    return { r, g, b, a };
  };
  const luminance = ({ r, g, b }) => {
    const f = (v) => ((v /= 255) <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4);
    return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b);
  };
  // 0.18 is where black and white text have equal contrast: below it, light text reads better.
  const DARK_BELOW = 0.18;

  /** "light" | "dark" for the page as it is now. */
  function detect() {
    for (const el of [document.body, document.documentElement]) {
      const c = el && parse(getComputedStyle(el).backgroundColor);
      if (c && c.a > 0.5) return luminance(c) < DARK_BELOW ? "dark" : "light";
    }
    // Transparent all the way down: the browser canvas. Dark only if the page opts into it.
    const meta = document.querySelector('meta[name="color-scheme"]');
    const scheme = `${getComputedStyle(document.documentElement).colorScheme} ${meta ? meta.content : ""}`;
    if (!/\bdark\b/.test(scheme)) return "light";
    return !/\blight\b/.test(scheme) || matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  }

  /** CSS for the document's ::highlight() rules, from the tokens computed on our shadow host. */
  function highlightCSS(host) {
    const cs = getComputedStyle(host), v = (n) => cs.getPropertyValue(`--rl-${n}`).trim();
    const rule = (name, fill, line, style, width) =>
      `::highlight(${name}){background-color:${v(fill)};text-decoration:underline ${style} ${v(line)} ${width};text-underline-offset:3px}`;
    return [
      rule("redline-draft", "mark-tint-draft", "mark", "dashed", "1.5px"),
      rule("redline-open", "mark-tint", "mark", "solid", "2px"),
      rule("redline-answered", "agent-tint", "agent", "solid", "2px"),
      rule("redline-focus", "mark-tint-focus", "mark", "solid", "3px"),
      rule("redline-focus-agent", "agent-tint-focus", "agent", "solid", "3px"),
      "@media (forced-colors: active){::highlight(redline-open),::highlight(redline-answered),::highlight(redline-draft),::highlight(redline-focus),::highlight(redline-focus-agent){background-color:Highlight;color:HighlightText}}",
    ].join("\n");
  }

  window.Redline = Object.assign(window.Redline || {}, { ui: Object.assign((window.Redline && window.Redline.ui) || {}, { theme: { detect, highlightCSS } }) });
})();
