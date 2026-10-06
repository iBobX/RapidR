// Drives a throwaway VS Code through the states the README shows
// (capture.js runs it and captures the window at each `shot(name)`).
'use strict';
const fs = require('fs');
const path = require('path');
const vscode = require('vscode');

const SIG = process.env.SHOT_SIGNALS;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function shot(name) {
    try { await vscode.commands.executeCommand('notifications.clearAll'); } catch (_) { /* */ }
    const done = path.join(SIG, 'done-' + name);
    fs.writeFileSync(path.join(SIG, 'ready-' + name), '');
    for (let i = 0; i < 150 && !fs.existsSync(done); i++) await sleep(200);
}

async function until(fn, ms = 20000) {
    const end = Date.now() + ms;
    while (Date.now() < end) {
        const v = await fn();
        if (v) return v;
        await sleep(200);
    }
    throw new Error('timed out');
}

async function open(file) {
    const doc = await vscode.workspace.openTextDocument(file);
    return vscode.window.showTextDocument(doc, { preview: false });
}

function lineOf(editor, text) {
    const i = editor.document.getText().indexOf(text);
    return editor.document.positionAt(i);
}

async function placeCaret(editor, pos) {
    editor.selection = new vscode.Selection(pos, pos);
    editor.revealRange(new vscode.Range(pos, pos), vscode.TextEditorRevealType.InCenterIfOutsideViewport);
    await sleep(300);
}

exports.run = async function () {
    const ws = vscode.workspace.workspaceFolders[0].uri.fsPath;
    const ext = vscode.extensions.getExtension('rapidr.rapidr');
    const api = await ext.activate();
    await api.whenServerReady();
    for (const c of ['workbench.action.closeAllEditors', 'workbench.action.closePanel', 'workbench.action.closeAuxiliaryBar']) {
        try { await vscode.commands.executeCommand(c); } catch (_) { /* (not in every version) */ }
    }
    await vscode.commands.executeCommand('workbench.view.explorer');
    let ed = await open(path.join(ws, 'inventory.bas'));
    const original = ed.document.getText();
    const restore = async () => {
        const doc = ed.document;
        await ed.edit((e) => e.replace(new vscode.Range(doc.positionAt(0), doc.positionAt(doc.getText().length)), original));
        await doc.save();
        await sleep(600);
    };
    try { await vscode.commands.executeCommand('notifications.clearAll'); } catch (_) { /* */ }
    await sleep(3000);

    // 1. Completion: a component's members, with their docs.
    let at = lineOf(ed, '    Form.Caption = "Inventory: "');
    await ed.edit((e) => e.insert(new vscode.Position(at.line, 0), '    ItemList.Item\n'));
    let caret = new vscode.Position(at.line, '    ItemList.Item'.length);
    await placeCaret(ed, caret);
    await vscode.commands.executeCommand('editor.action.triggerSuggest');
    await sleep(1500);
    try { await vscode.commands.executeCommand('toggleSuggestionDetails'); } catch (_) { /* */ }
    await sleep(1000);
    await shot('completion');
    await vscode.commands.executeCommand('hideSuggestWidget');
    await restore();

    // 2. Hover: the program's own FUNCTION, from its $INCLUDE file.
    at = lineOf(ed, 'PriceOf(item, 2)');
    await placeCaret(ed, at.translate(0, 3));
    await vscode.commands.executeCommand('editor.action.showHover');
    await sleep(1500);
    await shot('hover');
    await open(path.join(ws, 'stock.inc'));
    await sleep(500);
    ed = await open(path.join(ws, 'inventory.bas'));
    await sleep(500);

    // 3. Signature help while typing a call.
    at = lineOf(ed, '    Form.Caption = "Inventory: "');
    await ed.edit((e) => e.insert(new vscode.Position(at.line, 0), '    total = total + PriceOf("Pen", \n'));
    await placeCaret(ed, new vscode.Position(at.line, '    total = total + PriceOf("Pen", '.length));
    await sleep(1200);
    await vscode.commands.executeCommand('editor.action.triggerParameterHints');
    await sleep(1200);
    await vscode.commands.executeCommand('editor.action.triggerParameterHints');
    await sleep(1500);
    await shot('signature');
    await vscode.commands.executeCommand('closeParameterHints');
    await restore();

    // 4. Diagnostics: a typo, in RapidQ's words.
    at = lineOf(ed, '    total = total + PriceOf(item, 2)');
    await ed.edit((e) => e.replace(new vscode.Range(at.translate(0, 4), at.translate(0, 9)), 'totl'));
    await until(() => vscode.languages.getDiagnostics(ed.document.uri).length > 0);
    await placeCaret(ed, new vscode.Position(at.line, 0));
    await vscode.commands.executeCommand('workbench.actions.view.problems');
    await sleep(500);
    await vscode.window.showTextDocument(ed.document);
    await placeCaret(ed, new vscode.Position(at.line, 0));
    await vscode.commands.executeCommand('editor.action.marker.next');
    await sleep(1500);
    await shot('diagnostics');
    try { await vscode.commands.executeCommand('closeMarkersNavigation'); } catch (_) { /* */ }
    await restore();
    await vscode.commands.executeCommand('workbench.action.closePanel');
    await sleep(500);

    // 5. The outline.
    await vscode.commands.executeCommand('outline.focus');
    await sleep(1500);
    await shot('outline');

    // 6. Debugging: a breakpoint, the variables, a data tip.
    ed = await open(path.join(ws, 'stats.bas'));
    at = lineOf(ed, '        sum = sum + values(k)');
    vscode.debug.addBreakpoints([new vscode.SourceBreakpoint(new vscode.Location(ed.document.uri, at))]);
    let stops = 0;
    const tracker = vscode.debug.registerDebugAdapterTrackerFactory('rapidr', {
        createDebugAdapterTracker: () => ({ onDidSendMessage: (m) => { if (m.type === 'event' && m.event === 'stopped') stops++; } }),
    });
    await vscode.debug.startDebugging(vscode.workspace.workspaceFolders[0], { type: 'rapidr', request: 'launch', name: 'stats.bas', program: ed.document.uri.fsPath });
    await until(() => stops >= 1);
    for (let n = 2; n <= 3; n++) {
        await vscode.commands.executeCommand('workbench.action.debug.continue');
        await until(() => stops >= n);
    }
    await sleep(800);
    await vscode.commands.executeCommand('workbench.view.debug');
    await sleep(800);
    ed = vscode.window.activeTextEditor || ed;
    await placeCaret(ed, at.translate(0, '        su'.length));
    await sleep(500);
    await shot('debug');
    await vscode.commands.executeCommand('workbench.action.debug.stop');
    tracker.dispose();
    await sleep(1000);
    fs.writeFileSync(path.join(SIG, 'ready-last'), '');
    await sleep(1500);
};
