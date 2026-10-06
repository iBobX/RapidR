// The Marketplace README's screenshots (images/*.png), taken from a real VS
// Code driven through each feature (driver.js) on the showcase programs.
//
//   node test/screenshots/capture.js          (macOS; after `npm install`)
//   RAPIDR_PATH=/path/to/rapidr node test/screenshots/capture.js
//
// VS Code is @vscode/test-electron's copy (.vscode-test/), with a throwaway
// profile and a copy of showcase/ in a temporary folder — never your own VS
// Code, settings or files. The window is captured with macOS' screencapture
// (the terminal running this needs the Screen Recording permission); the
// title bar ("[Extension Development Host]") is cropped off and each image
// scaled to 1440 pixels wide (crop.swift). Look at every image before
// committing.
'use strict';

const { execFileSync } = require('child_process');
const fs = require('fs');
const os = require('os');
const path = require('path');
const { runTests } = require('@vscode/test-electron');

const EXT = path.resolve(__dirname, '..', '..');
const REPO = path.resolve(EXT, '..', '..', '..');
const SHOTS = ['completion', 'hover', 'signature', 'diagnostics', 'outline', 'debug'];

async function main() {
    if (process.platform !== 'darwin') throw new Error('the screenshots are taken on macOS (screencapture)');
    const rapidr = path.resolve(process.env.RAPIDR_PATH || path.join(REPO, 'target', 'debug', 'rapidr'));
    if (!fs.existsSync(rapidr)) throw new Error(`no rapidr at ${rapidr}: cargo build -p rapidr-cli, or set RAPIDR_PATH`);
    const work = fs.mkdtempSync(path.join(os.tmpdir(), 'rvshots-'));
    const workspace = path.join(work, 'rapidr-showcase');
    fs.cpSync(path.join(__dirname, 'showcase'), workspace, { recursive: true });
    const signals = path.join(work, 'signals');
    const out = path.join(work, 'out');
    fs.mkdirSync(signals);
    fs.mkdirSync(out);
    const winid = path.join(work, 'winid');
    execFileSync('swiftc', ['-O', path.join(__dirname, 'winid.swift'), '-o', winid]);
    const crop = path.join(work, 'crop');
    execFileSync('swiftc', ['-O', path.join(__dirname, 'crop.swift'), '-o', crop]);
    const userData = path.join(work, 'profile');
    fs.mkdirSync(path.join(userData, 'User'), { recursive: true });
    fs.writeFileSync(path.join(userData, 'User', 'settings.json'), JSON.stringify({
        'workbench.colorTheme': 'Default Dark Modern',
        'editor.fontSize': 15,
        'editor.lineHeight': 22,
        'editor.minimap.enabled': false,
        'editor.stickyScroll.enabled': false,
        'editor.suggest.preview': false,
        'workbench.startupEditor': 'none',
        'workbench.tips.enabled': false,
        'workbench.secondarySideBar.defaultVisibility': 'hidden',
        'workbench.layoutControl.enabled': false,
        'window.commandCenter': false,
        'window.newWindowDimensions': 'default',
        'security.workspace.trust.enabled': false,
        'update.mode': 'none',
        'extensions.autoUpdate': false,
        'telemetry.telemetryLevel': 'off',
        'chat.disableAIFeatures': true,
        'chat.commandCenter.enabled': false,
        'files.autoSave': 'off',
        'debug.toolBarLocation': 'docked',
        'rapidr.path': rapidr,
    }, null, 2));

    // The driver says when a state is on screen (ready-<name>); the window
    // is captured from here, and done-<name> lets it go on.
    const captured = [];
    const timer = setInterval(() => {
        for (const f of fs.readdirSync(signals)) {
            if (!f.startsWith('ready-')) continue;
            const name = f.slice('ready-'.length);
            fs.rmSync(path.join(signals, f));
            try {
                const id = execFileSync(winid, ['rapidr-showcase']).toString().trim();
                execFileSync('screencapture', ['-x', '-o', '-l', id, path.join(out, `${name}.png`)]);
                captured.push(name);
            } catch (err) {
                console.error(`${name}: ${err.message}`);
            }
            fs.writeFileSync(path.join(signals, `done-${name}`), '');
        }
    }, 300);
    try {
        await runTests({
            extensionDevelopmentPath: EXT,
            extensionTestsPath: path.join(__dirname, 'driver.js'),
            launchArgs: [workspace, '--skip-welcome', '--skip-release-notes', '--disable-telemetry',
                // (the window is captured while other windows may cover it)
                '--disable-renderer-backgrounding', '--disable-backgrounding-occluded-windows', '--disable-background-timer-throttling',
                '--user-data-dir', userData, '--extensions-dir', path.join(userData, 'extensions')],
            extensionTestsEnv: { SHOT_SIGNALS: signals, RAPIDR_PRINT_TO: path.join(work, 'prints'), RAPIDR_REGISTRY: path.join(work, 'registry.reg') },
        });
    } finally {
        clearInterval(timer);
    }
    for (const name of SHOTS) {
        const src = path.join(out, `${name}.png`);
        if (!fs.existsSync(src)) throw new Error(`no ${name} screenshot`);
        const dest = path.join(EXT, 'images', `${name}.png`);
        const h = Number(execFileSync('sips', ['-g', 'pixelHeight', src]).toString().match(/pixelHeight: (\d+)/)[1]);
        // (the title bar: 33 points of a 900-point window)
        execFileSync(crop, [src, dest, String(Math.round((h * 66) / 1800)), '1440']);
        console.log(`images/${name}.png`);
    }
    fs.rmSync(work, { recursive: true, force: true });
}

main().catch((err) => {
    console.error(err);
    process.exit(1);
});
