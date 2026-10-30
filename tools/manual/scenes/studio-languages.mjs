// docs/manual/studio-languages.md's screenshots (I18N-STUDIO): RapidR Studio
// in Spanish (the ide/lang/es.po sample: the menu bar, Settings' Language),
// a compiler error in Problems in Spanish, and the pseudo-language xx that
// shows every text a catalog translates — switched as Settings switches it
// (the `lang:` step), live.

import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

export const topic = "studio-languages";

export const scenes = [
  {
    name: "settings-language",
    open: "",
    do: "wait,lang:es,wait,view.settings",
    window: 2,
    delay: 6,
    crop: [0, 0, 660, 300],
  },
  {
    name: "menus-spanish",
    open: "examples/gui/hello_form.rr",
    do: "wait,lang:es,wait",
    delay: 7,
    crop: [0, 0, 760, 330],
  },
  {
    name: "problems-spanish",
    // (a program with a mistake, in the scene's folder)
    setup(work) {
      mkdirSync(work, { recursive: true });
      writeFileSync(join(work, "mistake.rr"), "' a mistake: Problems says it in Studio's language\nDIM b AS RButton\nPRINT \"x\" THEN\n");
      return { open: join(work, "mistake.rr") };
    },
    do: "wait,lang:es,wait,view.problems",
    delay: 7,
    crop: [244, 560, 794, 190],
  },
  {
    name: "pseudo-language",
    open: "",
    do: "wait,lang:xx,wait",
    delay: 6,
  },
];
