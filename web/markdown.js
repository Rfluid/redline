/* redline markdown: a small, safe renderer for agent replies in the overlay.
 * Escape first, then add a fixed set of tags: paragraphs, line breaks, **bold**, *italic*, `code`,
 * fenced code blocks, - / 1. lists, # headings (as bold lines) and [links](https://…).
 * No HTML from the input survives, and links only take http(s) and mailto.
 * Served in front of overlay.js (one /overlay.js).
 */
(function () {
  "use strict";
  const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]);
  const SAFE_URL = /^(https?:\/\/|mailto:)/i;

  // `text` is already escaped. Code spans are cut out first so nothing inside them is formatted.
  function inline(text) {
    return text.split(/(`[^`\n]+`)/).map((part, i) => {
      if (i % 2) return `<code>${part.slice(1, -1)}</code>`;
      return part
        .replace(/\[([^\]\n]+)\]\(([^)\s]+)\)/g, (m, label, url) =>
          SAFE_URL.test(url) ? `<a href="${url}" target="_blank" rel="noopener noreferrer">${label}</a>` : m)
        .replace(/\*\*([^*\n]+)\*\*/g, "<strong>$1</strong>")
        .replace(/(^|[^\w*])\*([^*\n]+)\*(?!\w)/g, "$1<em>$2</em>")
        .replace(/(^|[^\w])_([^_\n]+)_(?!\w)/g, "$1<em>$2</em>");
    }).join("");
  }

  function block(chunk) {
    const lines = chunk.split("\n");
    if (lines.every((l) => /^\s*[-*] /.test(l))) return `<ul>${lines.map((l) => `<li>${inline(l.replace(/^\s*[-*] /, ""))}</li>`).join("")}</ul>`;
    if (lines.every((l) => /^\s*\d+[.)] /.test(l))) return `<ol>${lines.map((l) => `<li>${inline(l.replace(/^\s*\d+[.)] /, ""))}</li>`).join("")}</ol>`;
    const h = /^#{1,6} (.*)$/.exec(chunk);
    if (h && lines.length === 1) return `<p><strong>${inline(h[1])}</strong></p>`;
    return `<p>${lines.map(inline).join("<br>")}</p>`;
  }

  /** Markdown source → safe HTML string. */
  function render(src) {
    const parts = esc(String(src).replace(/\r\n?/g, "\n")).split(/^```[^\n]*\n([\s\S]*?)^```[ \t]*$/m);
    return parts.map((part, i) => (i % 2
      ? `<pre><code>${part.replace(/\n$/, "")}</code></pre>`
      : part.split(/\n[ \t]*\n/).map((c) => c.trim()).filter(Boolean).map(block).join(""))).join("");
  }

  window.Redline = Object.assign(window.Redline || {}, { markdown: { render } });
})();
