/* redline popovers: the thread, draft and compose popovers' markup, and where they go
 * (.design/components.md › Thread popover / Compose popover, .design/layout.md › Popovers).
 * Pure functions: HTML strings with every user string escaped, and one placement helper.
 * Served in front of overlay.js.
 */
(function () {
  "use strict";
  const R = window.Redline;
  const icon = (n) => R.ui.icons[n];
  const esc = (s) => String(s).replace(/[&<>"]/g, (ch) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[ch]);
  const MAC = /Mac|iP(hone|ad|od)/.test(navigator.platform || navigator.userAgent);
  const KEY = { send: MAC ? "⌘↵" : "Ctrl+Enter", draft: MAC ? "⌥↵" : "Alt+Enter" };
  const STATUS = { draft: "Draft", open: "Open", working: "Working", answered: "Answered", resolved: "Resolved" };
  const TAGS = {
    fuzzy: ["Edited", "The commented text was edited; this is where it is now"],
    moved: ["Moved", "The commented spot moved; this is where it was"],
    // Answered and gone: usually because the agent made the change you asked for.
    changed: ["Text changed", "The commented text has changed since; the quote is what was commented"],
    none: ["Detached: showing original quote", "The commented text changed or is gone; the quote is what was commented", "detached"],
  };

  function tag(status, match) {
    const t = TAGS[match === "none" && (status === "answered" || status === "resolved") ? "changed" : match];
    return t ? `<span class="tag${t[2] ? ` ${t[2]}` : ""}" title="${t[1]}">${t[0]}</span>` : "";
  }
  const chip = (status) => `<span class="st ${status}">${STATUS[status]}</span>`;
  const dots = `<span class="dots" aria-hidden="true"><i></i><i></i><i></i></span>`;

  /** The ball is in the agent's court: it's working, or the last word is the user's. */
  const pending = (t) => t.status === "working" || (t.status === "open" && t.messages.length > 0 && t.messages.at(-1).author === "user");

  /** Where a pending thread stands, under its last message: a typing row, like a chat's. */
  function waiting(t) {
    if (!pending(t)) return "";
    const working = t.status === "working", text = working ? t.note || "Agent is working on it" : "Sent · waiting for the agent";
    return `<div class="note${working ? "" : " wait"}" role="status"><span class="av agent" aria-hidden="true">${icon("pen")}</span>
      <span class="bub">${dots}</span><span class="nt">${esc(text)}</span></div>`;
  }

  /** The quote line: the commented words, a spot, or the page. */
  function quote(a) {
    if (a && a.kind === "region") return `<div class="q" data-a="goto">${icon("spot")}${esc(a.selector || "spot")}</div>`;
    const words = a && (a.quote || a.label || a.selector || a.key);
    return `<div class="q" data-a="goto">${words ? `“<u>${esc(words)}</u>”` : "(page)"}</div>`;
  }
  const thumb = (img, att) => img ? `<img class="thumb" alt="screenshot" src="data:${esc(img.mime)};base64,${esc(img.data)}">`
    : att ? `<img class="thumb" alt="screenshot" data-att="${esc(att)}">` : "";

  function ago(ms) {
    const s = (Date.now() - ms) / 1000;
    if (s < 45) return "just now";
    if (s < 3600) return `${Math.round(s / 60)} min`;
    if (s < 86400) return `${Math.round(s / 3600)} h`;
    return new Date(ms).toLocaleDateString();
  }

  function message(m) {
    const agent = m.author === "agent";
    return `<div class="msg ${agent ? "agent" : "user"}"><span class="av${agent ? " agent" : ""}" aria-hidden="true">${agent ? icon("pen") : "Y"}</span>
      <div><div class="who"><b${agent ? ` class="agent"` : ""}>${agent ? "Agent" : "You"}</b>${m.created ? ` · ${ago(m.created)}` : ""}</div>
      <div class="body${agent ? " md" : ""}">${agent ? R.markdown.render(m.body) : esc(m.body)}</div></div></div>`;
  }

  /** A thread: header, quote, messages, where it stands, reply drafts, reply box. */
  function thread(t, { n, match, replyDrafts = [] }) {
    const resolved = t.status === "resolved";
    return `<div class="ph"><span class="num">#${n}</span>${chip(t.status)}<span class="sp"></span>
      <button class="btn sm" data-a="${resolved ? "reopen" : "resolve"}">${resolved ? "Reopen" : `${icon("check")}Resolve`}</button>
      <button class="btn g sm i" data-a="close" aria-label="Close">${icon("close")}</button></div>
    <div class="pb">${quote(t.anchor)}${tag(t.status, match)}${thumb(null, t.anchor && t.anchor.image)}
      ${t.messages.map(message).join("")}
      ${waiting(t)}
      ${replyDrafts.map((d) => `<div class="rdraft"><div class="who"><b>Draft reply</b></div><div class="body">${esc(d.body)}</div>
        <div class="row"><button class="btn g sm" data-a="drop-draft" data-id="${esc(d.id)}">Discard</button>
        <button class="btn p sm" data-a="send-draft" data-id="${esc(d.id)}">Send</button></div></div>`).join("")}
    </div>
    <div class="pf"><label class="reply"><span class="sr">Reply</span><textarea name="redline-reply" rows="1" placeholder="Reply…"></textarea>
      <button class="btn p i" data-a="reply" aria-label="Send reply">${icon("send")}</button></label>
      <span class="hint keys"><kbd>${KEY.send}</kbd> send · <kbd>${KEY.draft}</kbd> save as draft</span></div>`;
  }

  /** A comment kept in this browser, not sent yet. */
  function draft(d, n, match) {
    return `<div class="ph"><span class="num">#${n}</span>${chip("draft")}<span class="sp"></span>
      <button class="btn g sm i" data-a="close" aria-label="Close">${icon("close")}</button></div>
    <div class="pb">${quote(d.anchor)}${tag("open", match)}${thumb(d.image)}<div class="msg"><span class="av" aria-hidden="true">Y</span>
      <div><div class="who"><b>You</b> · not sent</div><div class="body">${esc(d.body)}</div></div></div></div>
    <div class="pf"><span class="hint">Only you can see drafts</span><button class="btn g" data-a="drop-draft" data-id="${esc(d.id)}">Discard</button>
      <button class="btn p" data-a="send-draft" data-id="${esc(d.id)}">Send</button></div>`;
  }

  /** A new comment on `anchor` (and its spot screenshot, if any). */
  function compose(anchor, image) {
    return `<div class="pb">${quote(anchor)}${thumb(image)}
      <label><span class="sr">Comment for the agent</span><textarea name="redline-comment" placeholder="Comment for the agent…"></textarea></label></div>
    <div class="pf"><span class="hint keys"><kbd>${KEY.send}</kbd> send · <kbd>Esc</kbd> cancel</span>
      <button class="btn" data-a="draft">Save draft</button><button class="btn p" data-a="send">Send</button></div>`;
  }

  /** Put `pop` next to its marker (`ref`), else beside or under the highlight (`hl`); viewport rects.
   *  `below`: straight under `hl` (the compose box, so the selection and the margin stay visible). */
  function place(pop, ref, hl, below) {
    pop.style.left = pop.style.top = "";
    if (matchMedia("(max-width: 600px)").matches) return; // a bottom sheet, from CSS
    const w = pop.offsetWidth, h = pop.offsetHeight;
    let left, top;
    if (!ref && !hl) { left = innerWidth - w - 24; top = 80; } else if (below && hl) {
      left = Math.max(8, Math.min(hl.left, innerWidth - w - 8));
      top = hl.bottom + 8 + h > innerHeight - 8 ? hl.top - 8 - h : hl.bottom + 8;
    } else {
      const r = ref || hl, a = hl || ref;
      left = r.right + 14; top = r.top - 12;
      if (left + w > innerWidth - 8) {
        left = a.left - 14 - w;
        if (left < 8) { left = Math.max(8, Math.min(a.left, innerWidth - w - 8)); top = a.bottom + 8; }
      }
    }
    top = Math.max(8, Math.min(top, innerHeight - h - 8));
    pop.style.left = `${left + scrollX}px`;
    pop.style.top = `${top + scrollY}px`;
  }

  window.Redline = Object.assign(R, { ui: Object.assign(R.ui || {}, { popover: { thread, draft, compose, place, chip, quote, esc, pending, KEY } }) });
})();
