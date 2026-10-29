// Helpers of the integration tests (not a test file).
'use strict';

const assert = require('assert');
const path = require('path');
const vscode = require('vscode');

const FIXTURES = path.resolve(__dirname, '..', 'fixtures', 'workspace');
const EXTENSION_ID = 'rapidr.rapidr';

async function activate() {
    const ext = vscode.extensions.getExtension(EXTENSION_ID);
    assert.ok(ext, `${EXTENSION_ID} is not loaded`);
    const api = await ext.activate();
    assert.ok(api.rapidr().path, `rapidr was not found: ${api.rapidr().error}`);
    await api.whenServerReady();
    return api;
}

async function open(file) {
    const doc = await vscode.workspace.openTextDocument(path.isAbsolute(file) ? file : path.join(FIXTURES, file));
    await vscode.window.showTextDocument(doc);
    return doc;
}

/** The position `offset` characters into the first `text` in the document. */
function at(doc, text, offset = 0) {
    const i = doc.getText().indexOf(text);
    assert.ok(i >= 0, `"${text}" is not in ${doc.fileName}`);
    return doc.positionAt(i + offset);
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** Asks until `fn` gives something truthy (the server may still be analysing). */
async function eventually(fn, what, timeoutMs = 20000) {
    const end = Date.now() + timeoutMs;
    let last;
    for (;;) {
        try {
            const v = await fn();
            if (v) return v;
        } catch (err) {
            last = err;
        }
        if (Date.now() > end) throw new Error(`${what}: no answer in ${timeoutMs} ms${last ? ` (${last.message})` : ''}`);
        await sleep(250);
    }
}

function labels(list) {
    return (list ? list.items : []).map((i) => (typeof i.label === 'string' ? i.label : i.label.label));
}

function hoverText(hovers) {
    return (hovers || [])
        .flatMap((h) => h.contents)
        .map((c) => (typeof c === 'string' ? c : c.value || ''))
        .join('\n');
}

/** Reverts an edited document to what is on disk (the fixtures stay as they are). */
async function revert(doc) {
    await vscode.window.showTextDocument(doc);
    await vscode.commands.executeCommand('workbench.action.files.revert');
}

module.exports = { FIXTURES, EXTENSION_ID, activate, open, at, sleep, eventually, labels, hoverText, revert };
