/* useRedline(): React hook over redline's client.js. No build step; copy this file and
 * redline-store.js into your app (bundlers often refuse imports from outside the project).
 *
 *   const { threads, drafts, connected, error, comment, reply, resolve, addDraft, submitDrafts } = useRedline();
 *   comment({ kind: "app", key: "revenue_chart" }, "make it a bar chart");
 */
import { useSyncExternalStore } from "react";
import { createRedlineStore } from "./redline-store.js";

export { threadsFor } from "./redline-store.js";

const stores = new WeakMap();

/** One store per client, shared by every component that calls the hook. */
export function storeFor(client) {
  let store = stores.get(client);
  if (!store) {
    store = createRedlineStore(client);
    stores.set(client, store);
    store.connect();
  }
  return store;
}

export function useRedline(client = globalThis.Redline && globalThis.Redline.client) {
  if (!client) throw new Error("useRedline: load client.js with data-redline-artifact, or pass a redline Client");
  const store = storeFor(client);
  const snapshot = useSyncExternalStore(store.subscribe, store.getSnapshot);
  return { ...snapshot, ...store.actions };
}
