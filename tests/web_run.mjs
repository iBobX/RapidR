// Runs programs on the web runtime's own test page (tests/web_kernel.html:
// the runtime on its own, the UI kernel its GUI host — docs/web-host-plan.md)
// for the browser suites that check what a program does on the web: its
// output, its windows (read and driven with tests/web_kernel_page.mjs), its
// dialogs, files and storage, and the debugger's side of RapidR's program
// session protocol (rapidr-session), as RapidR Studio's run frame
// (ide/web/run.html) speaks it.
//
//   const r = await openRunner(browser, { deviceScaleFactor: 2 });
//   await r.run(source, { assets, args, name, files });
//   await r.waitOutput("done");             r.output()  r.errors()
//   await k.click(r.page, "Button1");       (the program's windows are the page's)
//   await r.stop();
//
//   await r.debug(source, { breakpoints: [6] });
//   const stop = await r.waitStopped();     (the `stopped` event: line, reason)
//   await r.request({ type: "stackTrace" }); r.send({ type: "stepOver" });
//
// Each run (and each debug session) is a fresh load of the page, as each run
// in an IDE is a fresh frame: nothing of the program before is left. The
// browser keeps the runtime cached between loads.

export const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";

/// Assets as the compiler takes them: an object (name → data URL) or a list
/// of { name, dataUrl } (each also under `assets/name`, as a project's).
export function assetMap(assets) {
  if (!assets) return {};
  if (!Array.isArray(assets)) return assets;
  const out = {};
  for (const a of assets) {
    out[a.name] = a.dataUrl;
    out[`assets/${a.name}`] = a.dataUrl;
  }
  return out;
}

/// A runner on its own page of `where` (a Browser or a BrowserContext);
/// `pageOptions` as Browser.newPage's (deviceScaleFactor, viewport, …).
export async function openRunner(where, pageOptions = {}) {
  const page = await where.newPage(pageOptions);
  const r = new Runner(page);
  await r.init();
  return r;
}

class Runner {
  constructor(page) {
    this.page = page;
    /// What the program printed (stdout: console.log / info), line by line.
    this.lines = [];
    /// Errors: the runtime's console.error / warn lines, the page's uncaught
    /// errors, a compile error ("compile: …").
    this.errs = [];
    /// The page's uncaught errors since the runner opened (all runs).
    this.pageErrors = [];
    this.session = null;
    page.on("console", (m) => {
      const t = m.type();
      if (t === "log" || t === "info") this.lines.push(...m.text().split("\n"));
      else if (t === "error" || t === "warning") {
        // (the page's own resources: a favicon the test server hasn't)
        if (/Failed to load resource/.test(m.text())) return;
        this.errs.push(m.text());
      }
    });
    page.on("pageerror", (e) => {
      this.errs.push(e.message);
      this.pageErrors.push(e.message);
    });
  }

  async init() {
    await this.page.exposeFunction("__rr_session_event", (json) => this.onSessionEvent(json));
  }

  /// Loads the runtime's page afresh (the program before is gone).
  async load() {
    // (the page before goes first: what it printed has all arrived once
    // the blank page is in, and is no part of this run)
    this.session = null;
    await this.goto("about:blank");
    this.lines = [];
    this.errs = [];
    await this.goto(`${URL_BASE}/tests/web_kernel.html`);
    await this.page.waitForFunction(() => window.rrReady, null, { timeout: 30000 });
  }

  /// (a program may navigate its page itself — a router, a link — while
  /// it goes: the navigation is tried again)
  async goto(url) {
    for (let attempt = 1; ; attempt++) {
      try {
        return await this.page.goto(url, { waitUntil: "load" });
      } catch (e) {
        if (attempt >= 4 || !/interrupted by another navigation|ERR_ABORTED/.test(e.message)) throw e;
        await this.page.waitForTimeout(200);
      }
    }
  }

  /// Compiles `source` (or project `files`: name → text, `name` the main
  /// one) and runs it. `assets`: the project's files ($RESOURCE, pictures,
  /// databases); `args`: its command line. Resolves once it has started
  /// (its main code may still be running). Returns false on a compile error
  /// (in `errs`).
  async run(source, { assets = null, args = [], name = "program", files = null, env = null } = {}) {
    await this.load();
    const result = await this.page.evaluate(({ source, assets, args, name, files, env }) => {
      window.__rapidr_assets = assets;
      window.RAPIDR_ARGS = args;
      let bc;
      try {
        bc = files ? window.rr.compile_files(name, files, assets) : window.rr.compile(source, name, assets);
      } catch (e) {
        return "compile: " + (e && e.message ? e.message : e);
      }
      if (env) window.rr.rapidr_set_test_env(env);
      try {
        window.rr.rapidr_run_bc(bc);
      } catch (e) {
        console.error("[run error] " + (e && e.message ? e.message : e));
      }
      return null;
    }, { source, assets: assetMap(assets), args, name, files, env });
    if (result) {
      this.errs.push(result);
      return false;
    }
    return true;
  }

  /// Ends the program: the page goes.
  async stop() {
    this.session = null;
    await this.goto("about:blank");
  }

  /// What the program printed, its lines joined.
  output() {
    return this.lines.join("\n");
  }

  errors() {
    return this.errs.join("\n");
  }

  /// Whether the program has ended (END, or its main code ran out with no
  /// form left).
  ended() {
    return this.lines.includes("[RapidR] Program ended.");
  }

