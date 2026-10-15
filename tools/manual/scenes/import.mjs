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
  // (IMPORT-INC) "Include files": Tools > Settings' include folders (a
  // folder as a RapidQ user's would be), and RapidR's own Shapes example
  // (examples/rapidq/shapes: includes in lib/ and forms/) imported — its
  // includes in the copy's project, the report's Include files table.
  { topic: "import", name: "include-settings", do: "includes:/Users/you/RapidQ/include,wait,view.settings", window: 2, delay: 5 },
  {
    topic: "import",
    name: "import-includes",
    delay: 8,
    setup(work, root) {
      cpSync(join(root, "examples", "rapidq", "shapes"), join(work, "shapes"), { recursive: true });
      return { do: `import:${join(work, "shapes", "shapes.bas")},wait,anchor:include-files` };
    },
  },
];
