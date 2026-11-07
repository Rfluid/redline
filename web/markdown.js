/* redline markdown: a small, safe renderer for agent replies in the overlay.
 * Blocks: paragraphs (single newlines become line breaks), # and setext headings (as bold lines),
 * fenced code (``` or ~~~), nested and loose - / * / + / 1. lists, task items ([ ] / [x]),
 * > quotes, --- rules and | pipe | tables. Inline: **bold**, *italic* / _italic_, ***both***,
 * ~~strike~~, `code` (any backtick run), \-escapes, [links](https://…), <https://…> and bare URLs.
 * Every piece of literal text is HTML-escaped as it is emitted, and the only tags are the fixed set
 * above; links only take http(s) and mailto. Served in front of overlay.js (one /overlay.js).
 */
(function () {
  "use strict";
  const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]);
  const SAFE_URL = /^(https?:\/\/|mailto:)/i;
  const link = (url, label) => `<a href="${esc(url)}" target="_blank" rel="noopener noreferrer">${label}</a>`;

  // ---- inline -------------------------------------------------------------------------------
  // Pass 1 cuts out escapes, code spans and links as placeholders (\uE000n\uE001); pass 2 pairs
  // emphasis delimiters recursively over the rest, so tags always nest properly.
  const ESC = /\\([!-/:-@[-`{-~])/y, TICKS = /`+/y, AUTO = /<((?:https?:\/\/|mailto:)[^\s<>]+)>/y;
  const URL = /https?:\/\/[^\s<>]+/y;
  // Bounded, so a long run of unmatched "[" stays linear.
  const LINK = /!?\[((?:\\.|`[^`\n]{0,200}`|[^\]\\`]){0,500})\]\(\s*(<[^<>\n]{0,2048}>|(?:[^\s()\\]|\\.|\((?:[^\s()\\]|\\.){0,512}\)){0,2048})(?:\s+(?:"[^"\n]{0,512}"|'[^'\n]{0,512}'))?\s*\)/y;
  const PH = /\uE000(\d+)\uE001/g, WORD = /[\p{L}\p{N}]/u, SPACE = /\s/;

  function inline(src, inLink = false) {
    const held = [], hold = (html) => `\uE000${held.push(html) - 1}\uE001`, noClose = new Set(); // backtick runs with no match
    let s = "";
    for (let i = 0; i < src.length;) {
      const c = src[i], at = (rx) => ((rx.lastIndex = i), rx.exec(src));
      let m, html;
      if (c === "\\" && (m = at(ESC))) html = esc(m[1]);
      else if (c === "`") {
        const run = at(TICKS)[0].length, end = noClose.has(run) ? -1 : codeEnd(src, i + run, run);
        if (end < 0) noClose.add(run);
        m = [end < 0 ? c.repeat(run) : src.slice(i, end + run)];
        html = end < 0 ? esc(m[0]) : `<code>${esc(codeText(src.slice(i + run, end)))}</code>`;
      }
      else if (!inLink && c === "<" && (m = at(AUTO))) html = link(m[1], esc(m[1]));
      else if (!inLink && (c === "[" || (c === "!" && src[i + 1] === "[")) && (m = at(LINK))) {
        const url = m[2].replace(/^<|>$/g, "").replace(/\\(.)/g, "$1");
        html = SAFE_URL.test(url) ? link(url, inline(m[1], true)) : esc(m[0]);
      } else if (!inLink && c === "h" && !WORD.test(src[i - 1] || "") && (m = at(URL))) {
        m = [trimUrl(m[0])];
        html = link(m[0], esc(m[0]));
      }
      if (html === undefined) { s += c; i++; } else { s += hold(html); i += m[0].length; }
    }
    return emph(s).replace(PH, (_, n) => held[n]);
  }

  // Where a code span opened by `run` backticks closes: the next run of exactly that many, or -1.
  function codeEnd(src, from, run) {
    for (let p = src.indexOf("`", from), q; p >= 0; p = src.indexOf("`", q)) {
      for (q = p; src[q] === "`";) q++;
      if (q - p === run) return p;
    }
    return -1;
  }

  // A code span's newlines are spaces; one space of padding on both sides is dropped.
  function codeText(t) {
    t = t.replace(/\n/g, " ");
    return /^ [\s\S]* $/.test(t) && t.trim() ? t.slice(1, -1) : t;
  }

  // Bare URLs don't take trailing punctuation, or a ")" that closes the surrounding sentence.
  function trimUrl(u) {
    let n = u.length, open = u.split("(").length, close = u.split(")").length;
    for (;; n--) {
      if (u[n - 1] === ")" && close > open) close--;
      else if (!/[.,:;!?'"*_~]/.test(u[n - 1])) return u.slice(0, n);
    }
  }

  const TAGS = { "~2": ["<del>", "</del>"], 1: ["<em>", "</em>"], 2: ["<strong>", "</strong>"], 3: ["<strong><em>", "</em></strong>"] };
  function emph(s) {
    const failed = {}; // c+k → the earliest start a closer search failed from: any later one fails too
    let out = "";
    for (let i = 0; i < s.length;) {
      const c = s[i];
      if (c !== "*" && c !== "_" && c !== "~") { out += c === "\n" ? "<br>" : esc(c); i++; continue; }
      let run = 1;
      while (s[i + run] === c) run++;
      const k = c === "~" ? (run === 2 ? 2 : 0) : Math.min(run, 3);
      const opens = k && s[i + run] && !SPACE.test(s[i + run]) && !(c === "_" && WORD.test(s[i - 1] || ""));
      const j = opens && !(i + run >= failed[c + k]) ? closer(s, i + run, k, c) : -1;
      if (opens && j < 0) failed[c + k] = Math.min(failed[c + k] ?? Infinity, i + run);
      if (j < 0) { out += esc(s.slice(i, i + run)); i += run; continue; }
      const [a, b] = TAGS[c === "~" ? "~2" : k];
      out += esc(c.repeat(run - k)) + a + emph(s.slice(i + run, j)) + b;
      i = j + k;
    }
    return out;
  }

  // The first run of exactly k delimiters after `from` that can close (no space before it; an
  // underscore not followed by a letter). Runs of other lengths are skipped whole.
  function closer(s, from, k, c) {
    for (let p = from + 1; p < s.length; p++) {
      if (s[p] !== c) continue;
      let run = 1;
      while (s[p + run] === c) run++;
      if (run === k && !SPACE.test(s[p - 1]) && !(c === "_" && WORD.test(s[p + run] || ""))) return p;
      p += run - 1;
    }
    return -1;
  }

  // ---- blocks -------------------------------------------------------------------------------
  const FENCE = /^( {0,3})(`{3,}|~{3,})/, HR = /^ {0,3}([-*_])(?:[ \t]*\1){2,}[ \t]*$/;
  const HEAD = /^ {0,3}#{1,6}(?:[ \t]+(.*?))?(?:[ \t]+#+)?[ \t]*$/, SETEXT = /^ {0,3}(?:=+|-+)[ \t]*$/;
  const QUOTE = /^ {0,3}> ?/, ITEM = /^( {0,3})([-*+]|(\d{1,9})[.)])(?:[ \t]+|$)/;
  const TDELIM = /^ {0,3}\|?[ \t]*:?-+:?[ \t]*(?:\|[ \t]*:?-+:?[ \t]*)*\|?[ \t]*$/;
  const MAX_DEPTH = 12, indent = (l) => /^ */.exec(l)[0].length;

  function cells(l) {
    let s = l.trim();
    if (s.startsWith("|")) s = s.slice(1);
    if (s.endsWith("|") && !s.endsWith("\\|")) s = s.slice(0, -1);
    return s.split(/(?<!\\)\|/).map((c) => c.trim().replace(/\\\|/g, "|"));
  }
  const isTable = (ls, i) => ls[i].includes("|") && i + 1 < ls.length && ls[i + 1].includes("|") && TDELIM.test(ls[i + 1])
    && cells(ls[i]).length === cells(ls[i + 1]).length;

  // Does line i start a block that ends a paragraph? (An ordered list only does if it starts at 1.)
  function starts(ls, i) {
    const l = ls[i], m = ITEM.exec(l);
    return FENCE.test(l) || HR.test(l) || HEAD.test(l) || QUOTE.test(l) || isTable(ls, i)
      || (!!m && !!l.slice(m[0].length).trim() && (!m[3] || +m[3] === 1));
  }

  function blocks(ls, tight = false, depth = 0) {
    if (depth > MAX_DEPTH) return `<p>${inline(ls.join("\n").trim())}</p>`;
    let out = "";
    for (let i = 0, m; i < ls.length;) {
      const l = ls[i];
      if (!l.trim()) { i++; continue; }
      if ((m = FENCE.exec(l))) {
        const end = new RegExp(`^ {0,3}${m[2][0]}{${m[2].length},}[ \\t]*$`), cut = new RegExp(`^ {0,${m[1].length}}`), code = [];
        for (i++; i < ls.length && !end.test(ls[i]); i++) code.push(ls[i].replace(cut, ""));
        out += `<pre><code>${esc(code.join("\n"))}</code></pre>`;
        i++;
      } else if (HR.test(l)) { out += "<hr>"; i++; }
      else if ((m = HEAD.exec(l))) { out += m[1] ? `<p class="h"><strong>${inline(m[1])}</strong></p>` : ""; i++; }
      else if (QUOTE.test(l)) {
        const q = [];
        while (i < ls.length && QUOTE.test(ls[i])) q.push(ls[i++].replace(QUOTE, ""));
        out += `<blockquote>${blocks(q, false, depth + 1)}</blockquote>`;
      } else if (isTable(ls, i)) i = table(ls, i, (h) => (out += h));
      else if (ITEM.test(l)) i = list(ls, i, depth, (h) => (out += h));
      else {
        const p = [l.trim()];
        let head = false;
        for (i++; i < ls.length && ls[i].trim(); i++) {
          if (SETEXT.test(ls[i])) { head = true; i++; break; }
          if (starts(ls, i)) break;
          p.push(ls[i].trim());
        }
        const html = inline(p.join("\n"));
        out += head ? `<p class="h"><strong>${html}</strong></p>` : tight ? html : `<p>${html}</p>`;
      }
    }
    return out;
  }

  function table(ls, i, emit) {
    const align = cells(ls[i + 1]).map((c) => (c.endsWith(":") ? (c.startsWith(":") ? ' class="c"' : ' class="r"') : ""));
    const row = (l, tag, cs = cells(l)) => `<tr>${align.map((a, k) => `<${tag}${a}>${inline(cs[k] || "")}</${tag}>`).join("")}</tr>`;
    let body = "";
    const head = row(ls[i], "th");
    for (i += 2; i < ls.length && ls[i].trim() && !starts(ls, i); i++) body += row(ls[i], "td");
    emit(`<div class="tbl"><table><thead>${head}</thead>${body && `<tbody>${body}</tbody>`}</table></div>`);
    return i;
  }

  // One list: items of the same kind. An item takes the lines indented to its content column (or
  // any deeper list item), plus lazy continuation lines. Blank lines between blocks make it loose.
  function list(ls, i, depth, emit) {
    const first = ITEM.exec(ls[i]), ordered = !!first[3], kind = first[2].slice(-1), items = [];
    let loose = false, gap = false;
    while (i < ls.length) {
      const m = ITEM.exec(ls[i]);
      if (!m || !!m[3] !== ordered || m[2].slice(-1) !== kind || HR.test(ls[i])) break;
      const own = m[1].length + m[2].length, rest = ls[i].slice(m[0].length);
      const w = !rest.trim() || m[0].length - own > 4 ? own + 1 : m[0].length;
      const body = [ls[i].slice(Math.min(w, m[0].length))];
      for (i++; i < ls.length; i++) {
        const l = ls[i], ind = indent(l);
        if (!l.trim()) body.push("");
        else if (ind >= w || (ind > m[1].length && ITEM.test(l))) body.push(l.slice(Math.min(ind, w)));
        else if (body[body.length - 1].trim() && !ITEM.test(l) && !starts(ls, i)) body.push(l);
        else break;
      }
      if (gap) loose = true;
      for (gap = false; body.length > 1 && !body[body.length - 1].trim(); gap = true) body.pop();
      if (body.some((b, k) => !b.trim() && k + 1 < body.length && !indent(body[k + 1]) && !ITEM.test(body[k + 1]))) loose = true;
      items.push(body);
    }
    const tag = ordered ? "ol" : "ul", start = ordered && +first[3] !== 1 ? ` start="${+first[3]}"` : "";
    emit(`<${tag}${start}>${items.map((b) => item(b, !loose, depth)).join("")}</${tag}>`);
    return i;
  }

  function item(body, tight, depth) {
    const t = /^\[([ xX])\](?:[ \t]+|$)/.exec(body[0]);
    if (t) body[0] = body[0].slice(t[0].length);
    const box = t ? `<input type="checkbox" disabled${t[1] === " " ? "" : " checked"}>` : "";
    return `<li${t ? ' class="task"' : ""}>${box}${blocks(body, tight, depth + 1)}</li>`;
  }

  /** Markdown source → safe HTML string. */
  function render(src) {
    const lines = String(src).replace(/[\uE000\uE001]/g, "\uFFFD").replace(/\r\n?/g, "\n")
      .split("\n").map((l) => l.replace(/^\t+/, (t) => "    ".repeat(t.length)).replace(/[ \t]+$/, ""));
    return blocks(lines);
  }

  window.Redline = Object.assign(window.Redline || {}, { markdown: { render } });
})();
