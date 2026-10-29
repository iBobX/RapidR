// A client for the network examples' servers (tests/examples_run.mjs's
// `client`): it waits for the example to listen on 127.0.0.1:<port>, sends
// its requests, and writes what came back to <outfile>, one line each, then
// <outfile>.done. The examples suite reads the lines with the program's
// output (its `expect`).
//
//   node tests/http_client_probe.mjs echo <port> <outfile>
//       (network/echo_server.rr: 3 POSTs — a word, 1 MB by Content-Length,
//       1 MB chunked — each sent back whole)
//   node tests/http_client_probe.mjs api <port> <outfile> <token>
//       (network/local_api.rr: the refusals, /hello, /echo, /events, a body
//       over MaxRequestSize)
//   node tests/http_client_probe.mjs mcp <port> <outfile> <token>
//       (network/mcp_tools.rr, RMcpServer: the refusals; an earlier
//       revision's session — initialize, tools/list, tools/call, a wrong
//       call, DELETE —; the current revision — server/discover, calls
//       with their headers, a header that doesn't match, a version it
//       doesn't speak, a method it doesn't have)

import http from "node:http";
import net from "node:net";
import { writeFileSync, appendFileSync } from "node:fs";

const [mode, port, outfile, token] = process.argv.slice(2);
writeFileSync(outfile, "");
const say = (line) => appendFileSync(outfile, `client: ${line}\n`);

async function listening(ms = 30000) {
  const until = Date.now() + ms;
  while (Date.now() < until) {
    const ok = await new Promise((resolve) => {
      const s = net.connect({ host: "127.0.0.1", port: Number(port) }, () => { s.end(); resolve(true); });
      s.on("error", () => resolve(false));
    });
    if (ok) return true;
    await new Promise((r) => setTimeout(r, 200));
  }
  return false;
}

/// One request: { status, headers, body } (body as text); `chunks`: the body
/// sent in pieces without a Content-Length (chunked).
/// (`Expect: 100-continue` in `headers`: the body waits for the server's
/// go-ahead, as curl's large uploads do — a refusal comes instead)
function request({ method = "GET", path = "/", headers = {}, body, chunks }) {
  return new Promise((resolve) => {
    const req = http.request({ host: "127.0.0.1", port: Number(port), method, path, headers, agent: false }, (res) => {
      const parts = [];
      res.on("data", (d) => parts.push(d));
      res.on("end", () => resolve({ status: res.statusCode, headers: res.headers, body: Buffer.concat(parts).toString("utf8") }));
    });
    req.setTimeout(20000, () => req.destroy(new Error("timeout")));
    req.on("error", (e) => resolve({ status: 0, headers: {}, body: String(e.message) }));
    const send = () => {
      if (chunks) for (const c of chunks) req.write(c);
      else if (body !== undefined) req.write(body);
      req.end();
    };
    if (headers.Expect) req.on("continue", send);
    else send();
  });
}

const big = "0123456789abcdef".repeat(65536);
const auth = token ? { Authorization: `Bearer ${token}` } : {};

