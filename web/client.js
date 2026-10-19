/* redline client: protocol only, no UI.
 *
 *   <script src="http://127.0.0.1:7465/client.js"
 *           data-redline-artifact="art_..." data-redline-token="..."></script>
 *
 * Exposes window.Redline = { Client, anchors, client }. `client` is auto-created from the
 * data attributes. The daemon serves /client.js as web/anchors.js followed by this file. Anything can drive it: the bundled overlay, a React app, a Streamlit
 * component, a test.
 */
(function () {
  "use strict";
  const script = document.currentScript;

  class Emitter {
    constructor() { this._h = {}; }
    on(ev, fn) { (this._h[ev] ||= []).push(fn); return () => this.off(ev, fn); }
    off(ev, fn) { this._h[ev] = (this._h[ev] || []).filter((f) => f !== fn); }
    emit(ev, data) { (this._h[ev] || []).forEach((f) => { try { f(data); } catch (e) { console.error(e); } }); }
  }

  class Client extends Emitter {
    /** @param {{server: string, artifact: string, token: string}} o */
    constructor(o) {
      super();
      this.server = o.server.replace(/\/$/, "");
      this.artifact = o.artifact;
      this.token = o.token;
      this._draftKey = `redline:drafts:${this.artifact}`;
      this._retry = 500;
    }

    async _fetch(path, opts = {}) {
      const r = await fetch(this.server + path, {
        ...opts,
        headers: { "content-type": "application/json", authorization: `Bearer ${this.token}`, ...(opts.headers || {}) },
      });
      const body = await r.json().catch(() => ({}));
      if (!r.ok) throw new Error(body.error || `redline ${r.status}`);
      return body;
    }

    threads() { return this._fetch(`/api/artifacts/${this.artifact}/threads`); }

    /** Send new threads and replies in one go. Each is delivered to the agent as an event. */
    submit({ threads = [], replies = [] }) {
      return this._fetch(`/api/artifacts/${this.artifact}/submit`, { method: "POST", body: JSON.stringify({ threads, replies }) });
    }
    /** `image` (optional, region anchors): `{mime, data}` with base64 data, e.g. from anchors.snapshot(). */
    comment(anchor, body, image) { return this.submit({ threads: [{ anchor, body, ...(image ? { image } : {}) }] }); }
    reply(threadId, body) { return this.submit({ replies: [{ thread_id: threadId, body }] }); }
    /** An attachment (region screenshot) as an object URL, fetched with the token; cached. */
    async attachmentURL(id) {
      this._att ||= new Map();
      if (!this._att.has(id)) {
        const r = await fetch(`${this.server}/api/attachments/${encodeURIComponent(id)}`, { headers: { authorization: `Bearer ${this.token}` } });
        if (!r.ok) throw new Error(`redline ${r.status}`);
        this._att.set(id, URL.createObjectURL(await r.blob()));
      }
      return this._att.get(id);
    }
    /** What's known about a newer redline: `{current, latest, available, notify, notes, command, …}`. */
    update() { return this._fetch(`/api/artifacts/${this.artifact}/update`); }
    /** Hide the update notice: `{version}` skips that release, `{all: true}` turns update notices off. */
    dismissUpdate(d) {
      return this._fetch(`/api/artifacts/${this.artifact}/update/dismiss`, { method: "POST", body: JSON.stringify(d) });
    }
    setStatus(threadId, status) {
      return this._fetch(`/api/threads/${threadId}/status`, { method: "POST", body: JSON.stringify({ status }) });
    }

    // Drafts stay in this browser until submitted; the agent never sees them.
    drafts() { try { return JSON.parse(localStorage.getItem(this._draftKey) || "[]"); } catch { return []; } }
    _saveDrafts(d) { try { localStorage.setItem(this._draftKey, JSON.stringify(d)); } catch {} this.emit("drafts", d); }
    addDraft(draft) {
      const d = this.drafts();
      d.push({ id: `d_${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`, ...draft });
      this._saveDrafts(d);
    }
    removeDraft(id) { this._saveDrafts(this.drafts().filter((x) => x.id !== id)); }
    async submitDrafts(ids) {
      const all = this.drafts();
      const pick = ids ? all.filter((d) => ids.includes(d.id)) : all;
      const threads = pick.filter((d) => !d.thread_id).map(({ anchor, body, image }) => ({ anchor, body, ...(image ? { image } : {}) }));
      const replies = pick.filter((d) => d.thread_id).map(({ thread_id, body }) => ({ thread_id, body }));
      const events = await this.submit({ threads, replies });
      this._saveDrafts(all.filter((d) => !pick.includes(d)));
      return events;
    }

    /** Live events (agent replies, status changes, other tabs' comments). Emits "event", "open", "close". */
    connect() {
      const url = `${this.server.replace(/^http/, "ws")}/api/artifacts/${this.artifact}/ws?token=${encodeURIComponent(this.token)}`;
      const ws = (this._ws = new WebSocket(url));
      ws.onopen = () => { this._retry = 500; this.emit("open"); };
      ws.onmessage = (m) => { try { this.emit("event", JSON.parse(m.data)); } catch {} };
      ws.onclose = () => {
        this.emit("close");
        if (this._closed) return;
        setTimeout(() => this.connect(), this._retry);
        this._retry = Math.min(this._retry * 2, 15000);
      };
      return this;
    }
    close() { this._closed = true; this._ws && this._ws.close(); }
  }

  const anchors = (window.Redline && window.Redline.anchors) || {};
  let client = null;
  const id = script && script.dataset.redlineArtifact;
  if (id) {
    client = new Client({ server: new URL(script.src).origin, artifact: id, token: script.dataset.redlineToken || "" });
  }
  window.Redline = Object.assign(window.Redline || {}, { Client, anchors, client });
  window.dispatchEvent(new CustomEvent("redline:ready", { detail: window.Redline }));
})();
