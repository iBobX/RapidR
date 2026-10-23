// docs/manual/web.md's screenshots (WEB-OPEN): RapidR Studio on the web
// given a project file alone (File > Open picks one file: the browser lets
// the page read only that one) asks to open the project's folder. RapidR's
// own fixture (tests/fixtures/studio_web_open), the project file alone in
// the page's store.

export const scenes = [
  {
    topic: "web",
    name: "open-its-folder",
    host: "web",
    webFiles: ["tests/fixtures/studio_web_open/Kiosk.rrproj"],
    env: { RAPIDR_TEST_FILE_DIALOG: "tests/fixtures/studio_web_open/Kiosk.rrproj" },
    do: "file.open",
    delay: 5,
    viewport: { width: 1280, height: 800 },
    crop: [393, 298, 495, 203],
  },
];
