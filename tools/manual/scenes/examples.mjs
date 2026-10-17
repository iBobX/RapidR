// The manual's pages on the examples where RapidR is installed
// (docs/manual/getting-started.md, docs/manual/data-science-with-rapidr.md;
// EXAMPLES-DATA): RapidR Studio opening an example from a read-only home —
// the Output saying where the user's copy is, with "Copy again from the
// original" —, File > Copy Example Again's question, and million_rows.rr run
// where it can't write.
import { chmodSync, cpSync, mkdirSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

export const topic = "examples";

// (RapidR's home as an install has it: its examples and Studio's pictures,
// read-only — every folder and file)
function readOnly(path) {
  if (statSync(path).isDirectory()) {
    for (const f of readdirSync(path)) readOnly(join(path, f));
    chmodSync(path, 0o555);
  } else chmodSync(path, 0o444);
}
function install(work, root) {
  const home = join(work, "install");
  mkdirSync(join(home, "ide"), { recursive: true });
  cpSync(join(root, "examples"), join(home, "examples"), { recursive: true, filter: (src) => !/\.tmp\.[^/\\]*$|big(_\d+)?\.csv$/.test(src) });
  cpSync(join(root, "ide", "assets"), join(home, "ide", "assets"), { recursive: true });
  readOnly(home);
  // ("Documents": the scene's own, never the user's — Studio runs in the
  // scene's folder, so the Output shows a short path)
  return { home, env: { RAPIDR_DOCUMENTS: "Documents" } };
}

export const scenes = [
  // the example's card clicked: the copy opens, the Output says where
  {
    name: "studio-example-copy",
    studio: { delay: 5 },
    crop: [0, 64, 560, 706],
    env: { RAPIDR_TEST_EVENTS: "extitle(1).__mousedown_5_5,extitle(1).__mouseup_5_5" },
    setup: (work, root) => install(work, root),
  },
  // File > Copy Example Again from the Original: asked first
  {
    name: "copy-again-question",
    studio: { delay: 5 },
    window: 2,
    setup: (work, root) => ({ ...install(work, root), open: join(work, "install", "examples", "data", "linked_data.rr"), do: "file.exampleAgain" }),
  },
  // million_rows.rr run where it can't write: the window says so
  {
    name: "million-rows-read-only",
    program: "examples/data/million_rows.rr",
    args: ["2000"],
    delay: 3,
    crop: [0, 0, 638, 110],
    setup: (work, root) => ({ cwd: join(install(work, root).home, "examples", "data") }),
  },
];
