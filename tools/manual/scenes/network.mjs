// docs/manual/network.md's screenshots: the local API example
// (examples/network/local_api.rr, RHttpServer) on 127.0.0.1, after the
// tests' client (tests/http_client_probe.mjs) has called it — refused
// requests, /hello, /echo, three Server-Sent Events. Its own token, a port of
// its own; nothing leaves the computer.
import { spawn } from "node:child_process";
import { join } from "node:path";

export const topic = "network";

const PORT = "47190";
const TOKEN = "manual-token";

export const scenes = [
  {
    name: "local-api",
    program: "examples/network/local_api.rr",
    args: [PORT, TOKEN],
    delay: 5,
    setup(work, root) {
      // (the client, beside the program: it waits for the server to listen)
      spawn(process.execPath, [join(root, "tests/http_client_probe.mjs"), "api", PORT, join(work, "client.txt"), TOKEN], { stdio: "ignore", detached: true }).unref();
      return {};
    },
  },
];
