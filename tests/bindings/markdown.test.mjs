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
  assert.equal(md("# Done"), "<p><strong>Done</strong></p>");
  assert.equal(md("snake_case_name stays"), "<p>snake_case_name stays</p>");
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
  ];
  for (const p of payloads) {
    const out = md(p);
    assert.doesNotMatch(out, /<(img|script|svg|b)\b/i, `${p} → ${out}`);
    assert.doesNotMatch(out, /href="(?!https?:|mailto:)/i, `${p} → ${out}`);
    assert.doesNotMatch(out, /\son\w+="/i, `${p} → ${out}`);
  }
});
