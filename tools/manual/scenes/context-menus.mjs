// The manual's "Right-click menus" (docs/manual/right-click-menus.md and
// the section in docs/manual/components.md): the context menus example
// (examples/gui/context_menus.rr) — a note right-clicked (its menu, Open
// bold, the keys shown, Delete last), the ••• button's menu, opened by its
// OnClick and still open when the click is over, a row's actions under the mouse —
// and RapidR Studio's own menus: a file in the Project tree right-clicked,
// a component in the designer, the code editor's menu opened with
// Shift+F10 (under the caret), a property's menu in the dark look; (2b)
// a problem's menu in the Problems page, the code's menu on a problem with
// its fix first, the Watch pane's menu opened from the keyboard while paused.
export const topic = "context-menus";

const NOTES = "examples/gui/context_menus.rr";
const COUNTER = "tests/fixtures/studio_search/Counter.rrproj";
const HELLO = "examples/gui/hello_form.rr";
const PROBLEMS = "tests/fixtures/studio_copy/problems.rr";
const FIX = "tests/fixtures/studio_ctx/fix.rr";
const SHOP = "examples/debugging/shop.rr";

export const scenes = [
  { name: "notes-menu", program: NOTES, delay: 2, env: { RAPIDR_TEST_EVENTS: "notes.__wait_200,notes.__rightclick_60_45" } },
  { name: "notes-dots-menu", program: NOTES, delay: 2, env: { RAPIDR_TEST_EVENTS: "notes.__wait_200,notes.__item_0,morebtn.__mousedown_30_13,morebtn.__mouseup_30_13" } },
  { name: "notes-row-actions", program: NOTES, delay: 2, env: { RAPIDR_TEST_EVENTS: "notes.__wait_200,notes.__mousemove_150_71" } },
  { name: "studio-project-file", open: COUNTER, do: "view.project,wait", delay: 6, env: { RAPIDR_TEST_EVENTS: "studio.__wait_3000,projecttree.__rightclick_110_102,projecttree.__wait_500" }, crop: [0, 68, 640, 470] },
  { name: "studio-designer", open: HELLO, do: "wait", delay: 6, env: { RAPIDR_TEST_EVENTS: "studio.__wait_3000,designdoc(0).__rightclick_180_118,designdoc(0).__wait_500" }, crop: [240, 68, 560, 520] },
  { name: "studio-code-keyboard", open: COUNTER, do: "wait", delay: 6, env: { RAPIDR_TEST_EVENTS: "studio.__wait_3000,codedoc(0).__mousedown_120_212,codedoc(0).__mouseup_120_212,codedoc(0).__key_121_256,codedoc(0).__wait_500" }, crop: [240, 68, 520, 560] },
  { name: "studio-problems", open: PROBLEMS, do: "wait,run.start,wait,wait", delay: 8, env: { RAPIDR_TEST_EVENTS: "outputbox.__wait_4000,outputbox.__rightclick_300_60,outputbox.__wait_400" }, crop: [240, 520, 620, 249] },
  { name: "studio-code-fix", open: FIX, do: "view.code,wait", delay: 8, env: { RAPIDR_TEST_EVENTS: "studio.__wait_4500,codedoc(0).__rightclick_120_130,codedoc(0).__wait_600" }, crop: [240, 140, 460, 470] },
  { name: "studio-watch-keyboard-dark", theme: "rapidr-dark", open: SHOP, do: "line:76,debug.toggleBreakpoint,watch:count,run.start,wait,wait,wait,view.watch,watchpick:W1", delay: 10, env: { RAPIDR_TEST_EVENTS: "studio.__wait_7000,watchtree.__key_121_256,watchtree.__wait_600" }, crop: [540, 560, 470, 209] },
  { name: "studio-greyed-tip", open: "tests/fixtures/studio_copy/three.rr", do: "run.start,wait,wait,wait", delay: 9, env: { RAPIDR_TEST_EVENTS: "outputbox.__wait_4000,outputbox.__rightclick_100_80,outputbox.__wait_300,studio.__mousemove_400_590,outputbox.__wait_1500" }, crop: [240, 560, 660, 209] },
  { name: "studio-inspector-dark", theme: "rapidr-dark", open: HELLO, do: "wait", delay: 6, env: { RAPIDR_TEST_EVENTS: "studio.__wait_3000,designdoc(0).__mousedown_180_118,designdoc(0).__mouseup_180_118,inspector.__wait_300,inspector.__rightclick_60_230,inspector.__wait_500" }, crop: [760, 68, 518, 480] },
];
