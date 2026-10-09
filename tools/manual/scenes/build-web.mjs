// docs/manual/building-apps.md's screenshots (WEB-BUILD): Run > Build Web App
// (.zip) in RapidR Studio on the web — the Run menu (no app builds for a
// computer there), and the Build page after the build. RapidR's own fixture
// (tests/fixtures/studio_app).

const NOTES = ["tests/fixtures/studio_app/Notes.rrproj", "tests/fixtures/studio_app/main.rr", "tests/fixtures/studio_app/note.svg"];

export const scenes = [
  {
    topic: "build-web",
    name: "build-page",
    host: "web",
    open: "tests/fixtures/studio_app/Notes.rrproj",
    webFiles: NOTES,
    do: "run.buildWeb",
    delay: 12,
    viewport: { width: 1280, height: 800 },
    crop: [296, 480, 400, 320],
  },
  {
    topic: "build-web",
    name: "run-menu",
    host: "web",
    open: "tests/fixtures/studio_app/Notes.rrproj",
    webFiles: NOTES,
    do: "wait",
    delay: 6,
    viewport: { width: 1280, height: 800 },
    // (the menu bar's Run, a real click: the menu bar is drawn, not DOM)
    web: async (page) => {
      // (the page again without the capture hooks, which end Studio's run: a Studio that takes real input)
      await page.addInitScript(() => { window.RAPIDR_STUDIO_TEST = {}; });
      await page.reload({ waitUntil: "load" });
      await page.waitForTimeout(6000);
      await page.mouse.move(220, 44);
      await page.waitForTimeout(200);
      await page.mouse.down();
      await page.mouse.up();
      await page.waitForTimeout(800);
    },
    crop: [0, 0, 640, 360],
  },
];
