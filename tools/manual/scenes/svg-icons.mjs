// The manual's "Your own SVG icons" (docs/manual/svg-icons.md) and
// "Keeping secrets" (docs/manual/secrets.md): the SVG icons example
// (examples/gui/svg_icons.rr) in light, dark and high contrast — the same
// SVGs drawn again in each theme's colours, at 2x —, and the secrets
// example (examples/data/secrets.rr) after a password was saved and read
// back, in the test keystore (shots.mjs: RAPIDR_AI_KEYSTORE=memory).
export const topic = "svg-icons";

const ICONS = "examples/gui/svg_icons.rr";
const SECRETS = "examples/data/secrets.rr";

export const scenes = [
  { name: "svg-icons-light", program: ICONS, delay: 2, crop: [0, 0, 400, 230] },
  { name: "svg-icons-dark", program: ICONS, delay: 2, crop: [0, 0, 400, 230], env: { RAPIDR_TEST_EVENTS: "pickdark.onclick" } },
  { name: "svg-icons-high-contrast", program: ICONS, delay: 2, crop: [0, 0, 400, 230], env: { RAPIDR_TEST_EVENTS: "pickcontrast.onclick" } },
  { topic: "secrets", name: "secrets-read-back", program: SECRETS, delay: 2,
    env: { RAPIDR_TEST_EVENTS: "valueedit.__key_83,valueedit.__key_51,valueedit.__key_67,valueedit.__key_82,valueedit.__key_69,valueedit.__key_84,savebtn.onclick,readbtn.onclick" } },
];