if (!(await listening())) {
  say("the server never listened");
} else if (mode === "echo") {
  const a = await request({ method: "POST", path: "/echo", body: "hello" });
  say(`echo word ${a.status} ${a.body === "hello" ? "same" : JSON.stringify(a.body.slice(0, 40))}`);
  const b = await request({ method: "POST", path: "/echo", headers: { "Content-Length": big.length }, body: big });
  say(`echo 1 MB ${b.status} ${b.body === big ? "same" : `${b.body.length} characters`}`);
  const c = await request({ method: "POST", path: "/echo", chunks: [big.slice(0, 300000), big.slice(300000, 700000), big.slice(700000)] });
  say(`echo 1 MB chunked ${c.status} ${c.body === big ? "same" : `${c.body.length} characters`}`);
} else if (mode === "api") {
  say(`no token ${(await request({ path: "/hello" })).status}`);
  say(`wrong token ${(await request({ path: "/hello", headers: { Authorization: "Bearer nope" } })).status}`);
  say(`foreign origin ${(await request({ path: "/hello", headers: { ...auth, Origin: "https://evil.example" } })).status}`);
  say(`foreign host ${(await request({ path: "/hello", headers: { ...auth, Host: `evil.example:${port}` } })).status}`);
  // (a link or an image on another site's page: no Origin, the browser says cross-site)
  say(`cross-site ${(await request({ path: "/hello", headers: { ...auth, "Sec-Fetch-Site": "cross-site" } })).status}`);
  const h = await request({ path: "/hello", headers: auth });
  say(`hello ${h.status} ${h.body.trim()}`);
  const json = JSON.stringify({ jsonrpc: "2.0", id: 1, method: "ping" });
  const e = await request({ method: "POST", path: "/echo", headers: { ...auth, "Content-Type": "application/json" }, body: json });
  say(`echo ${e.status} ${e.headers["content-type"]} ${e.body === json ? "same" : "different"}`);
  const s = await request({ path: "/events", headers: auth });
  const events = s.body.split("\n\n").filter((x) => x.includes("data:"));
  say(`events ${s.status} ${s.headers["content-type"]} ${events.length} ${events.map((x) => x.match(/data: (.*)/)[1]).join(" ")}`);
  const t = await request({ method: "POST", path: "/echo", headers: { ...auth, "Content-Length": 5 * 1024 * 1024, Expect: "100-continue" }, body: "a".repeat(5 * 1024 * 1024) });
  say(`too large ${t.status}`);
  say(`not found ${(await request({ path: "/nothing", headers: auth })).status}`);
} else if (mode === "mcp") {
  const MODERN = "2026-07-28";
  const post = (msg, headers = {}) => request({ method: "POST", path: "/mcp", headers: { ...auth, "Content-Type": "application/json", Accept: "application/json, text/event-stream", ...headers }, body: JSON.stringify(msg) });
  const parse = (r) => { try { return JSON.parse(r.body); } catch { return {}; } };
  const meta = { "io.modelcontextprotocol/protocolVersion": MODERN, "io.modelcontextprotocol/clientInfo": { name: "probe", version: "1.0" }, "io.modelcontextprotocol/clientCapabilities": {} };
  const modern = (id, method, params = {}, extra = {}) => {
    const name = params.name || params.uri;
    return post({ jsonrpc: "2.0", id, method, params: { ...params, _meta: meta } }, { "MCP-Protocol-Version": MODERN, "Mcp-Method": method, ...(name ? { "Mcp-Name": name } : {}), ...extra });
  };
  const text = (r) => (parse(r).result?.content || []).map((c) => c.text).join("|").trim().replaceAll("\n", " / ");
  say(`no token ${(await request({ method: "POST", path: "/mcp", body: "{}" })).status}`);
  say(`foreign origin ${(await post({ jsonrpc: "2.0", id: 1, method: "ping" }, { Origin: "https://evil.example" })).status}`);
  say(`get ${(await request({ path: "/mcp", headers: auth })).status}`);
  // (an earlier revision: a session)
  const init = await post({ jsonrpc: "2.0", id: 1, method: "initialize", params: { protocolVersion: "2025-06-18", capabilities: {}, clientInfo: { name: "probe-legacy", version: "0.9" } } });
  const sid = init.headers["mcp-session-id"] || "";
  const ir = parse(init).result || {};
  say(`initialize ${init.status} ${ir.protocolVersion} ${ir.serverInfo?.name} session ${sid.length}`);
  const inSession = (msg) => post(msg, { "Mcp-Session-Id": sid, "MCP-Protocol-Version": "2025-06-18" });
  say(`initialized ${(await inSession({ jsonrpc: "2.0", method: "notifications/initialized" })).status}`);
  const list = parse(await inSession({ jsonrpc: "2.0", id: 2, method: "tools/list" })).result?.tools || [];
  say(`tools ${list.map((t) => t.name).join(",")} read-only ${list.filter((t) => t.annotations?.readOnlyHint).length}`);
  say(`add ${text(await inSession({ jsonrpc: "2.0", id: 3, method: "tools/call", params: { name: "add_note", arguments: { Text: "Water the plants" } } }))}`);
  const wrong = parse(await inSession({ jsonrpc: "2.0", id: 4, method: "tools/call", params: { name: "add_note", arguments: {} } })).result || {};
  say(`wrong call error ${wrong.isError} ${(wrong.content || [])[0]?.text}`);
  say(`unknown tool ${parse(await inSession({ jsonrpc: "2.0", id: 5, method: "tools/call", params: { name: "nope" } })).error?.code}`);
  say(`end session ${(await request({ method: "DELETE", path: "/mcp", headers: { ...auth, "Mcp-Session-Id": sid } })).status}`);
  say(`ended session ${(await inSession({ jsonrpc: "2.0", id: 6, method: "tools/list" })).status}`);
  // (the current revision: no session, each request says who and what)
  const d = parse(await modern(10, "server/discover")).result || {};
  say(`discover ${d.supportedVersions?.join(",")} ${d._meta?.["io.modelcontextprotocol/serverInfo"]?.name} ${d.resultType}`);
  say(`list ${text(await modern(11, "tools/call", { name: "list_notes", arguments: {} }))}`);
  say(`find ${text(await modern(12, "tools/call", { name: "find_notes", arguments: { Word: "PLANTS" } }))}`);
  const mm = await modern(13, "tools/call", { name: "list_notes", arguments: {} }, { "Mcp-Name": "find_notes" });
  say(`header mismatch ${mm.status} ${parse(mm).error?.code}`);
  const old = await post({ jsonrpc: "2.0", id: 14, method: "tools/list", params: { _meta: { ...meta, "io.modelcontextprotocol/protocolVersion": "1900-01-01" } } }, { "MCP-Protocol-Version": "1900-01-01", "Mcp-Method": "tools/list" });
  say(`unsupported ${old.status} ${parse(old).error?.code} ${parse(old).error?.data?.supported?.[0]}`);
  const nm = await modern(15, "nothing/here");
  say(`no method ${nm.status} ${parse(nm).error?.code}`);
  say(`no session ${(await post({ jsonrpc: "2.0", id: 16, method: "tools/list" })).status}`);
}
writeFileSync(`${outfile}.done`, "");
