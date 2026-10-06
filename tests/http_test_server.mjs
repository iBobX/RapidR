// The GUI tests' own HTTP server (QDOWNLOAD's fixture, tests/fixtures/
// download.bas; the network examples, tests/examples_run.mjs): files of the
// repository's tests/ and examples/ by their path, sent
// slowly (four parts, 100 ms apart) so a program's timers tick while it
// waits; a file that isn't there is a 404. Local only (127.0.0.1, a free
// port), with CORS for the browser's page. The runners give programs its
// address as RAPIDR_TEST_HTTP (host:port).
//
// It runs in a worker thread of its own: the desktop runner waits for each
// program with execFileSync, which holds the main thread's event loop.

import { createServer } from "node:http";
import { readFileSync, existsSync, statSync } from "node:fs";
import { dirname, join, normalize, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { Worker, isMainThread, parentPort } from "node:worker_threads";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");

function serve() {
  const server = createServer((req, res) => {
    const headers = { "Access-Control-Allow-Origin": "*", "Access-Control-Expose-Headers": "Content-Length" };
    const path = normalize(decodeURIComponent((req.url || "/").split("?")[0])).replace(/^([/\\])+/, "");
    const file = join(ROOT, path);
    if (!/^(tests|examples)[/\\]/.test(path) || !existsSync(file) || !statSync(file).isFile()) {
      res.writeHead(404, { ...headers, "Content-Type": "text/plain", "Content-Length": "9" });
      res.end("not found");
      return;
    }
    const body = readFileSync(file);
    res.writeHead(200, { ...headers, "Content-Type": "application/octet-stream", "Content-Length": String(body.length) });
    const parts = 4, size = Math.ceil(body.length / parts);
    let i = 0;
    const next = () => {
      if (i >= parts) return res.end();
      res.write(body.subarray(i * size, (i + 1) * size));
      i++;
      setTimeout(next, 100);
    };
    next();
  });
  server.listen(0, "127.0.0.1", () => parentPort.postMessage(server.address().port));
}

if (!isMainThread) serve();

/// Starts the server in its worker: its "127.0.0.1:port".
export async function startHttpServer() {
  const worker = new Worker(fileURLToPath(import.meta.url));
  const port = await new Promise((ok) => worker.once("message", ok));
  // (it never keeps a runner alive)
  worker.unref();
  return { address: `127.0.0.1:${port}`, close: () => worker.terminate() };
}
