// The manual's "Debugging in RapidR Studio" (docs/manual/debugging.md):
// its screenshots, made by tools/manual/shots.mjs. The programs are the
// page's own (tools/manual/scenes/debugger/); the steps are Studio's --do
// steps (commands, and the test steps of ide/debug.inc and ide/panels.inc).
//
// Studio's window is 1280 × 800 (logical pixels): the editor from (244, 70),
// the panes below it from y = 568, the status bar at y = 750; the
// Properties column from x = 1036 isn't needed here.

const COUNTER = "tools/manual/scenes/debugger/counter.rr";
const SHARE = "tools/manual/scenes/debugger/share.rr";
const GREETER = "tools/manual/scenes/debugger/greeter.rr";

// (the debugger's example: examples/debugging/shop.rr — line 76 adds a
// LineTotal in AddUp's loop; line 147 calls AddUp, after the week's table)
const SHOP = "examples/debugging/shop.rr";
const LOOP = "line:76,debug.toggleBreakpoint,run.start,wait,wait,wait";

const EDITOR = [244, 70, 792, 300];
const EDITOR_TALL = [244, 70, 792, 560];
// (the Variables pane alone)
const VARS = [544, 568, 492, 202];
const BOTTOM = [0, 568, 1036, 202];
const WORKBENCH = [0, 64, 1036, 706];
const BP = "line:11,debug.toggleBreakpoint";
const HIT = `${BP},run.start,wait,wait,wait`;

export const scenes = [
  { topic: "debugging", name: "breakpoint-set", open: COUNTER, do: `${BP},wait`, delay: 4, crop: EDITOR },
  { topic: "debugging", name: "breakpoint-hit", open: COUNTER, do: HIT, delay: 6, crop: [0, 0, 1036, 770] },
  { topic: "debugging", name: "breakpoint-condition", open: COUNTER, do: `${BP},debug.editBreakpoint,bpcond:i = 2,wait`, delay: 4, crop: WORKBENCH },
  { topic: "debugging", name: "logpoint", open: COUNTER, do: `${BP},bplog:adding {i}: total is {total},run.start,wait,wait,wait,wait`, delay: 7, crop: BOTTOM },
  { topic: "debugging", name: "step-into", open: COUNTER, do: `${HIT},debug.stepInto,wait,debug.stepOver,wait,view.variables`, delay: 7, crop: WORKBENCH },
  { topic: "debugging", name: "watch", open: COUNTER, do: `${HIT},debug.stepInto,wait,debug.stepOver,wait,watch:total + k,watch:n * 100,view.watch,wait`, delay: 7, crop: BOTTOM },
  { topic: "debugging", name: "set-value", open: COUNTER, do: `${HIT},view.variables,varpick:G/total,key:F2,type:100`, delay: 7, crop: BOTTOM },
  { topic: "debugging", name: "data-tip", open: COUNTER, do: `${BP},bpcond:i = 2,run.start,wait,wait,wait,debug.stepInto,wait,debug.stepOver,wait,hover:7:5,wait`, delay: 7, crop: EDITOR },
  { topic: "debugging", name: "call-stack-frame", open: COUNTER, do: `${HIT},debug.stepInto,wait,debug.stepOver,wait,view.variables,frame:1,wait`, delay: 7, crop: WORKBENCH },
  { topic: "debugging", name: "immediate", open: COUNTER, do: `${BP},bpcond:i = 3,run.start,wait,wait,wait,view.immediate,focus:immediatebox,type:? total * 10,key:Enter,wait,wait,wait,type:total = 0,key:Enter,wait,wait,wait,type:? total,key:Enter,wait,wait`, delay: 12, crop: BOTTOM },
  { topic: "debugging", name: "pause-waiting", open: GREETER, do: "run.start,wait,wait,wait,run.pause,wait,wait", delay: 6, crop: [0, 64, 1036, 706] },
  { topic: "debugging", name: "program-window", open: GREETER, do: "run.start,wait,wait,wait", delay: 5, program: true },
  { topic: "debugging", name: "runtime-error", open: SHARE, do: "run.start,wait,wait,wait,view.variables", delay: 6, crop: WORKBENCH },
  // (DEBUG-VM: the values inspector, Step Into Specific, Set Next Statement)
  { topic: "debugging", name: "datatip-tree", open: SHOP, do: `${LOOP},hover:76:37,wait,wait,wait,tipopen:1,wait,wait,tipopen:4,wait,wait,tipopen:7,wait,wait`, delay: 9, crop: EDITOR_TALL },
  { topic: "debugging", name: "datatip-frame", open: SHOP, do: "line:147,debug.toggleBreakpoint,run.start,wait,wait,wait,hover:140:2,wait,wait,wait,wait", delay: 8, crop: EDITOR },
  { topic: "debugging", name: "variables-changed", open: SHOP, do: `${LOOP},run.start,wait,wait,wait,view.variables`, delay: 8, crop: VARS },
  { topic: "debugging", name: "variables-components", open: SHOP, do: `${LOOP},view.variables,collapse:L,collapse:G,expand:C,wait,wait,expand:C/Form,wait,wait,expand:C/Form/Components,wait,wait`, delay: 10, crop: VARS },
  { topic: "debugging", name: "text-viewer", open: SHOP, do: "line:147,debug.toggleBreakpoint,run.start,wait,wait,wait,hover:142:3,wait,wait,wait,tipaction:0:view,wait,wait", delay: 8, window: 2 },
  { topic: "debugging", name: "step-into-specific", open: SHOP, do: `${LOOP},into:LineTotal,wait,wait,wait,view.variables`, delay: 8, crop: WORKBENCH },
  { topic: "debugging", name: "set-next", open: COUNTER, do: `${HIT},debug.stepInto,wait,wait,wait,debug.stepOver,wait,wait,wait,debug.stepOver,wait,wait,wait,debug.stepOver,wait,wait,wait,goto:6,wait,wait`, delay: 11, crop: WORKBENCH },
  // (the web: Studio in the browser, paused as on the desktop — its window
  // under the page's title bar — and a running program's window dragged
  // past Studio's own, whole)
  { topic: "debugging", name: "web-breakpoint-hit", host: "web", open: COUNTER, do: HIT, delay: 6, crop: [0, 0, 1280, 800] },
  { topic: "debugging", name: "web-program-window", host: "web", open: GREETER, do: "run.start", delay: 4, crop: [940, 190, 500, 260], web: dragPast },
  // (a window bigger than the page — the 1200 x 800 dashboard on a 1280 x
  // 720 page —, scrolled to its bottom: the run area's scroll bar along
  // the page's right edge)
  { topic: "debugging", name: "web-big-window", host: "web", open: "examples/gui/sales_dashboard.rr", do: "run.start", delay: 4, maximized: true, viewport: { width: 1280, height: 720 }, crop: [0, 0, 1280, 720], web: scrollDown },
];

