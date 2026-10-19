/* redline store: framework-agnostic state over a client.js `Client`.
 * subscribe/getSnapshot fit React's useSyncExternalStore (see useRedline.js), and anything else.
 * Snapshots are immutable: every change makes a new object.
 */

// A GET /threads response can be older than a live event that already arrived. Keep the newer copy.
const newer = (a, b) => (!a || b.updated > a.updated || (b.updated === a.updated && b.messages.length > a.messages.length) ? b : a);

function upsert(list, t) {
  const i = list.findIndex((x) => x.id === t.id);
  if (i === -1) return [...list, t];
  const keep = newer(list[i], t);
  return keep === list[i] ? list : list.map((x, j) => (j === i ? keep : x));
}

export function createRedlineStore(client) {
  let state = { threads: [], drafts: client.drafts(), connected: false, error: null };
  const subs = new Set();
  const set = (patch) => { state = { ...state, ...patch }; subs.forEach((f) => f()); };

  client.on("event", (ev) => set({ threads: upsert(state.threads, ev.thread) }));
  client.on("open", () => { set({ connected: true }); refresh(); });
  client.on("close", () => set({ connected: false }));
  client.on("drafts", (drafts) => set({ drafts }));

  async function refresh() {
    try { set({ threads: (await client.threads()).reduce(upsert, state.threads), error: null }); }
    catch (e) { set({ error: String(e.message || e) }); }
  }
  // Writes refresh afterwards (the WS event also arrives; upsert dedupes) and surface errors.
  const write = (fn) => async (...args) => {
    try { const r = await fn(...args); await refresh(); return r; }
    catch (e) { set({ error: String(e.message || e) }); throw e; }
  };

  const actions = {
    refresh,
    comment: write((anchor, body) => client.comment(anchor, body)),
    reply: write((threadId, body) => client.reply(threadId, body)),
    setStatus: write((threadId, status) => client.setStatus(threadId, status)),
    resolve: write((threadId) => client.setStatus(threadId, "resolved")),
    submitDrafts: write((ids) => client.submitDrafts(ids)),
    addDraft: (draft) => client.addDraft(draft),
    removeDraft: (id) => client.removeDraft(id),
  };

  let connected = false;
  return {
    subscribe(f) { subs.add(f); return () => subs.delete(f); },
    getSnapshot: () => state,
    actions,
    /** Opens the live connection once, unless something (the overlay) already did. */
    connect() { if (!connected && !client._ws) client.connect(); connected = true; refresh(); },
  };
}

/** Threads anchored to an app key, oldest first. */
export const threadsFor = (threads, key) => threads.filter((t) => t.anchor && t.anchor.key === key);
