// The bug of 2.121.3: an ancient `rapidr` 0.1.0 (from `cargo install`, no
// language server, no debugger) first on PATH was "found" and nothing
// worked — no completion, no hover, debugging did nothing, and no word said
// why. Run by test/runTest.js in a VS Code whose PATH is [the old rapidr,
// the real one] (RAPIDR_PATH off, no rapidr.path).
'use strict';

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');
const vscode = require('vscode');
const { activate, open, at, eventually, labels, hoverText } = require('./util');

describe('an ancient rapidr first on PATH', () => {
    let api;
    before(async function () {
        if (!process.env.RAPIDR_TEST_OLD) this.skip();
        api = await activate();
    });

    it('is skipped, the real one is used, and the output says why', () => {
        const r = api.rapidr();
        assert.strictEqual(r.source, 'PATH');
        assert.ok(r.path.endsWith('real-bin/rapidr'), r.path);
        assert.strictEqual(r.state, 'current');
        assert.strictEqual(r.skipped.length, 1);
        assert.strictEqual(r.skipped[0].path, process.env.RAPIDR_TEST_OLD);
        assert.strictEqual(r.skipped[0].version, '0.1.0');
        assert.match(r.skipped[0].error, /too old/);
    });

    it('completes after `DIM test AS `, hovers, and debugs', async () => {
        const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rvsc-path-'));
        const file = path.join(dir, 'types.bas');
        fs.writeFileSync(file, "$APPTYPE GUI\nDIM test AS \n");
        const types = await open(file);
        const list = await eventually(async () => {
            const l = await vscode.commands.executeCommand('vscode.executeCompletionItemProvider', types.uri, new vscode.Position(1, 'DIM test AS '.length));
            return labels(l).includes('INTEGER') ? l : null;
        }, 'completion after AS');
        assert.ok(labels(list).includes('STRING'));
        const main = await open('main.bas');
        const text = await eventually(async () => {
            const t = hoverText(await vscode.commands.executeCommand('vscode.executeHoverProvider', main.uri, at(main, 'MID$(', 1)));
            return /MID\$/i.test(t) ? t : null;
        }, 'hover');
        assert.ok(text);
        const events = [];
        const tracker = vscode.debug.registerDebugAdapterTrackerFactory('rapidr', {
            createDebugAdapterTracker: () => ({ onDidSendMessage: (m) => { if (m.type === 'event') events.push(m.event); } }),
        });
        try {
            const hello = await open('hello.bas');
            assert.ok(await vscode.debug.startDebugging(undefined, { type: 'rapidr', request: 'launch', name: 'hello', program: hello.uri.fsPath }));
            await eventually(() => events.includes('terminated'), 'the program to end', 30000);
        } finally {
            tracker.dispose();
        }
    });
});
