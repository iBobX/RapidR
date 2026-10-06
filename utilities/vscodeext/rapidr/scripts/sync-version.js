// The extension's version is RapidR's: `[workspace.package] version` in the
// repository's Cargo.toml, written into package.json (and package-lock.json).
// Outside a RapidR checkout it changes nothing.
//
//   node scripts/sync-version.js [--check]
'use strict';

const fs = require('fs');
const path = require('path');

const ext = path.join(__dirname, '..');
const cargo = path.join(ext, '..', '..', '..', 'Cargo.toml');

function workspaceVersion(text) {
    let section = '';
    for (const line of text.split(/\r?\n/)) {
        const header = /^\s*\[([^\]]+)\]\s*$/.exec(line);
        if (header) {
            section = header[1].trim();
            continue;
        }
        const m = /^\s*version\s*=\s*"([^"]+)"/.exec(line);
        if (m && section === 'workspace.package') return m[1];
    }
    return null;
}

if (!fs.existsSync(cargo)) {
    console.log(`(no ${cargo}: the version stays as package.json has it)`);
    process.exit(0);
}
const version = workspaceVersion(fs.readFileSync(cargo, 'utf8'));
if (!version) {
    console.error(`${cargo}: no [workspace.package] version`);
    process.exit(1);
}
const check = process.argv.includes('--check');
let stale = false;
for (const name of ['package.json', 'package-lock.json']) {
    const file = path.join(ext, name);
    if (!fs.existsSync(file)) continue;
    const json = JSON.parse(fs.readFileSync(file, 'utf8'));
    const before = json.version;
    const lockRoot = json.packages && json.packages[''];
    if (before === version && (!lockRoot || lockRoot.version === version)) continue;
    stale = true;
    if (check) {
        console.error(`${name}: version ${before}, RapidR is ${version} (npm run version:sync)`);
        continue;
    }
    json.version = version;
    if (lockRoot) lockRoot.version = version;
    fs.writeFileSync(file, JSON.stringify(json, null, 2) + '\n');
    console.log(`${name}: ${before} -> ${version}`);
}
if (check && stale) process.exit(1);
if (!stale) console.log(`version ${version} (RapidR's)`);
