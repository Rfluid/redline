// Unit test for web/markdown.js (the overlay's renderer for agent replies): formatting and XSS.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import vm from "node:vm";

const ctx = { window: {} };
vm.runInNewContext(readFileSync(new URL("../../web/markdown.js", import.meta.url), "utf8"), ctx);
const md = ctx.window.Redline.markdown.render;

test("formats the supported subset", () => {
  assert.equal(md("**bold** and *it* and `x<y`"), "<p><strong>bold</strong> and <em>it</em> and <code>x&lt;y</code></p>");
  assert.equal(md("- one\n- two"), "<ul><li>one</li><li>two</li></ul>");
  assert.equal(md("1. a\n2. b"), "<ol><li>a</li><li>b</li></ol>");
  assert.equal(md("line\nbreak\n\nnext"), "<p>line<br>break</p><p>next</p>");
  assert.equal(md("intro\n \n- a\n- b"), "<p>intro</p><ul><li>a</li><li>b</li></ul>", "whitespace-only line is blank");
  assert.equal(md("```js\nif (a < b) **x**\n```"), "<pre><code>if (a &lt; b) **x**</code></pre>");
  assert.equal(md("[docs](https://x.dev/a?b=1&c=2)"), '<p><a href="https://x.dev/a?b=1&amp;c=2" target="_blank" rel="noopener noreferrer">docs</a></p>');
  assert.equal(md("# Done"), '<p class="h"><strong>Done</strong></p>');
  assert.equal(md("snake_case_name stays"), "<p>snake_case_name stays</p>");
});

test("lists: after a paragraph, nested, loose, numbered from n, tasks, lazy lines", () => {
  assert.equal(md("Changes:\n- a\n- b"), "<p>Changes:</p><ul><li>a</li><li>b</li></ul>", "a list right under a line");
  assert.equal(md("- a\n  - b\n    - c\n- d"), "<ul><li>a<ul><li>b<ul><li>c</li></ul></li></ul></li><li>d</li></ul>");
  assert.equal(md("1. a\n  - b\n2. c"), "<ol><li>a<ul><li>b</li></ul></li><li>c</li></ol>", "2-space nesting under 1.");
  assert.equal(md("- a\n\n- b"), "<ul><li><p>a</p></li><li><p>b</p></li></ul>", "one loose list, not two");
  assert.equal(md("3. a\n4. b"), '<ol start="3"><li>a</li><li>b</li></ol>');
  assert.equal(md("- [ ] todo\n- [x] done"), '<ul><li class="task"><input type="checkbox" disabled>todo</li><li class="task"><input type="checkbox" disabled checked>done</li></ul>');
  assert.equal(md("- a\nwrapped\n- b"), "<ul><li>a<br>wrapped</li><li>b</li></ul>");
  assert.equal(md("- a\n  ```\n  x()\n  ```\n- b"), "<ul><li>a<pre><code>x()</code></pre></li><li>b</li></ul>");
  assert.equal(md("- a\n- b\n\nafter"), "<ul><li>a</li><li>b</li></ul><p>after</p>");
  assert.equal(md("Steps:\n2. not a list"), "<p>Steps:<br>2. not a list</p>", "only 1. interrupts a paragraph");
  assert.equal(md("* * *"), "<hr>");
});

test("headings, quotes, rules, tables, fences", () => {
  assert.equal(md("## Title\nbody"), '<p class="h"><strong>Title</strong></p><p>body</p>');
  assert.equal(md("Title\n==="), '<p class="h"><strong>Title</strong></p>');
  assert.equal(md("> **q**\n> more"), "<blockquote><p><strong>q</strong><br>more</p></blockquote>");
  assert.equal(md("a\n\n---\n\nb"), "<p>a</p><hr><p>b</p>");
  assert.equal(md("Data:\n| k | n |\n|:-:|--:|\n| `a\\|b` | 1 |"),
    '<p>Data:</p><div class="tbl"><table><thead><tr><th class="c">k</th><th class="r">n</th></tr></thead><tbody><tr><td class="c"><code>a|b</code></td><td class="r">1</td></tr></tbody></table></div>');
  assert.equal(md("~~~py\nx = 1\n~~~"), "<pre><code>x = 1</code></pre>");
  assert.equal(md("```\nunclosed **x**"), "<pre><code>unclosed **x**</code></pre>");
});

