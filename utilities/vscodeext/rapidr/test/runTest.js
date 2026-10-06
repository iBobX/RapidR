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
    const userData = fs.mkdtempSync(path.join(os.tmpdir(), 'rvsc-'));
    try {
        await run(ext, rapidr, prints, userData);
    } finally {
        fs.rmSync(userData, { recursive: true, force: true });
        fs.rmSync(prints, { recursive: true, force: true });
    }
}

async function run(ext, rapidr, prints, userData) {
    await runTests({
        ...vscodeToUse(),
        extensionDevelopmentPath: ext,
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
            RAPIDR_PATH: rapidr,
            RAPIDR_PRINT_TO: prints,
            // (and nothing touches the real registry / preferences store)
            RAPIDR_REGISTRY: path.join(prints, 'registry.reg'),
            RAPIDR_TEST_GREP: process.env.RAPIDR_TEST_GREP || '',
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
