// docs/manual/app-eyes-and-hands.md's pictures: the program's own eyes and
// hands — examples/gui/snapshot.rr checking its own layout after its
// button was narrowed (RWindowCapture), examples/gui/guided_tour.rr while its
// "Show me" tour types (the virtual pointer over the box) and when it is
// done (RAppHands); and RAI asking before it uses the program Studio runs
// (examples/gui/hello_form.rr, the tests' mock provider: no key, no
// network, no cost). Programs of RapidR's own, nothing saved outside the
// scene's folder.

import { spawn } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const mock = spawn(process.execPath, [join(ROOT, "tests", "ai_mock_server.mjs"), "0"], { stdio: ["ignore", "pipe", "inherit"] });
const MOCK = await new Promise((resolve) => mock.stdout.once("data", (d) => resolve(String(d).trim())));
process.on("exit", () => mock.kill());
const AI_ENV = { RAPIDR_AI_KEYSTORE: "memory", ANTHROPIC_API_KEY: "", OPENAI_API_KEY: "", DEEPSEEK_API_KEY: "", ANTHROPIC_WORKSPACE_ID: "" };

export const topic = "app-eyes";

export const scenes = [
  {
    name: "snapshot-check",
    program: "examples/gui/snapshot.rr",
    delay: 1,
    env: { RAPIDR_TEST_SYSTEM_LOOK: "light", RAPIDR_TEST_EVENTS: "sizebutton.onclick,checkbutton.onclick" },
  },
  {
    name: "tour-pointer",
    program: "examples/gui/guided_tour.rr",
    delay: 1,
    env: { RAPIDR_TEST_SYSTEM_LOOK: "light", RAPIDR_TEST_EVENTS: "tourbutton.onclick,form.__wait_1700" },
  },
  {
    name: "tour-done",
    program: "examples/gui/guided_tour.rr",
    delay: 1,
    env: { RAPIDR_TEST_SYSTEM_LOOK: "light", RAPIDR_TEST_EVENTS: "tourbutton.onclick,form.__wait_7000" },
  },
  {
    // RAI asked to use the running program: its card waits for the user
    // (Allow for this run / Don't allow) before it looks or clicks
    name: "rai-asks",
    open: "examples/gui/hello_form.rr",
    do: `ai:on,ai:provider=compatible,ai:base=${MOCK},ai:model=mock-small,run.start,wait,wait,wait,wait,ai:ask=greet ada in my running program,wait,wait`,
    delay: 9,
    env: { ...AI_ENV, RAPIDR_TEST_PROGRAM_EVENTS: "form.__wait_60000" },
    crop: [640, 30, 640, 720],
  },
];
