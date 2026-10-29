// docs/manual/ai-apps.md's pictures: RapidR Studio with AI apps switched on
// (the test step mcp:on, with RAPIDR_TEST_MCP_TOKEN standing in for the
// keychain's token), the fixture project tests/fixtures/studio_mcp open, and
// an AI app — the tests' client, tests/http_client_probe.mjs — calling a few
// tools: the AI Apps pane listing them under Allow everything, a change
// waiting for the user under Ask before changes, Settings > AI Apps. A port
// and a token of their own; nothing leaves the computer, nothing is saved.
import { spawn } from "node:child_process";
import { join } from "node:path";

export const topic = "ai-apps";

const TOKEN = "manual-mcp-token-0123456789abcdef";
const client = (mode, port) => (work, root) => {
  // (the AI app, beside Studio: it waits for the server to listen)
  spawn(process.execPath, [join(root, "tests/http_client_probe.mjs"), mode, String(port), join(work, "client.txt"), TOKEN], { stdio: "ignore", detached: true }).unref();
  return {};
};

export const scenes = [
  {
    name: "pane",
    open: "tests/fixtures/studio_mcp/Hello.rrproj",
    do: "mcp:mode=all,mcp:on=47391",
    delay: 14,
    env: { RAPIDR_TEST_MCP_TOKEN: TOKEN },
    setup: client("studio-all", 47391),
  },
  {
    name: "ask",
    open: "tests/fixtures/studio_mcp/Hello.rrproj",
    do: "mcp:on=47392",
    delay: 14,
    env: { RAPIDR_TEST_MCP_TOKEN: TOKEN },
    setup: client("studio-ask", 47392),
  },
  {
    name: "settings",
    open: "tests/fixtures/studio_mcp/Hello.rrproj",
    do: "mcp:on=47393,wait,mcp:settings",
    delay: 6,
    window: 2,
    env: { RAPIDR_TEST_MCP_TOKEN: TOKEN },
  },
];
