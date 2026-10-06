// Packages the extension: dist/rapidr-<version>.vsix (vsce runs
// `vscode:prepublish` first: the bundle and THIRD_PARTY_NOTICES.md).
// Publishing is by hand: docs/vscode-publishing.md.
//
//   node scripts/package.js [--out <file.vsix>]
'use strict';

const { execFileSync } = require('child_process');
const fs = require('fs');
const path = require('path');

const ext = path.join(__dirname, '..');
const { version } = JSON.parse(fs.readFileSync(path.join(ext, 'package.json'), 'utf8'));
const i = process.argv.indexOf('--out');
const out = i > 0 ? path.resolve(process.argv[i + 1]) : path.join(ext, 'dist', `rapidr-${version}.vsix`);
fs.mkdirSync(path.dirname(out), { recursive: true });
const vsce = require.resolve('@vscode/vsce/vsce', { paths: [ext] });
// (--no-dependencies: everything the extension runs is bundled into dist/extension.js)
execFileSync(process.execPath, [vsce, 'package', '--no-dependencies', '--out', out], { cwd: ext, stdio: 'inherit' });
console.log(`${out} (${(fs.statSync(out).size / 1024).toFixed(0)} KB)`);
