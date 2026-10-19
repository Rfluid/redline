// Test harness for the browser side: a sandboxed daemon, a page server, and headless Chrome
// driven over the DevTools protocol. No npm dependencies (Node ≥ 22 has fetch and WebSocket).

import { spawn, execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, existsSync, rmSync } from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { join, dirname, isAbsolute } from "node:path";
import { fileURLToPath } from "node:url";

export const repo = join(dirname(fileURLToPath(import.meta.url)), "../..");
const win = process.platform === "win32";
const exe = process.env.REDLINE_BIN || join(repo, `target/release/redline${win ? ".exe" : ""}`);
/** Python on PATH: `python3` on Unix, `python` on Windows. */
export const python = win ? "python" : "python3";
const CHROMES = ["google-chrome", "google-chrome-stable", "chromium", "chromium-browser",
  "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
  ...[process.env.PROGRAMFILES, process.env["PROGRAMFILES(X86)"], process.env.LOCALAPPDATA]
    .filter(Boolean).map((d) => join(d, "Google/Chrome/Application/chrome.exe"))];
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** Agent side: the redline CLI against a throwaway state dir and port. */
export class Agent {
  constructor() {
    const home = mkdtempSync(join(tmpdir(), "rl-browser-"));
    // Own config.toml, and no background release check (tests write update.json themselves).
    this.env = { ...process.env, REDLINE_HOME: home, REDLINE_CONFIG: join(home, "config.toml"), REDLINE_NO_UPDATE_CHECK: "1",
      REDLINE_PORT: String(20000 + Math.floor(Math.random() * 20000)), RUST_LOG: "redline=debug,tower_http=debug" };
  }
  run(...args) { return execFileSync(exe, args, { env: this.env, encoding: "utf8" }); }
  json(...args) { return JSON.parse(this.run(...args)); }
  /** Starts `redline wait` from the last cursor; resolves with its JSON once comments arrive. */
  wait(session, timeout = 20) {
    return new Promise((resolve, reject) => {
      const args = ["wait", "--session", session, "--since", String(this.cursor || 0), "--timeout", String(timeout)];
      const p = spawn(exe, args, { env: this.env });
      let out = "";
      p.stdout.on("data", (d) => (out += d));
      p.on("exit", (code) => {
        if (code !== 0) return reject(new Error(`wait exited ${code}`));
        const v = JSON.parse(out);
        this.cursor = v.next;
        resolve(v);
      });
    });
  }
  log() { return readFileSync(join(this.env.REDLINE_HOME, "daemon.log"), "utf8"); }
  adminToken() { return readFileSync(join(this.env.REDLINE_HOME, "admin.token"), "utf8").trim(); }
  stop() { try { this.run("stop"); } catch {} rmSync(this.env.REDLINE_HOME, { recursive: true, force: true }); }
}

/** Serves `pages` (path → html, or JS for `*.js` paths) on a random port. */
export async function serve(pages) {
  const srv = createServer((req, res) => {
    const html = pages[req.url.split("?")[0]];
    if (html === undefined) { res.writeHead(204).end(); return; } // favicon etc.: no console noise
    const type = req.url.endsWith(".js") ? "application/javascript" : "text/html";
    res.writeHead(200, { "content-type": `${type}; charset=utf-8` }).end(html);
  });
  await new Promise((r) => srv.listen(0, "127.0.0.1", r));
  return { url: `http://127.0.0.1:${srv.address().port}`, close: () => srv.close() };
}

function findChrome() {
  if (process.env.CHROME) return process.env.CHROME;
  for (const c of CHROMES) {
    if (isAbsolute(c)) { if (existsSync(c)) return c; continue; }
    try { execFileSync(win ? "where" : "which", [c], { stdio: "ignore" }); return c; } catch {}
  }
  throw new Error("no Chrome found; set CHROME=/path/to/chrome");
}

