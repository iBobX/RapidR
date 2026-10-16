// The manual's "Themes" section (docs/manual/components.md#themes) and
// RapidR Studio following the system's look (THEME-LIVE): the themes
// example (examples/gui/themes.rr) as it opens on a light system, then
// after the system switched to dark while it runs (the test hook
// RAPIDR_TEST_SYSTEM_LOOK and a `__look_dark` step: its OnThemeChange took
// the card's colours again); Studio's Welcome page the same way, and
// scrolled to its last example card by the keyboard.
export const topic = "themes";

const THEMES = "examples/gui/themes.rr";

export const scenes = [
  { name: "themes-light", program: THEMES, delay: 2, env: { RAPIDR_TEST_SYSTEM_LOOK: "light" } },
  { name: "themes-system-dark", program: THEMES, delay: 2, env: { RAPIDR_TEST_SYSTEM_LOOK: "light", RAPIDR_TEST_EVENTS: "form.__look_dark" } },
  { name: "studio-system-dark", theme: "rapidr", delay: 5, crop: [245, 68, 790, 500], env: { RAPIDR_TEST_SYSTEM_LOOK: "light", RAPIDR_TEST_EVENTS: "studio.__look_dark" } },
  { name: "studio-welcome-last-card", theme: "rapidr", delay: 5, crop: [245, 68, 790, 500], env: { RAPIDR_TEST_SYSTEM_LOOK: "dark", RAPIDR_TEST_EVENTS: "projecttree.__key_9_256" } },
];
