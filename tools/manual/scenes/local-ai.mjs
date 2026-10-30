// The manual's "AI on this computer" (docs/manual/local-ai.md): Set Up RAI's
// page of the models on this computer, against the tests' mock playing a
// local AI app (tests/ai_mock_server.mjs, /s/local: made-up models with
// made-up names, no network, nothing downloaded or run) on a made-up
// computer of 16 GB with 12 free (ai:setupfacts) — never this computer's
// servers or memory; and the RSystemInfo example.

import { spawn } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

export const topic = "local-ai";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const mock = spawn(process.execPath, [join(ROOT, "tests", "ai_mock_server.mjs"), "0"], {
  stdio: ["ignore", "pipe", "inherit"],
  env: { ...process.env, AI_MOCK_LOCAL_NAMES: "example-small,example-chat,example-huge,example-embed" },
});
const MOCK = await new Promise((resolve) => mock.stdout.once("data", (d) => resolve(String(d).trim())));
process.on("exit", () => mock.kill());
const LOCAL = MOCK.replace(/\/v1$/, "") + "/s/local/v1";
const ENV = { RAPIDR_AI_KEYSTORE: "memory", ANTHROPIC_API_KEY: "", OPENAI_API_KEY: "", DEEPSEEK_API_KEY: "" };
const SETUP = `ai:setupprobes=${LOCAL}|http://127.0.0.1:9/v1,ai:setupfacts=total=16384&free=12288,ai:setup`;

const idle = (n) => Array(n).fill("aionnote.onclick").join(",");
const click = (c, x, y) => `${c}.__mousedown_${x}_${y},${c}.__mouseup_${x}_${y}`;
const NEXT = click("aisunextbtn", 30, 14);

export const scenes = [
  {
    // Set Up RAI's first page: free, on this computer — what was found
    name: "setup-found",
    open: "examples/gui/hello_form.rr",
    do: SETUP,
    delay: 4,
    window: 2,
    env: { ...ENV, RAPIDR_TEST_EVENTS: idle(8) },
  },
  {
    // ... Next: the models, the best match first, each with its chip
    name: "setup-models",
    open: "examples/gui/hello_form.rr",
    do: SETUP,
    delay: 4,
    window: 2,
    env: { ...ENV, RAPIDR_TEST_EVENTS: [idle(8), NEXT, idle(6)].join(",") },
  },
  {
    // ... Check My Models: each model tried here, the best match said
    name: "setup-checked",
    open: "examples/gui/hello_form.rr",
    do: SETUP,
    delay: 4,
    window: 2,
    env: { ...ENV, RAPIDR_TEST_EVENTS: [idle(8), NEXT, idle(4), click("aisucheckbtn", 30, 14), "aisucheckbtn.__until_Caption_&Check My Models", idle(4)].join(",") },
  },
  {
    // ... the same in the dark look
    name: "setup-checked-dark",
    open: "examples/gui/hello_form.rr",
    do: SETUP,
    delay: 4,
    window: 2,
    theme: "rapidr-dark",
    env: { ...ENV, RAPIDR_TEST_EVENTS: [idle(8), NEXT, idle(4), click("aisucheckbtn", 30, 14), "aisucheckbtn.__until_Caption_&Check My Models", idle(4)].join(",") },
  },
  {
    // ... none found: what a local AI app is, another address
    name: "setup-none",
    open: "examples/gui/hello_form.rr",
    do: "ai:setupprobes=http://127.0.0.1:9/v1,ai:setup",
    delay: 4,
    window: 2,
    env: { ...ENV, RAPIDR_TEST_EVENTS: [idle(8), NEXT, idle(4)].join(",") },
  },
  {
    // ... a model's licence, as its local AI app reports it
    name: "setup-licence",
    open: "examples/gui/hello_form.rr",
    do: SETUP,
    delay: 4,
    window: 3,
    env: { ...ENV, RAPIDR_TEST_EVENTS: [idle(8), NEXT, idle(4), click("aisulicbtn", 30, 14), idle(4)].join(",") },
  },
  // (RSystemInfo: the example, on the computer that makes the picture)
  { name: "system-info", program: "examples/system_info.rr", delay: 2 },
];
