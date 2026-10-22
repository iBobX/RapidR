// docs/manual/ai-assistant.md's screenshots (AI-PROVIDERS): RapidR Studio's
// assistant and the RAI component, against the tests' mock provider
// (tests/ai_mock_server.mjs: scripted answers, no key, no network, no
// cost) — its models made-up names, the key a made-up one kept in memory
// (RAPIDR_AI_KEYSTORE=memory: never the keychain), the machine's own
// provider variables blanked, so no real key or its last characters can
// reach a picture.

import { spawn } from "node:child_process";
import { cpSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

// (AI-UX) RapidR's home as an install has it — its examples (the recipes)
// and Studio's pictures, a release.toml —, so a new program goes where an
// installed Studio puts it: Documents/RapidR/Projects, here the scene's own
// "Documents" (a short path on screen; never the user's)
function installed(work, root) {
  const home = join(work, "install");
  mkdirSync(join(home, "ide"), { recursive: true });
  cpSync(join(root, "examples"), join(home, "examples"), { recursive: true, filter: (src) => !/\.tmp\.[^/\\]*$|big(_\d+)?\.csv$/.test(src) });
  cpSync(join(root, "ide", "assets"), join(home, "ide", "assets"), { recursive: true });
  writeFileSync(join(home, "release.toml"), 'version = "0.0.0"\nrust = "1.0.0"\nkind = "sdk"\n');
  return { home, env: { RAPIDR_DOCUMENTS: "Documents" } };
}

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const mock = spawn(process.execPath, [join(ROOT, "tests", "ai_mock_server.mjs"), "0"], {
  stdio: ["ignore", "pipe", "inherit"],
  env: { ...process.env, AI_MOCK_MODELS: "example-mini:Example Mini,example-pro:Example Pro" },
});
const MOCK = await new Promise((resolve) => mock.stdout.once("data", (d) => resolve(String(d).trim())));
process.on("exit", () => mock.kill());

const HOME_OF_SCENE = join(ROOT, "tests", "results", "manual-shots", "ai-home");
// (Anthropic, OpenAI and DeepSeek played by the mock too:
// RAPIDR_AI_TEST_SERVER, a loopback address only)
const MOCK_ROOT = MOCK.replace(/\/v1$/, "");
const ENV = { RAPIDR_AI_KEYSTORE: "memory", RAPIDR_AI_TEST_SERVER: MOCK_ROOT, ANTHROPIC_API_KEY: "", OPENAI_API_KEY: "", DEEPSEEK_API_KEY: "", ANTHROPIC_WORKSPACE_ID: "" };
// (an assistant set up as a user sets it: Anthropic (the mock), a made-up
// key, the models fetched, the fast one picked, the prices the user typed
// for the meter)
const READY = `ai:on,ai:provider=anthropic,ai:key=sk-ant-EXAMPLE-not-a-real-key-0000,ai:fetch,wait,wait,ai:model=example-mini,ai:prices=1/5`;
// (the settings dialog used as a person uses it: its AI tab clicked, a key
// typed key by key, Fetch Models pressed; the mock's answer decided by the
// made-up key: "bad" refused, "org" an organisation's)
const typeInto = (comp, text) => [...text].map((c) => (/[a-z]/.test(c) ? c.toUpperCase().charCodeAt(0) : /[0-9]/.test(c) ? c.charCodeAt(0) : 189)).map((k) => `${comp}.__key_${k}`).join(",");
const idle = (n) => Array(n).fill("aionnote.onclick").join(",");
const AI_TAB = "settingstabs.__mousedown_75_12,settingstabs.__mouseup_75_12";
const FETCH = "aifetchbtn.__mousedown_60_14,aifetchbtn.__mouseup_60_14";

export const topic = "ai";
export const scenes = [
  {
    // Tools > Settings > AI: a key the provider refused — what it said,
    // whole, under the list; Fetch Models ready to be pressed again
    name: "settings-refused-key",
    open: "examples/gui/hello_form.rr",
    do: "view.settings",
    delay: 4,
    window: 2,
    env: { ...ENV, RAPIDR_TEST_EVENTS: [AI_TAB, "aiprovbox.__item_1", typeInto("aikeyedit", "example-bad-key-0000"), FETCH, idle(40)].join(",") },
  },
  {
    // ... an organisation's Anthropic key: the workspace Anthropic wants
    name: "settings-workspace",
    open: "examples/gui/hello_form.rr",
    do: "view.settings",
    delay: 4,
    window: 2,
    env: { ...ENV, RAPIDR_TEST_EVENTS: [AI_TAB, "aiprovbox.__item_0", typeInto("aikeyedit", "example-org-key-0000"), FETCH, idle(40)].join(",") },
  },
  {
    // Tools > Settings > AI
    name: "settings",
    open: "examples/gui/hello_form.rr",
    do: `${READY},ai:settings,wait`,
    delay: 6,
    window: 2,
    env: ENV,
  },
  {
    // The same in Studio on the web: the key for the tab, or remembered on
    // this device (with what that means)
    name: "settings-web",
    host: "web",
    open: "examples/gui/hello_form.rr",
    do: `${READY},ai:settings,wait`,
    delay: 8,
    viewport: { width: 1280, height: 800 },
    crop: [350, 47, 580, 693],
  },
  {
    // The Assistant panel: a question, the tools it used, the token meter
    name: "panel",
    open: "examples/gui/hello_form.rr",
    do: `${READY},ai:ask=describe the form,wait,wait,wait,wait`,
    delay: 8,
    env: ENV,
    crop: [640, 30, 640, 720],
  },
  {
    // A change proposed as a diff: the review
    name: "review",
    open: "examples/gui/hello_form.rr",
    do: `${READY},ai:ask=rename the window,wait,wait,wait,wait,ai:review,wait`,
    delay: 8,
    window: 2,
    env: ENV,
  },
  {
    // "Build me …" with Review first on: the new program previewed before
    // anything is written
    name: "new-program",
    open: "examples/gui/hello_form.rr",
    do: `${READY},ai:reviewfirst=1,ai:ask=build me a hello-form program,wait,wait,wait,wait,ai:review,wait`,
    delay: 8,
    window: 2,
    env: ENV,
  },
  {
    // (AI-UX) "Build a scientific calculator": the program opens in Studio
    // at once, its form in the designer, the banner with Run It on top
    name: "banner",
    studio: { do: `${READY},ai:ask=Build a scientific calculator with graphics please.,wait,ai:wait,wait,wait,wait`, delay: 9 },
    setup: (work, root) => installed(work, root),
    env: ENV,
    crop: [0, 0, 1280, 400],
  },
  {
    // ... in the dark look
    name: "banner-dark",
    theme: "rapidr-dark",
    studio: { do: `${READY},ai:ask=Build a scientific calculator with graphics please.,wait,ai:wait,wait,wait,wait`, delay: 9 },
    setup: (work, root) => installed(work, root),
    env: ENV,
    crop: [0, 0, 1280, 400],
  },
  {
    // ... the chat: the cards Studio writes (the recipe it started from,
    // the program created, with Open and Undo), then the model's short reply
    name: "chat-cards",
    studio: { do: `${READY},ai:ask=Build a scientific calculator with graphics please.,wait,ai:wait,wait,wait,wait`, delay: 9 },
    setup: (work, root) => installed(work, root),
    env: ENV,
    crop: [896, 120, 384, 480],
  },
  {
    // (R3) A message sent while the assistant works: queued above the box,
    // × to take it back; it goes when the answer ends
    name: "queued",
    open: "examples/gui/hello_form.rr",
    do: `${READY},ai:ask=slowly rename the window,wait,wait,ai.assistant,type:and add a Clear button,key:Enter,wait`,
    delay: 6,
    env: ENV,
    crop: [640, 30, 640, 720],
  },
  {
    // (R11) A change cut off at its end (`END FUNCTI`): refused, a card
    // says so, nothing to review (Studio's three-file test project)
    name: "refused",
    open: "tests/fixtures/studio_ai_sites/main.rr",
    do: `${READY},ai:ask=finish the count,wait,ai:wait,wait`,
    delay: 7,
    env: ENV,
    crop: [640, 30, 640, 720],
  },
  {
    // What was sent: the last request, the key never shown
    name: "what-was-sent",
    open: "examples/gui/hello_form.rr",
    do: `${READY},ai:ask=hello,wait,wait,ai:sent,wait`,
    delay: 7,
    window: 2,
    // (the audit log where a Mac keeps it, under a home of the scene's own:
    // the path a user sees, not this machine's)
    env: { ...ENV, HOME: HOME_OF_SCENE, RAPIDR_AI_AUDIT: join(HOME_OF_SCENE, "Library", "Application Support", "RapidR", "ai-audit.jsonl") },
  },
  {
    // An RAI on a form, live in the designer: the provider's own models in
    // the Model row once it has a key (kept in the keychain, never in the code)
    name: "designer-rai",
    open: "tests/fixtures/studio_ai/rai_form.rr",
    do: `wait,view.sideBySide,pick:AI,prop:Provider=anthropic,prop:BaseURL=${MOCK},prop:APIKey=sk-ant-EXAMPLE-not-a-real-key-0000,wait,wait,wait,row:Model,prop:Model=example-mini,wait,row:Model`,
    delay: 6,
    env: ENV,
    crop: [244, 60, 1036, 512],
  },
  {
    // examples/ai/rai_models.rr: a provider (here a local server), its
    // models, a question, the answer streamed in
    name: "rai-models-example",
    program: "examples/ai/rai_models.rr",
    args: [MOCK],
    delay: 3,
    env: {
      ...ENV,
      RAPIDR_TEST_EVENTS: `listbtn.onclick,${Array(4).fill("info.onclick").join(",")},modelbox.__item_0,askbtn.onclick,${Array(6).fill("info.onclick").join(",")}`,
    },
  },
  {
    // ... with Anthropic: its Workspace ID row (an organisation's key wants
    // one), the models, a question answered
    name: "rai-models-workspace",
    program: "examples/ai/rai_models.rr",
    delay: 3,
    env: {
      ...ENV,
      RAPIDR_TEST_EVENTS: `providerbox.__item_0,${typeInto("keyedit", "example-org-key-0000")},${typeInto("wsedit", "wrkspc-example")},listbtn.onclick,${Array(6).fill("info.onclick").join(",")},modelbox.__item_0,askbtn.onclick,${Array(8).fill("info.onclick").join(",")}`,
    },
  },
];
