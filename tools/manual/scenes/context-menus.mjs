// The manual's "Right-click menus" (docs/manual/right-click-menus.md and
// the section in docs/manual/components.md): the context menus example
// (examples/gui/context_menus.rr) — a note right-clicked (its menu, Open
// bold, the keys shown, Delete last), a row's actions under the mouse —
// and RapidR Studio's own menus: a file in the Project tree right-clicked,
// a component in the designer, the code editor's menu opened with
// Shift+F10 (under the caret), a property's menu in the dark look.
export const topic = "context-menus";

const NOTES = "examples/gui/context_menus.rr";
const COUNTER = "tests/fixtures/studio_search/Counter.rrproj";
const HELLO = "examples/gui/hello_form.rr";

export const scenes = [
  { name: "notes-menu", program: NOTES, delay: 2, env: { RAPIDR_TEST_EVENTS: "notes.__wait_200,notes.__rightclick_60_45" } },
  { name: "notes-row-actions", program: NOTES, delay: 2, env: { RAPIDR_TEST_EVENTS: "notes.__wait_200,notes.__mousemove_150_71" } },
  { name: "studio-project-file", open: COUNTER, do: "view.project,wait", delay: 6, env: { RAPIDR_TEST_EVENTS: "studio.__wait_3000,projecttree.__rightclick_110_102,projecttree.__wait_500" }, crop: [0, 68, 640, 420] },
  { name: "studio-designer", open: HELLO, do: "wait", delay: 6, env: { RAPIDR_TEST_EVENTS: "studio.__wait_3000,designdoc(0).__rightclick_180_118,designdoc(0).__wait_500" }, crop: [240, 68, 560, 520] },
  { name: "studio-code-keyboard", open: COUNTER, do: "wait", delay: 6, env: { RAPIDR_TEST_EVENTS: "studio.__wait_3000,codedoc(0).__mousedown_120_212,codedoc(0).__mouseup_120_212,codedoc(0).__key_121_256,codedoc(0).__wait_500" }, crop: [240, 68, 520, 560] },
  { name: "studio-inspector-dark", theme: "rapidr-dark", open: HELLO, do: "wait", delay: 6, env: { RAPIDR_TEST_EVENTS: "studio.__wait_3000,designdoc(0).__mousedown_180_118,designdoc(0).__mouseup_180_118,inspector.__wait_300,inspector.__rightclick_60_230,inspector.__wait_500" }, crop: [760, 68, 518, 480] },
];
