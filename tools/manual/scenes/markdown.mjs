// docs/manual/markdown.md's screenshots (MD-VIEW): a Markdown file in RapidR
// Studio — a README in Preview, the same side by side with its source, and
// a RapidQ import's report. RapidR's own fixtures (tests/fixtures/markdown).

export const scenes = [
  { topic: "markdown", name: "preview", open: "tests/fixtures/markdown/README.md", do: "wait", delay: 4 },
  { topic: "markdown", name: "side-by-side", open: "tests/fixtures/markdown/README.md", do: "wait,view:Split", delay: 4 },
  { topic: "markdown", name: "import-report", open: "tests/fixtures/markdown/rapidr-import-report.md", do: "wait", delay: 4 },
];
