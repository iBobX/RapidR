// The manual's "Versions" (docs/manual/versions.md): Versions in RapidR
// Studio (lane G3) — a copy of a small project (tests/fixtures/
// studio_versions) started, saved as versions, changed, compared, gone back
// to; the local history of a project never versioned — and RDiffView's three
// views in the diff views example (examples/studio/diff_views.rr): the text
// view with its changed words, the form view (the form before and after, the
// changes listed), the table view of two CSV files.
import { cpSync, mkdirSync, mkdtempSync, symlinkSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

export const topic = "versions";

const DIFF = "examples/studio/diff_views.rr";

// (a fixture copied outside this repository — Studio rightly refuses to
// start versions in a folder inside another one that has them, as the
// flows' copies are — and reached as the scene's Projects/<name>: a short
// path on screen; Studio's HOME the scene's folder)
const project = (work, root, fixture, name) => {
  const copy = mkdtempSync(join(tmpdir(), "rapidr-versions-shot-"));
  cpSync(join(root, fixture), copy, { recursive: true });
  mkdirSync(join(work, "Projects"), { recursive: true });
  symlinkSync(copy, join(work, "Projects", name));
};
const hello = (work, root) => {
  project(work, root, "tests/fixtures/studio_versions", "Hello");
  return { open: "Projects/Hello/Hello.rrproj", cwd: work, env: { HOME: work } };
};
// (keys typed into a field, as the tests' typeInto: lower-case letters)
const typed = (comp, text) => [...text].map((c) => `${comp}.__key_${c === " " ? 32 : c.toUpperCase().charCodeAt(0)}`).join(",");
const click = (comp, x, y) => `${comp}.__mousedown_${x}_${y},${comp}.__mouseup_${x}_${y}`;

// (VERSIONS-POLISH: a project with 29 changes in folders — tests/fixtures/
// studio_versions_many —, a first version of main.rr and its project file)
const shots = (work, root) => {
  project(work, root, "tests/fixtures/studio_versions_many", "Shots");
  return { open: "Projects/Shots/Shots.rrproj", cwd: work, env: { HOME: work } };
};
const FIRST = "versions.pane,versions.start,versions:author=Ana,versions:save=first";
const EDIT = 'code:PRINT "Done."=>PRINT "All done!",file.save';
const SECOND = `${FIRST},${EDIT},versions:save=second`;

export const scenes = [
  // a project without versions: what they are, Start Versioning
  { name: "studio-start", setup: hello, do: "versions.pane", delay: 5, crop: [0, 68, 640, 420] },
  // two versions saved, one change since: the Versions pane, its timeline
  // (the left column a little wider than at first)
  { name: "studio-timeline", setup: hello, do: `${SECOND},code:PRINT "All done!"=>PRINT "Bye",file.save,versions.pane,versions:width=360,wait`, delay: 6, crop: [0, 68, 640, 560] },
  // 29 changes: the project's files, the folders (screenshots closed), the filter
  { name: "studio-changes", setup: shots, do: "versions.pane,versions.start,versions:author=Ana,versions:saveonly=first;main.rr;Shots.rrproj,versions.pane,versions:width=360,wait", env: { RAPIDR_TEST_EVENTS: "verchanges.__item_7" }, delay: 6, crop: [0, 68, 640, 560] },
  // the pane in a narrow column, in RapidR's dark look: the chips under the
  // title, the buttons on two rows
  { name: "studio-narrow", setup: hello, theme: "rapidr-dark", do: `${SECOND},code:PRINT "All done!"=>PRINT "Bye",file.save,versions.pane,versions:width=260,wait`, delay: 6, crop: [0, 68, 560, 560] },
  // main.rr changed since "first": the gutter's mark, the Project pane's M,
  // the status bar
  { name: "studio-changed", setup: hello, do: `${FIRST},${EDIT},view.project`, delay: 6 },
  // the mark clicked (Show This Line's Change): what the line was, Undo This Change
  { name: "studio-change-box", setup: hello, do: `${FIRST},${EDIT},line:12,versions.showChange`, delay: 6, crop: [240, 68, 640, 400] },
  // Save a Version...: the words, the files, your name
  { name: "studio-save", setup: hello, do: `${FIRST},${EDIT},versions.save`, env: { RAPIDR_TEST_EVENTS: typed("vermsgedit", "second") }, delay: 6, window: 2 },
  // a key in the code: said, saved only once ticked
  {
    name: "studio-secret",
    setup: hello,
    // (not a real key: made of two pieces, so no file holds a key's shape)
    do: `${FIRST},code:PRINT "Done."=>key$ = "${"sk"}-ant-api03-abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGH",file.save,versions.save`,
    env: { RAPIDR_TEST_EVENTS: `${typed("vermsgedit", "keys")},${click("versaveok", 40, 14)}` },
    delay: 6,
    window: 2,
  },
  // What Changed? of "second"
  { name: "studio-what-changed", setup: hello, do: `${SECOND},versions:pick=second,versions.whatChanged`, delay: 6, crop: [0, 68, 1040, 500] },
  // Go Back to "first": asked first
  { name: "studio-go-back", setup: hello, do: `${SECOND},versions:pick=first,versions.goBack`, delay: 5, window: 2 },
  // a project never versioned: a copy of main.rr at each save
  {
    name: "studio-local-history",
    setup: hello,
    do: 'code:PRINT "Done."=>PRINT "One",file.save,code:PRINT "One"=>PRINT "Two",file.save,code:PRINT "Two"=>PRINT "Three",file.save,versions.localHistory',
    env: { RAPIDR_TEST_EVENTS: "verdifflist.__item_3" },
    delay: 6,
    crop: [240, 68, 800, 500],
  },
  { name: "diff-text", program: DIFF, delay: 2, crop: [0, 0, 860, 600] },
  { name: "diff-form", program: DIFF, delay: 2, crop: [0, 0, 860, 600], env: { RAPIDR_TEST_EVENTS: "bform.onclick" } },
  { name: "diff-table", program: DIFF, delay: 2, crop: [0, 0, 860, 600], env: { RAPIDR_TEST_EVENTS: "btable.onclick" } },
];
