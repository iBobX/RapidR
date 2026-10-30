// docs/manual/for-teachers.md's pictures: School Lock (ide/schoollock.inc) —
// turned on with a password typed twice, RAI's card and the status bar's
// chip while it locks, Unlock… after a wrong password and during the
// minute's wait, Settings > AI and AI Apps read-only, and a lock set by the
// computer's policy (the tests' file, tests/fixtures/school_lock: never the
// real one). The password is a test's ("maestra1"); nothing leaves the
// computer.
import { join } from "node:path";

export const topic = "for-teachers";

// (keys typed into a component, as tests/studio_flows.mjs's typeInto)
const typeInto = (comp, text) => [...text].map((c) => (/[a-z]/.test(c) ? String(c.toUpperCase().charCodeAt(0)) : /[A-Z]/.test(c) ? `${c.charCodeAt(0)}_256` : String(c.charCodeAt(0)))).map((k) => `${comp}.__key_${k}`).join(",");
const OPEN = "examples/gui/hello_form.rr";
const FIVE_WRONG = ["a1b2c3", "a1b2c4", "a1b2c5", "a1b2c6", "a1b2c7"].map((p) => `lock:unlock=${p}`).join(",");

export const scenes = [
  {
    // Tools > School Lock…: what to lock, the password twice (masked), its strength
    name: "turn-on",
    open: OPEN,
    do: "tools.schoolLock",
    delay: 5,
    window: 2,
    env: { RAPIDR_TEST_EVENTS: [typeInto("lockpwedit", "maestra1"), typeInto("lockpw2edit", "maestra1")].join(",") },
  },
  {
    // RAI's pane while AI is locked: the teacher's card, Unlock…
    name: "card",
    open: OPEN,
    do: "lock:set=maestra1,ai.assistant,wait",
    delay: 6,
    crop: [880, 60, 398, 330],
  },
  {
    // the status bar's chip
    name: "chip",
    open: OPEN,
    do: "lock:set=maestra1",
    delay: 5,
    crop: [540, 742, 738, 27],
  },
  {
    // Unlock…: a wrong password said, the tries left
    name: "unlock",
    open: OPEN,
    do: "lock:set=maestra1,tools.schoolLock",
    delay: 6,
    window: 2,
    env: { RAPIDR_TEST_EVENTS: [typeInto("lockpwedit", "maestra2"), "lockpwedit.__key_13"].join(",") },
  },
  {
    // five wrong passwords: a minute's wait, counted down
    name: "wait",
    open: OPEN,
    do: `lock:set=maestra1,${FIVE_WRONG},tools.schoolLock`,
    delay: 9,
    window: 2,
  },
  {
    // Settings > AI read-only, the note and Unlock…
    name: "settings",
    open: OPEN,
    do: "lock:set=maestra1,wait,ai.settings",
    delay: 6,
    window: 2,
    crop: [0, 0, 660, 330],
  },
  {
    // Settings > AI Apps: the switch off and greyed
    name: "apps-settings",
    open: OPEN,
    do: "lock:set=maestra1,wait,mcp.settings",
    delay: 6,
    window: 2,
    crop: [0, 0, 660, 300],
  },
  {
    // a lock from the computer's policy: unlocked until Studio closes, never turned off here
    name: "policy",
    open: OPEN,
    do: "tools.schoolLock",
    delay: 6,
    window: 2,
    setup: (work, root) => ({ env: { RAPIDR_SCHOOL_POLICY: join(root, "tests/fixtures/school_lock/policy.toml") } }),
  },
];
