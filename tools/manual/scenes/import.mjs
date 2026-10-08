// docs/manual/importing-rapidq.md's screenshots (R-NAMES): the Import
// commands in the command palette, and a RapidQ-style program imported —
// the copy open as a project, its report beside the code. The program is
// RapidR's own fixture (tests/fixtures/rapidq_import), never RapidQ's.

import { cpSync } from "node:fs";
import { join } from "node:path";

export const scenes = [
  { topic: "import", name: "import-commands", do: "wait,palette:import", delay: 5 },
  {
    topic: "import",
    name: "import-result",
    delay: 7,
    setup(work, root) {
      cpSync(join(root, "tests", "fixtures", "rapidq_import"), join(work, "greeter"), { recursive: true });
      return { do: `import:${join(work, "greeter", "greeter.rqw")}` };
    },
  },
  {
    topic: "import",
    name: "imported-design",
    delay: 7,
    setup(work, root) {
      cpSync(join(root, "tests", "fixtures", "rapidq_import"), join(work, "greeter"), { recursive: true });
      return { do: `import:${join(work, "greeter", "greeter.rqw")},wait,file.close,wait,view.designer,pick:HelloBtn` };
    },
  },
];
