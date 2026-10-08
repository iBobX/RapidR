// The start of a native web build (`rapidr build --web`;
// interpreter/rapidr-webbundle): loads the program's own wasm module, named
// by the page's <meta name="rapidr-module">. Nothing in this file is
// generated: no project or file name is ever written into a script
// (docs/security-audit.md SEC-16).
const name = document.querySelector('meta[name="rapidr-module"]')?.content || "program";
const program = await import("./" + encodeURIComponent(name) + ".js");
await program.default();
