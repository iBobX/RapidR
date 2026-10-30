// The outside writer of tests/studio_flows.mjs' ext-* cases (lane
// EXTERNAL-CHANGES): another process changing a project's files while
// RapidR Studio has them open, as another AI agent, an editor or git would.
//
//   node tests/ext_writer.mjs <meeting folder> <project folder> <steps.json>
//
// steps.json: [{ "write": { "main.rr": "text" }, "remove": ["greet.rr"] }, …].
// Step n waits until Studio's `ext:go=<n>` leaves <meeting>/go-<n>, makes its
// changes in the project, then leaves <meeting>/done-<n> for Studio's
// `ext:await=<n>`. It gives up after two minutes.
import { existsSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const [meet, project, stepsFile] = process.argv.slice(2);
const steps = JSON.parse(readFileSync(stepsFile, "utf8"));
const until = Date.now() + 120000;
const pause = (ms) => new Promise((r) => setTimeout(r, ms));

for (let n = 1; n <= steps.length; n++) {
  while (!existsSync(join(meet, `go-${n}`))) {
    if (Date.now() > until) process.exit(1);
    await pause(30);
  }
  const s = steps[n - 1];
  for (const [rel, text] of Object.entries(s.write || {})) writeFileSync(join(project, rel), text);
  for (const rel of s.remove || []) rmSync(join(project, rel), { force: true });
  writeFileSync(join(meet, `done-${n}`), "done");
}