test("inline: nesting, strike, escapes, code runs, autolinks", () => {
  assert.equal(md("**a *b* c** *a **b** c* ***x*** ~~y~~ __z__"),
    "<p><strong>a <em>b</em> c</strong> <em>a <strong>b</strong> c</em> <strong><em>x</em></strong> <del>y</del> <strong>z</strong></p>");
  assert.equal(md("2 * 3 * 4, **open and snake_case_x"), "<p>2 * 3 * 4, **open and snake_case_x</p>");
  assert.equal(md("\\*no\\* \\`no\\` `` a`b ``"), "<p>*no* `no` <code>a`b</code></p>");
  const a = (u) => `<a href="${u}" target="_blank" rel="noopener noreferrer">`;
  assert.equal(md("See https://x.dev/a_(b). (https://x.dev/c)"), `<p>See ${a("https://x.dev/a_(b)")}https://x.dev/a_(b)</a>. (${a("https://x.dev/c")}https://x.dev/c</a>)</p>`);
  assert.equal(md("<https://x.dev?a=1&b=2>"), `<p>${a("https://x.dev?a=1&amp;b=2")}https://x.dev?a=1&amp;b=2</a></p>`);
  assert.equal(md('[**b** `c`](https://x.dev/p_(q) "t")'), `<p>${a("https://x.dev/p_(q)")}<strong>b</strong> <code>c</code></a></p>`);
});

test("pathological input stays fast and shallow", () => {
  const N = 64 * 1024; // MAX_MESSAGE_BYTES
  const inputs = ["*a ".repeat(N / 3), "_a ".repeat(N / 3), "~~a ".repeat(N / 4), "[".repeat(N), "[`".repeat(N / 2), ">".repeat(N),
    Array.from({ length: 300 }, (_, i) => `${"`".repeat(i + 1)}x`).join(""), `${"|a".repeat(N / 4)}\n${"|-".repeat(N / 4)}`,
    `https://x.dev/${")".repeat(N / 2)}`, Array.from({ length: 350 }, (_, i) => `${" ".repeat(i)}- x`).join("\n")];
  for (const p of inputs) {
    const t = performance.now();
    md(p);
    assert.ok(performance.now() - t < 1000, `${p.slice(0, 20)}… took ${performance.now() - t} ms`);
  }
});

test("no HTML or script URLs survive", () => {
  const payloads = [
    "<img src=x onerror=alert(1)>",
    "<script>alert(1)</script>",
    "[x](javascript:alert(1))",
    "[x](JAVASCRIPT:alert(1))",
    "[x](data:text/html,<script>alert(1)</script>)",
    '[x](https://ok.dev" onmouseover="alert(1))',
    "**<svg onload=alert(1)>**",
    "`<b>`",
    "```\n</code></pre><img src=x onerror=alert(1)>\n```",
    "| <img src=x onerror=alert(1)> |\n|---|\n| <script>x</script> |",
    "> <svg onload=alert(1)>",
    "- [x] <img src=x onerror=alert(1)>",
    "<javascript:alert(1)>",
    "[x](<javascript:alert(1)>)",
    "[x](\\javascript:alert(1))",
    "[x](https://ok.dev/\"onclick=\"alert(1))",
    "https://ok.dev/\"onmouseover=\"alert(1)",
    "`\uE0000\uE001` \uE0000\uE001",
  ];
  for (const p of payloads) {
    const out = md(p);
    assert.doesNotMatch(out, /<(img|script|svg|b)\b/i, `${p} → ${out}`);
    assert.doesNotMatch(out, /href="(?!https?:|mailto:)/i, `${p} → ${out}`);
    assert.doesNotMatch(out, /\son\w+="/i, `${p} → ${out}`);
  }
});
