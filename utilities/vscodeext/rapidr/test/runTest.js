// The integration tests: downloads a VS Code (once, into .vscode-test/),
// starts it on test/fixtures/workspace with only this extension and a fresh,
// temporary profile (never yours), and runs test/suite/*.test.js.
//
//   npm test                          (unit tests, then these)
//   RAPIDR_PATH=/path/to/rapidr npm test
//   VSCODE_VERSION=1.90.0 npm test    (default: stable)
//   VSCODE_PATH=".../Visual Studio Code.app/Contents/MacOS/Code" npm test
//                                     (an installed VS Code; still a throwaway profile)
//   RAPIDR_TEST_GREP=diagnostics npm test
//   RAPIDR_EXTENSION_PATH=/tmp/unzipped/extension npm test   (the packaged extension, an unzipped .vsix)
//
// Two VS Code runs: the suite (the extension finds rapidr through
// RAPIDR_PATH), then test/suite/path.test.js in a VS Code whose PATH has an
// ancient rapidr 0.1.0 first and the real one after it (the extension must
// skip the old one: the finding-rapidr bug of 2.121.3).
//
// Needs a rapidr with `rapidr lsp`: RAPIDR_PATH, else this checkout's
// target/debug/rapidr (cargo build -p rapidr-cli). On Linux without a display:
// xvfb-run -a npm test.
'use strict';

const fs = require('fs');
const os = require('os');
const path = require('path');
const { runTests } = require('@vscode/test-electron');

async function main() {
    const ext = path.resolve(__dirname, '..');
    const repo = path.resolve(ext, '..', '..', '..');
    const exe = process.platform === 'win32' ? 'rapidr.exe' : 'rapidr';
    const rapidr = path.resolve(process.env.RAPIDR_PATH || path.join(repo, 'target', 'debug', exe));
    if (!fs.existsSync(rapidr)) {
        console.error(`No rapidr at ${rapidr}: build it (cargo build -p rapidr-cli) or set RAPIDR_PATH.`);
        process.exit(1);
    }
    // (nothing the tests run prints on a real printer)
    const prints = fs.mkdtempSync(path.join(os.tmpdir(), 'rapidr-vscode-prints-'));
    // A fresh profile each run, with a short path: VS Code's IPC socket lives
    // in it, and a socket's path can't pass 103 characters on macOS.
    const userData = () => fs.mkdtempSync(path.join(os.tmpdir(), 'rvsc-'));
    const profiles = [];
    try {
        const first = userData();
        profiles.push(first);
        await run(ext, prints, first, { RAPIDR_PATH: rapidr, RAPIDR_TEST_GREP: process.env.RAPIDR_TEST_GREP || '' });
        if (process.platform !== 'win32' && !process.env.RAPIDR_TEST_GREP) {
            const second = userData();
            profiles.push(second);
            await run(ext, prints, second, pathEnvironment(rapidr, prints));
        }
    } finally {
        for (const p of profiles) fs.rmSync(p, { recursive: true, force: true });
        fs.rmSync(prints, { recursive: true, force: true });
    }
}

/**
 * The second run's environment: RAPIDR_PATH is off, and PATH has a "rapidr"
 * that is version 0.1.0 (it knows nothing of `lsp`), then the real one.
 */
function pathEnvironment(rapidr, scratch) {
    const old = path.join(scratch, 'old-bin');
    const real = path.join(scratch, 'real-bin');
    fs.mkdirSync(old, { recursive: true });
    fs.mkdirSync(real, { recursive: true });
    fs.writeFileSync(path.join(old, 'rapidr'), '#!/bin/sh\nif [ "$1" = "version" ]; then echo "RapidR 0.1.0"; else echo "error: unknown command $1" >&2; exit 2; fi\n', { mode: 0o755 });
    fs.symlinkSync(rapidr, path.join(real, 'rapidr'));
    return {
        RAPIDR_PATH: '',
        RAPIDR_TEST_SUITE: 'path',
        RAPIDR_TEST_REAL: rapidr,
        RAPIDR_TEST_OLD: path.join(old, 'rapidr'),
        PATH: [old, real, process.env.PATH || ''].join(path.delimiter),
    };
}

async function run(ext, prints, userData, env) {
    await runTests({
        ...vscodeToUse(),
        // (RAPIDR_EXTENSION_PATH: the `extension/` folder of an unzipped .vsix — what is
        // installed, not the source: a file the package leaves out shows up here)
        extensionDevelopmentPath: process.env.RAPIDR_EXTENSION_PATH ? path.resolve(process.env.RAPIDR_EXTENSION_PATH) : ext,
        extensionTestsPath: path.join(__dirname, 'suite', 'index.js'),
        launchArgs: [
            path.join(__dirname, 'fixtures', 'workspace'),
            '--disable-extensions',
            '--disable-workspace-trust',
            '--skip-welcome',
            '--skip-release-notes',
            '--disable-telemetry',
            '--user-data-dir', userData,
            '--extensions-dir', path.join(ext, '.vscode-test', 'extensions'),
        ],
        extensionTestsEnv: {
            RAPIDR_PRINT_TO: prints,
            // (and nothing touches the real registry / preferences store)
            RAPIDR_REGISTRY: path.join(prints, 'registry.reg'),
            RAPIDR_TEST_GREP: '',
            ...env,
        },
    });
}

// VSCODE_PATH: a VS Code already installed (its executable, e.g.
// "/Applications/Visual Studio Code.app/Contents/MacOS/Code"), run with the
// throwaway profile above; else @vscode/test-electron's download.
function vscodeToUse() {
    if (process.env.VSCODE_PATH) return { vscodeExecutablePath: process.env.VSCODE_PATH };
    return { version: process.env.VSCODE_VERSION || 'stable' };
}

main().catch((err) => {
    console.error(err && err.message ? err.message : err);
    process.exit(1);
});
