// docs/manual/ai-assistant.md's "Coach mode" and docs/manual/for-teachers.md's
// "Keep RAI in Coach mode" pictures (ide/aicoach.inc): the dial, the hint
// ladder climbed as a student climbs it, Help With This Error, and the
// teacher's lock — against the tests' mock provider (tests/ai_mock_server.mjs:
// in Coach it answers with the hint Studio asks for; no key, no network).
import { spawn } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

export const topic = "ai";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const mock = spawn(process.execPath, [join(ROOT, "tests", "ai_mock_server.mjs"), "0"], {
  stdio: ["ignore", "pipe", "inherit"],
  env: { ...process.env, AI_MOCK_MODELS: "example-mini:Example Mini,example-pro:Example Pro" },
});
const MOCK = await new Promise((resolve) => mock.stdout.once("data", (d) => resolve(String(d).trim())));
process.on("exit", () => mock.kill());
const MOCK_ROOT = MOCK.replace(/\/v1$/, "");
const ENV = { RAPIDR_AI_KEYSTORE: "memory", RAPIDR_AI_TEST_SERVER: MOCK_ROOT, ANTHROPIC_API_KEY: "", OPENAI_API_KEY: "", DEEPSEEK_API_KEY: "", ANTHROPIC_WORKSPACE_ID: "" };
// (an assistant set up as a user sets it: the provider played by the mock, a
// made-up key, the models fetched, the fast one picked)
const READY = "ai:on,ai:provider=anthropic,ai:key=sk-ant-EXAMPLE-not-a-real-key-0000,ai:fetch,wait,wait,ai:model=example-mini,ai:hidepane=outlinetree";
const OPEN = "examples/gui/hello_form.rr";
const PANE = [640, 30, 640, 720];
const ASK = "ai:ask=how do I count the clicks on my button,ai:wait,wait";
const NEXT = "ai:coach:next,ai:wait,wait";

export const scenes = [
  {
    // the dial, Coach chosen: the empty chat says how Coach works
    name: "coach-dial",
    open: OPEN,
    do: `${READY},ai.assistant,ai:coach:mode=coach,wait`,
    delay: 6,
    env: ENV,
    crop: [896, 30, 384, 330],
  },
  {
    // hint 1 of 4: a nudge — a question, no code
    name: "coach-hint-1",
    open: OPEN,
    do: `${READY},ai.assistant,ai:coach:mode=coach,${ASK}`,
    delay: 7,
    env: ENV,
    crop: PANE,
  },
  {
    // hint 3 of 4: the shape of the code, blanks to fill
    name: "coach-hint-3",
    open: OPEN,
    do: `${READY},ai.assistant,ai:coach:mode=coach,${ASK},${NEXT},${NEXT}`,
    delay: 9,
    env: ENV,
    crop: PANE,
  },
  {
    // hint 4 of 4: the code shown as text — Copy, never Insert; no Next Hint
    name: "coach-hint-4",
    open: OPEN,
    do: `${READY},ai.assistant,ai:coach:mode=coach,${ASK},${NEXT},${NEXT},${NEXT}`,
    delay: 10,
    env: ENV,
    crop: PANE,
  },
  {
    // a program that doesn't compile: Help With This Error, its first hint
    name: "coach-error",
    open: "tests/fixtures/studio_coach/main.rr",
    do: `${READY},ai.assistant,ai:coach:mode=coach,run.start,wait,ai:coach:error,ai:wait,wait`,
    delay: 9,
    env: ENV,
  },
  {
    // School Lock keeps RAI in Coach: the dial with the teacher's note
    topic: "for-teachers",
    name: "coach-lock",
    open: OPEN,
    do: `${READY},ai.assistant,lock:set=maestra1|coach,wait`,
    delay: 9,
    env: ENV,
    crop: [896, 30, 384, 330],
  },
  {
    // Tools > School Lock…: AI in Studio unticked, Keep RAI in Coach mode ticked
    topic: "for-teachers",
    name: "turn-on-coach",
    open: OPEN,
    do: "tools.schoolLock",
    delay: 5,
    window: 2,
    env: { RAPIDR_TEST_EVENTS: "lockaibox.__key_32,lockcoachbox.__key_32" },
  },
];
