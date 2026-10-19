// Unit test for bindings/react/redline-store.js with a fake client (no React, no daemon).
import assert from "node:assert/strict";
import test from "node:test";
import { createRedlineStore, threadsFor } from "../../bindings/react/redline-store.js";

class FakeClient {
  constructor() { this.h = {}; this.server = []; this.draftList = []; this.connects = 0; }
  on(ev, fn) { (this.h[ev] ||= []).push(fn); }
  emit(ev, d) { (this.h[ev] || []).forEach((f) => f(d)); }
  drafts() { return this.draftList; }
  async threads() { return this.server; }
  async comment(anchor, body) {
    const t = { id: `t${this.server.length}`, anchor, status: "open", updated: 1, messages: [{ author: "user", body }] };
    this.server = [...this.server, t];
    return [{ thread: t }];
  }
  connect() { this.connects++; }
}

const tick = () => new Promise((r) => setImmediate(r));

test("comment, live event and stale refresh", async () => {
  const c = new FakeClient(), s = createRedlineStore(c);
  let renders = 0;
  s.subscribe(() => renders++);
  s.connect();
  assert.equal(c.connects, 1);
  await s.actions.comment({ kind: "app", key: "k" }, "hi");
  const before = s.getSnapshot();
  assert.equal(before.threads.length, 1);

  // A live agent reply arrives, then an older GET /threads lands: the reply must survive.
  const answered = { ...before.threads[0], status: "answered", updated: 2, messages: [...before.threads[0].messages, { author: "agent", body: "ok" }] };
  c.emit("event", { thread: answered });
  await s.actions.refresh();
  assert.equal(s.getSnapshot().threads[0].status, "answered");
  assert.notEqual(s.getSnapshot(), before, "snapshots are new objects");
  assert.ok(renders > 0);
  assert.equal(threadsFor(s.getSnapshot().threads, "k").length, 1);
  assert.equal(threadsFor(s.getSnapshot().threads, "other").length, 0);
});

test("connection and drafts state; connect reuses an existing socket", async () => {
  const c = new FakeClient();
  c._ws = {}; // the overlay already connected
  const s = createRedlineStore(c);
  s.connect();
  assert.equal(c.connects, 0);
  c.emit("open");
  await tick();
  assert.equal(s.getSnapshot().connected, true);
  c.emit("drafts", [{ id: "d1" }]);
  assert.deepEqual(s.getSnapshot().drafts, [{ id: "d1" }]);
  c.emit("close");
  assert.equal(s.getSnapshot().connected, false);
});

test("errors are surfaced and rethrown", async () => {
  const c = new FakeClient();
  c.comment = async () => { throw new Error("redline 413"); };
  const s = createRedlineStore(c);
  await assert.rejects(s.actions.comment({}, "x"), /413/);
  assert.equal(s.getSnapshot().error, "redline 413");
});
