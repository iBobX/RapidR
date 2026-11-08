// docs/manual/ai-apps.md's pictures: RapidR Studio with AI apps switched on
// (the test step mcp:on, with RAPIDR_TEST_MCP_TOKEN standing in for the
// keychain's token), the fixture project tests/fixtures/studio_mcp open, and
// an AI app — the tests' client, tests/http_client_probe.mjs — calling a few
// tools: the AI Apps pane listing them under Allow everything, a change
// waiting for the user under Ask before changes, Settings > AI Apps, the
// question the first time the switch is ticked (the test step mcp:consent). A port
// and a token of their own; nothing leaves the computer, nothing is saved.
import { spawn } from "node:child_process";
import { join } from "node:path";

export const topic = "ai-apps";

const TOKEN = "manual-mcp-token-0123456789abcdef";
// (a browser that offers tools to AI agents: the WebMCP draft's API behind
// the browser's own feature flag)
const WEBMCP = { args: ["--enable-features=WebMCP,WebMCPTesting"] };
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
    do: "mcp:on=47914,wait,mcp:settings",
    delay: 6,
    window: 2,
    env: { RAPIDR_TEST_MCP_TOKEN: TOKEN },
  },  // (Studio for the web: the same page, its one row in place of the address
  // and the token — in a browser that offers tools to AI agents (the WebMCP
  // draft's API, turned on by the browser's flag: RAPIDR_WEB_CHROME, a
  // throwaway profile) and in one that doesn't, where the switch is greyed)
  // (dark and high contrast: MANUAL_IMAGES=<a folder> and these themes, for
  // the design check; the manual shows the light one)
  ...["rapidr-light"].map((theme, i) => ({
    name: ["settings-web"][i],
    host: "web",
    testEnv: { RAPIDR_TEST_MCP_TOKEN: TOKEN },
    launch: WEBMCP,
    open: "tests/fixtures/studio_mcp/Hello.rrproj",
    webFiles: ["tests/fixtures/studio_mcp/Hello.rrproj", "tests/fixtures/studio_mcp/main.rr"],
    do: "mcp:on,wait,mcp:settings",
    delay: 8,
    theme,
    env: { RAPIDR_TEST_MCP_TOKEN: TOKEN },
  })),
  {
    name: "settings-web-unsupported",
    host: "web",
    testEnv: { RAPIDR_TEST_MCP_TOKEN: TOKEN },
    open: "tests/fixtures/studio_mcp/Hello.rrproj",
    webFiles: ["tests/fixtures/studio_mcp/Hello.rrproj", "tests/fixtures/studio_mcp/main.rr"],
    do: "wait,mcp:settings",
    delay: 8,
    env: { RAPIDR_TEST_MCP_TOKEN: TOKEN },
  },
  {
    // (the first call after the page loads: asked once, in the pane)
    name: "ask-web",
    host: "web",
    testEnv: { RAPIDR_TEST_MCP_TOKEN: TOKEN },
    launch: WEBMCP,
    open: "tests/fixtures/studio_mcp/Hello.rrproj",
    webFiles: ["tests/fixtures/studio_mcp/Hello.rrproj", "tests/fixtures/studio_mcp/main.rr"],
    do: "mcp:on",
    delay: 8,
    env: { RAPIDR_TEST_MCP_TOKEN: TOKEN },
    web: async (page) => {
      await page.evaluate(async () => {
        const tool = (await document.modelContext.getTools()).find((t) => t.name === "list_files");
        document.modelContext.executeTool(tool, {});
      });
      await page.waitForTimeout(2500);
    },
  },
  {
    // (the first time the switch is ticked: the question, Cancel the default)
    name: "consent",
    open: "tests/fixtures/studio_mcp/Hello.rrproj",
    do: "mcp:consent",
    delay: 5,
    window: 2,
    env: { RAPIDR_TEST_MCP_TOKEN: TOKEN },
  },
];
