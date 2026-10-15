// docs/manual/ai-assistant.md's screenshots (AI-PROVIDERS): RapidR Studio's
// assistant and the RAI component, against the tests' mock provider
// (tests/ai_mock_server.mjs: scripted answers, no key, no network, no
// cost) — its models made-up names, the key a made-up one kept in memory
// (RAPIDR_AI_KEYSTORE=memory: never the keychain), the machine's own
// provider variables blanked, so no real key or its last characters can
// reach a picture.

import { spawn } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const mock = spawn(process.execPath, [join(ROOT, "tests", "ai_mock_server.mjs"), "0"], {
  stdio: ["ignore", "pipe", "inherit"],
  env: { ...process.env, AI_MOCK_MODELS: "example-mini:Example Mini,example-pro:Example Pro" },
});
const MOCK = await new Promise((resolve) => mock.stdout.once("data", (d) => resolve(String(d).trim())));
process.on("exit", () => mock.kill());

const HOME_OF_SCENE = join(ROOT, "tests", "results", "manual-shots", "ai-home");
const ENV = { RAPIDR_AI_KEYSTORE: "memory", ANTHROPIC_API_KEY: "", OPENAI_API_KEY: "", DEEPSEEK_API_KEY: "", ANTHROPIC_WORKSPACE_ID: "" };
// (an assistant set up as a user sets it: Anthropic's format, the mock at
// its address, a made-up key, the models fetched, the fast one picked, the
// prices the user typed for the meter)
const READY = `ai:on,ai:provider=anthropic,ai:base=${MOCK},ai:key=sk-ant-EXAMPLE-not-a-real-key-0000,ai:fetch,wait,wait,ai:model=example-mini,ai:prices=1/5`;

export const topic = "ai";
export const scenes = [
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
    // "Build me …": the new program previewed before anything is written
    name: "new-program",
    open: "examples/gui/hello_form.rr",
    do: `${READY},ai:ask=build me a hello-form program,wait,wait,wait,wait,ai:review,wait`,
    delay: 8,
    window: 2,
    env: ENV,
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
];
