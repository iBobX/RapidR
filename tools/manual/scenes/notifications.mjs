// The manual's "Notifications" (docs/manual/notifications.md; lane NOTIFY):
// RapidR Studio's toast (twelve files rewritten at once, named in one), the
// history behind the bell, the question's strip on one line, the document
// tabs that keep their names (seventeen files, the arrows, the list), and
// the example program notify.rr.
//
// Each Studio scene works in its own folder, which is also the home folder
// ($HOME) Studio sees: the paths on screen are short (~/Projects/Outside).
// Toasts and their pictures stand still (RAPIDR_REDUCED_MOTION) and the
// history's times are fixed (RAPIDR_TEST_CLOCK), so the pictures are the same
// every time.
import { spawn } from "node:child_process";
import { cpSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

export const topic = "notifications";

const FIXTURE = "tests/fixtures/studio_external";
const main = (root) => readFileSync(join(root, FIXTURE, "main.rr"), "utf8");
const STILL = { RAPIDR_REDUCED_MOTION: "1", RAPIDR_TEST_CLOCK: "10:42" };

// (the project copied to <scene>/Projects/Outside; the outside writer started,
// waiting for Studio's `ext:go=1` to make its change)
function outside(changes) {
  return (work, root) => {
    const project = join(work, "Projects/Outside");
    cpSync(join(root, FIXTURE), project, { recursive: true });
    const meet = join(work, "meet");
    mkdirSync(meet, { recursive: true });
    writeFileSync(join(work, "external.json"), JSON.stringify([changes(root)]));
    spawn(process.execPath, [join(root, "tests/ext_writer.mjs"), meet, project, join(work, "external.json")], { stdio: "ignore", detached: true }).unref();
    return { open: "Projects/Outside/Outside.rrproj", cwd: work, env: { HOME: work, RAPIDR_TEST_EXT: meet, ...STILL } };
  };
}

const agent = outside((root) => ({ write: { "main.rr": main(root).replace('PRINT "Done."', 'PRINT "Changed by an agent."') } }));
const twelve = outside((root) => ({
  write: Object.fromEntries(["main.rr", "greet.rr", "window.rr", ...Array.from({ length: 9 }, (_, i) => `part${i + 1}.rr`)].map((f) => [f, readFileSync(join(root, FIXTURE, f), "utf8") + "' (rewritten by an agent)\n"])),
}));

// (seventeen files, two named main.rr, in a project of their own)
const shop = (work, root) => {
  cpSync(join(root, "tests/fixtures/studio_tabs"), join(work, "Projects/Shop"), { recursive: true });
  return { open: "Projects/Shop/Shop.rrproj", cwd: work, env: { HOME: work, ...STILL } };
};

const CHANGE = "ext:go=1,ext:await=1,ext:check,wait";
const MINE = 'code:PRINT "Done."=>PRINT "Mine.",line:27';
// (a few messages said one after another: their kinds, their actions, one with a Key)
const SAYS = [
  "notify:say=Version saved: Added the invoice form~success~Show",
  "notify:say=Chat deleted~info~Undo~chat-7",
  "notify:say=The disk is nearly full~warning",
  "notify:say=Could not save report.csv~error~Retry~save-failed",
  "notify:say=Copied~info",
  // (the Undo of the deleted chat can no longer work: greyed, its tooltip says why)
  "notify:retire=chat-7~Another chat was deleted since.",
].join(",");

export const scenes = [
  // twelve files rewritten at once: ONE toast naming three and "+9", the bell counting it
  { name: "toast", setup: twelve, do: `ext:openall,wait,${CHANGE}`, delay: 8, crop: [240, 380, 1040, 420] },
  // typed in, then changed outside: the question as one line on that editor
  { name: "strip", setup: agent, do: `${MINE},${CHANGE}`, delay: 6, crop: [240, 68, 800, 160] },
  // the bell's history: every message with its time and its action again
  { name: "history", setup: agent, do: `${SAYS},wait,notify:history,wait`, delay: 7, crop: [640, 300, 640, 500] },
  // seventeen open files: names that read, the arrows and the list button at the right end
  { name: "tabs", setup: shop, do: "ext:openall,wait,wait", delay: 8, crop: [240, 68, 780, 140] },
  // …and the list of every open tab, searchable
  { name: "tabs-list", setup: shop, do: "ext:openall,wait,wait,notify:tabs,wait,wait", delay: 9, crop: [240, 68, 780, 420] },
  // the example program: a toast with two actions, a second toast, the strip, the bell
  {
    name: "example",
    program: "examples/gui/notify.rr",
    delay: 3,
    env: { ...STILL, RAPIDR_TEST_EVENTS: "savedbtn.__mousedown_10_10,savedbtn.__mouseup_10_10,filesbtn.__mousedown_10_10,filesbtn.__mouseup_10_10,askbtn.__mousedown_10_10,askbtn.__mouseup_10_10" },
  },
];
