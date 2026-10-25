// The manual's "Projects in RapidR Studio" page (docs/manual/studio-projects.md,
// Preview 4 S1, S2, S6): a seven-file project with three forms in one file
// (tests/fixtures/studio_forms, RapidR's own test project), File > Open
// Recent, the offer to reopen the last project, and the PATH the programs
// run from Studio get (Tools > Settings > General).
//
// Each scene works in its own folder, which is also the home folder ($HOME)
// Studio sees: the paths on screen are short (~/Projects/Shelf), never the
// machine's.
import { cpSync, mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

export const topic = "studio-projects";

// (the project copied to <scene>/Projects/Shelf; pantry.rr to Projects/Pantry)
function projects(work, root) {
  cpSync(join(root, "tests/fixtures/studio_forms"), join(work, "Projects/Shelf"), { recursive: true });
  mkdirSync(join(work, "Projects/Pantry"), { recursive: true });
  cpSync(join(root, "examples/gui/pantry.rr"), join(work, "Projects/Pantry/pantry.rr"));
  return join(work, "Projects/Shelf/Shelf.rrproj");
}

// (a registry value as the registry file keeps a text with line breaks)
const hex = (s) => "hex(1):" + [...Buffer.from(s + "\0", "utf16le")].map((b) => b.toString(16).padStart(2, "0")).join(",");

// (Studio's settings as it left them: the recent list, the last project
// and the two files open in it)
function settings(work, root) {
  const shelf = projects(work, root);
  const pantry = join(work, "Projects/Pantry/pantry.rr");
  const files = `0\t${join(work, "Projects/Shelf/main.rr")}\n1\t${join(work, "Projects/Shelf/ui.rr")}\n`;
  writeFileSync(join(work, "studio.reg"), [
    "Windows Registry Editor Version 5.00", "",
    "[HKEY_CURRENT_USER]", "", "[HKEY_CURRENT_USER\\Software]", "", "[HKEY_CURRENT_USER\\Software\\RapidR]", "",
    "[HKEY_CURRENT_USER\\Software\\RapidR\\Studio]",
    `"Recent"=${hex(`${shelf}\n${pantry}\n`)}`,
    `"SessionProject"="${shelf}"`,
    `"SessionFiles"=${hex(files)}`,
    `"ReopenLast"="1"`, "",
  ].join("\r\n"));
  return { cwd: work, env: { HOME: work } };
}

export const scenes = [
  // the project tree: ui.rr under Forms with its three forms, main.rr under
  // Modules; NewForm (Home, Down × 4, Enter) open in ui.rr's designer
  {
    name: "forms-tree",
    setup: (work, root) => ({ open: projects(work, root), cwd: work, env: { HOME: work } }),
    env: { RAPIDR_TEST_EVENTS: "projecttree.__key_36,projecttree.__key_40,projecttree.__key_40,projecttree.__key_40,projecttree.__key_40,projecttree.__key_13" },
    delay: 6,
    crop: [0, 68, 1040, 330],
  },
  // File > Open Recent, opened from the keyboard (Alt+F, R)
  { name: "open-recent", fresh: false, setup: settings, env: { RAPIDR_TEST_EVENTS: "studio.__key_70_1,studio.__key_82" }, delay: 5, crop: [0, 0, 700, 360] },
  // the Welcome page's offer to reopen the last project (Enter reopens it)
  { name: "reopen", fresh: false, setup: settings, delay: 5, crop: [244, 96, 560, 420] },
  // Tools > Settings > General: the offer's setting and the programs' PATH,
  // read from a login shell (/bin/sh, with no startup files of its own)
  {
    name: "settings-path",
    setup: (work) => ({ cwd: work, env: { HOME: work, RAPIDR_LOGIN_PATH: "", RAPIDR_LOGIN_SHELL: "/bin/sh" } }),
    do: "view.settings",
    delay: 5,
    window: 2,
  },
  // The project's name selected at the top of the Project pane: the project's
  // properties in the Properties pane, Theme set to RapidR Dark (the main
  // file says so, and the form in the designer is dark; Studio stays light)
  {
    name: "project-properties",
    setup: (work, root) => {
      cpSync(join(root, "tests/fixtures/studio_projprops"), join(work, "Projects/RenMany"), { recursive: true });
      return { open: "Projects/RenMany/RenMany.rrproj", cwd: work, env: { HOME: work } };
    },
    do: "wait,view.designer,project:select,prop:Theme=RapidR Dark,row:Theme,wait",
    delay: 6,
    crop: [0, 68, 1280, 502],
  },
  // (S5) Project > Project Options: Targets, Web picked (what it means under it)
  {
    name: "targets",
    setup: (work, root) => {
      cpSync(join(root, "tests/fixtures/studio_search"), join(work, "counter"), { recursive: true });
      return { open: "counter/Counter.rrproj", cwd: work, env: { HOME: work } };
    },
    do: "wait,project.options",
    env: { RAPIDR_TEST_EVENTS: "apptargets.__item_1" },
    delay: 5,
    window: 2,
  },
];