// (the program's desktop scrolled to the window's bottom: the wheel over
// the scroll bar at the page's right edge)
async function scrollDown(page) {
  await page.waitForFunction(() => (window.RAPIDR_STUDIO_RUN_RECTS || []).length > 0, null, { timeout: 30000 });
  await page.waitForTimeout(800);
  const vp = page.viewportSize();
  await page.mouse.move(vp.width - 6, Math.round(vp.height / 2));
  for (let i = 0; i < 4; i++) await page.mouse.wheel(0, 100);
  // (the mouse off the bar: it shows as it rests)
  await page.mouse.move(vp.width - 300, 8);
  await page.waitForTimeout(500);
}

// (the program's window, by its title bar, to straddle Studio's right edge)
async function dragPast(page) {
  const frame = () => page.frames().find((f) => f.url().endsWith("/run.html"));
  await page.waitForFunction(() => (window.RAPIDR_STUDIO_RUN_RECTS || []).length > 0, null, { timeout: 30000 });
  await page.waitForTimeout(500);
  const w = await frame().evaluate(() => {
    const r = document.querySelector(".rr-kwin").getBoundingClientRect();
    return { x: r.left, y: r.top, w: r.width };
  });
  const from = { x: w.x + w.w / 2, y: w.y + 12 };
  const to = { x: 1280 - w.w / 2 + 120, y: 260 };
  await page.mouse.move(from.x, from.y);
  await page.mouse.down();
  for (let i = 1; i <= 10; i++) await page.mouse.move(from.x + ((to.x - from.x) * i) / 10, from.y + ((to.y - from.y) * i) / 10);
  await page.mouse.up();
  await page.waitForTimeout(400);
}
