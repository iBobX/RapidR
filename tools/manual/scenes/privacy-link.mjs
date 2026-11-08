// Studio's Privacy policy link (docs/manual/getting-started.md, "The Help
// menu"): the Welcome page's quiet Privacy link at its foot in the light and
// the dark look, and Help > Privacy Policy in Studio's menu.
// (a wide window: the page's cards in four columns, so its foot shows
// without scrolling)
export const topic = "help-menu";

export const scenes = [
  { name: "welcome-privacy-light", open: "", do: "layout:size=1800x960,wait", delay: 6, crop: [240, 60, 1260, 660] },
  { name: "welcome-privacy-dark", theme: "rapidr-dark", open: "", do: "layout:size=1800x960,wait", delay: 6, crop: [240, 60, 1260, 660] },
  // (Help open, as Alt+H opens it)
  { name: "help-menu", open: "", do: "wait", delay: 6, env: { RAPIDR_TEST_EVENTS: "studio.__wait_3000,studio.__key_72_1,studio.__wait_500" }, crop: [380, 0, 420, 200] },
  // (the same in Spanish: Alt+Y opens Ayuda)
  { name: "help-menu-es", open: "", do: "wait,lang:es", delay: 7, env: { RAPIDR_TEST_EVENTS: "studio.__wait_3500,studio.__key_89_1,studio.__wait_500" }, crop: [330, 0, 520, 200] },
];
