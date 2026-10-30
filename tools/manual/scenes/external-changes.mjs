// The manual's "When files change outside Studio" (docs/manual/
// studio-projects.md; lane EXTERNAL-CHANGES): a copy of a small project
// (tests/fixtures/studio_external) open in Studio while another process —
// tests/ext_writer.mjs, standing in for another AI agent, an editor or git —
// changes its files: main.rr loaded again (the status bar says so), the
// question when it has unsaved changes, Compare, the question before Save,
// twelve files at once, a file deleted.
//
// Each scene works in its own folder, which is also the home folder ($HOME)
// Studio sees: the paths on screen are short (~/Projects/Outside).
import { spawn } from "node:child_process";
import { cpSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

export const topic = "studio-projects";

const FIXTURE = "tests/fixtures/studio_external";
const main = (root) => readFileSync(join(root, FIXTURE, "main.rr"), "utf8");

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
    return { open: "Projects/Outside/Outside.rrproj", cwd: work, env: { HOME: work, RAPIDR_TEST_EXT: meet } };
  };
}

const agent = outside((root) => ({ write: { "main.rr": main(root).replace('PRINT "Done."', 'PRINT "Changed by an agent."') } }));
const twelve = outside((root) => ({
  write: Object.fromEntries(["main.rr", "greet.rr", "window.rr", ...Array.from({ length: 9 }, (_, i) => `part${i + 1}.rr`)].map((f) => [f, readFileSync(join(root, FIXTURE, f), "utf8") + "' (rewritten by an agent)\n"])),
}));
const gone = outside(() => ({ remove: ["greet.rr"] }));

const CHANGE = "ext:go=1,ext:await=1,ext:check,wait";
const MINE = 'code:PRINT "Done."=>PRINT "Mine.",line:27';

export const scenes = [
  // main.rr changed outside, nothing unsaved here: loaded again, the status bar says so
  { name: "outside-reloaded", setup: agent, do: `line:27,${CHANGE}`, delay: 6 },
  // typed in, then changed outside: the question above the editor
  { name: "outside-question", setup: agent, do: `${MINE},${CHANGE}`, delay: 6, crop: [240, 68, 800, 240] },
  // Compare: the disk's version beside yours
  { name: "outside-compare", setup: agent, do: `${MINE},${CHANGE},file.compareDisk`, delay: 6, crop: [240, 68, 800, 500] },
  // Keep Mine, then Save: asked first, Cancel the default
  { name: "outside-save-anyway", setup: agent, do: `${MINE},${CHANGE},file.keepMine,wait,file.save`, delay: 7, window: 2 },
  // twelve files at once: one note
  { name: "outside-many", setup: twelve, do: `ext:openall,wait,${CHANGE}`, delay: 8, crop: [240, 68, 800, 200] },
  // a file deleted outside: its tab marked, its text kept
  { name: "outside-deleted", setup: gone, do: `ext:open=greet.rr,wait,${CHANGE}`, delay: 6, crop: [240, 68, 800, 240] },
];