/** Headless Chrome with one page, plus console-error and dialog tracking. */
export class Browser {
  static async launch({ dark = false } = {}) {
    const profile = mkdtempSync(join(tmpdir(), "rl-chrome-"));
    const proc = spawn(findChrome(), ["--headless=new", "--remote-debugging-port=0", `--user-data-dir=${profile}`,
      "--no-first-run", "--no-default-browser-check", "--disable-gpu", "--no-sandbox", "about:blank"],
      { stdio: "ignore" });
    const portFile = join(profile, "DevToolsActivePort");
    for (let i = 0; i < 100 && !existsSync(portFile); i++) await sleep(100);
    const port = readFileSync(portFile, "utf8").split("\n")[0];
    let target;
    for (let i = 0; i < 50 && !target; i++) {
      const list = await fetch(`http://127.0.0.1:${port}/json/list`).then((r) => r.json()).catch(() => []);
      target = list.find((t) => t.type === "page");
      if (!target) await sleep(100);
    }
    const b = new Browser(proc, profile, new WebSocket(target.webSocketDebuggerUrl));
    await new Promise((r) => b.ws.addEventListener("open", r, { once: true }));
    await b.send("Runtime.enable");
    await b.send("Page.enable");
    if (dark) await b.send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-color-scheme", value: "dark" }] });
    return b;
  }

  constructor(proc, profile, ws) {
    Object.assign(this, { proc, profile, ws, id: 0, pending: new Map(), errors: [], dialogs: [] });
    ws.addEventListener("message", (m) => {
      const msg = JSON.parse(m.data);
      if (msg.id && this.pending.has(msg.id)) {
        const { resolve, reject } = this.pending.get(msg.id);
        this.pending.delete(msg.id);
        msg.error ? reject(new Error(msg.error.message)) : resolve(msg.result);
      } else if (msg.method === "Runtime.exceptionThrown") {
        this.errors.push(msg.params.exceptionDetails.exception?.description || msg.params.exceptionDetails.text);
      } else if (msg.method === "Runtime.consoleAPICalled" && ["error", "warning"].includes(msg.params.type)) {
        this.errors.push(msg.params.args.map((a) => a.value ?? a.description).join(" "));
      } else if (msg.method === "Page.javascriptDialogOpening") {
        this.dialogs.push(msg.params.message);
        this.send("Page.handleJavaScriptDialog", { accept: true });
      }
    });
  }

  send(method, params = {}) {
    const id = ++this.id;
    this.ws.send(JSON.stringify({ id, method, params }));
    return new Promise((resolve, reject) => this.pending.set(id, { resolve, reject }));
  }

  async goto(url) {
    await this.send("Page.navigate", { url });
    await this.waitFor("document.readyState === 'complete'");
  }

  /** Evaluates `expr` (an expression, may be a promise) in the page; returns its JSON value. */
  async eval(expr) {
    const r = await this.send("Runtime.evaluate", { expression: expr, awaitPromise: true, returnByValue: true });
    if (r.exceptionDetails) throw new Error(`eval failed: ${r.exceptionDetails.exception?.description || r.exceptionDetails.text}\n  in: ${expr}`);
    return r.result.value;
  }

  async waitFor(expr, timeout = 10000) {
    const end = Date.now() + timeout;
    let last;
    while (Date.now() < end) {
      last = await this.eval(expr).catch((e) => e.message);
      if (last === true) return;
      await sleep(50);
    }
    throw new Error(`timed out waiting for: ${expr} (last: ${JSON.stringify(last)})`);
  }

  async close() {
    const exited = this.proc.exitCode !== null || this.proc.signalCode !== null
      ? Promise.resolve()
      : new Promise((resolve) => this.proc.once("exit", resolve));
    try { await this.send("Browser.close"); } catch {}
    await Promise.race([exited, sleep(3000)]);
    this.proc.kill();
    await Promise.race([exited, sleep(2000)]);
    // Chrome's helper processes can still be writing the profile for a moment after it exits.
    // A leftover temp dir is not a test failure.
    try {
      rmSync(this.profile, { recursive: true, force: true, maxRetries: 20, retryDelay: 250 });
    } catch (e) {
      console.error(`# warning: could not remove ${this.profile}: ${e.code || e.message}`);
    }
  }
}

/** Runs named async steps in order; prints TAP-ish lines; exits 1 on the first failure. */
export async function steps(list, cleanup) {
  let failed = false;
  for (const [name, fn] of list) {
    try { await fn(); console.log(`ok - ${name}`); }
    catch (e) { console.error(`FAIL - ${name}\n${e.stack || e}`); failed = true; break; }
  }
  await cleanup();
  if (failed) process.exit(1);
  console.log("all passed");
}