  /// Waits until the output has `what` (a string or a RegExp); true if it came.
  async waitOutput(what, ms = 15000) {
    const has = () => (typeof what === "string" ? this.output().includes(what) : what.test(this.output()));
    for (let waited = 0; waited < ms && !has(); waited += 50) await this.page.waitForTimeout(50);
    return has();
  }

  /// Waits until `cond()` (sync or async) is true; true if it came.
  async until(cond, ms = 15000) {
    for (let waited = 0; waited < ms; waited += 50) {
      if (await cond()) return true;
      await this.page.waitForTimeout(50);
    }
    return !!(await cond());
  }

  /// Property `prop` of component `name`, as the program reads it.
  prop(name, prop) {
    return this.page.evaluate(({ name, prop }) => window.rr.rapidr_get_prop(name, prop), { name, prop });
  }

  /// Whether the program's main code has run to its end.
  mainDone() {
    return this.page.evaluate(() => window.rr.rapidr_main_done());
  }

  // ---- the debugger: the program under the session protocol ----------------

  /// Compiles `source` and opens it under a session: once the runtime is
  /// `ready`, the breakpoints (lines of `name`) are set and it starts.
  async debug(source, { breakpoints = [], name = "program", assets = null } = {}) {
    await this.load();
    this.session = { name, breakpoints, seq: 0, pending: new Map(), events: [], stopped: null, exited: false, waiters: [] };
    const result = await this.page.evaluate(({ source, name, assets }) => {
      window.__rapidr_assets = assets;
      let bc;
      try {
        bc = window.rr.compile(source, name, assets);
      } catch (e) {
        return "compile: " + (e && e.message ? e.message : e);
      }
      window.rr.session_open(bc, name, (json) => window.__rr_session_event(json));
      return null;
    }, { source, name, assets: assetMap(assets) });
    if (result) {
      this.errs.push(result);
      return false;
    }
    return true;
  }

  onSessionEvent(json) {
    const s = this.session;
    if (!s) return;
    let m;
    try { m = JSON.parse(json); } catch { return; }
    s.events.push(m);
    if (m.re) {
      const w = s.pending.get(m.re);
      if (w) {
        s.pending.delete(m.re);
        m.type === "error" ? w.reject(new Error(m.message)) : w.resolve(m);
      }
      return;
    }
    switch (m.type) {
      case "ready":
        this.send({ type: "setBreakpoints", file: s.name, breakpoints: s.breakpoints.map((line) => ({ line })) });
        this.send({ type: "start", debug: true });
        break;
      case "stopped": s.stopped = m; break;
      case "continued": s.stopped = null; break;
      case "output":
        (m.stream === "stderr" ? this.errs : this.lines).push(...String(m.text).replace(/\n$/, "").split("\n"));
        break;
      case "exited":
        s.exited = true;
        s.stopped = null;
        for (const w of s.pending.values()) w.reject(new Error("the program has ended"));
        s.pending.clear();
        break;
    }
  }

  /// Sends a request to the program's session; returns its number.
  send(command) {
    const s = this.session;
    const seq = ++s.seq;
    const json = JSON.stringify({ seq, ...command });
    this.page.evaluate((j) => window.rr.session_request(j), json).catch(() => {});
    return seq;
  }

  /// Sends a request; resolves with its reply (rejects with an `error` one).
  request(command) {
    return new Promise((resolve, reject) => {
      const s = this.session;
      const seq = s.seq + 1;
      s.pending.set(seq, { resolve, reject });
      this.send(command);
    });
  }

  /// Whether the program is stopped (a breakpoint, a step, a pause).
  get paused() {
    return !!this.session?.stopped;
  }

  /// Waits for the program to stop; the `stopped` event (its `line`,
  /// `reason`), or null.
  async waitStopped(ms = 20000) {
    for (let waited = 0; waited < ms; waited += 50) {
      if (this.session?.stopped) return this.session.stopped;
      if (this.session?.exited) return null;
      await this.page.waitForTimeout(50);
    }
    return this.session?.stopped ?? null;
  }

  /// Waits for the session's program to end; true if it did.
  async waitExited(ms = 20000) {
    for (let waited = 0; waited < ms && !this.session?.exited; waited += 50) await this.page.waitForTimeout(50);
    return !!this.session?.exited;
  }

  /// The stopped program's call stack ([{ id, name, line }]) and its top
  /// frame's variables: { stack, locals, globals } (values as the debugger
  /// shows them).
  async snapshot() {
    const { frames } = await this.request({ type: "stackTrace" });
    const vars = { locals: {}, globals: {} };
    const top = frames[0];
    if (top) {
      const { scopes } = await this.request({ type: "scopes", frame: top.id });
      for (const scope of scopes) {
        const { variables } = await this.request({ type: "variables", ref: scope.ref });
        const into = scope.name === "Globals" ? vars.globals : vars.locals;
        for (const v of variables) into[v.name] = sessionValue(v);
      }
    }
    return { stack: frames.map((f) => ({ name: f.name, line: f.line })), ...vars };
  }
}

/// A variable's value (rapidr-session's Variable) as a debugger shows it.
export function sessionValue(v) {
  switch (v.kind) {
    case "String": return v.value.slice(1, -1).replace(/""/g, '"');
    case "Integer": case "Double": return Number(v.value);
    case "Boolean": return v.value === "True";
    case "Empty": return null;
    default: return v.value;
  }
}
