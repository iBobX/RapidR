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

    // 6. Debugging: a breakpoint, the variables, watches and the debug
    // console evaluating expressions in the stopped frame.
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
    await sleep(500);
    // Watches: expressions of the program, evaluated by the VM in the frame
    // (selected in the editor, "Add to Watch").
    for (const expr of ['sum + values(k)', 'sum / n', 'values(k) * 2']) {
        // (the command takes the focused editor's selection; an expression
        // the program doesn't have is typed on a scratch line, then undone)
        ed = await vscode.window.showTextDocument(ed.document, { preserveFocus: false });
        let typed = false;
        if (!ed.document.getText().includes(expr)) {
            await ed.edit((e) => e.insert(new vscode.Position(ed.document.lineCount - 1, 0), `' ${expr}\n`));
            typed = true;
        }
        const i = ed.document.getText().indexOf(expr);
        ed.selection = new vscode.Selection(ed.document.positionAt(i), ed.document.positionAt(i + expr.length));
        await sleep(200);
        await vscode.commands.executeCommand('editor.debug.action.selectionToWatch');
        await sleep(300);
        if (typed) {
            await vscode.commands.executeCommand('undo');
        }
        continue;
        await vscode.commands.executeCommand('editor.debug.action.selectionToWatch');
        await sleep(400);
    }
    // The debug console: an expression typed there is printed.
    await vscode.commands.executeCommand('workbench.debug.action.focusRepl');
    await sleep(500);
    await vscode.commands.executeCommand('type', { text: 'UCASE$("mean so far: ") + STR$(sum / (k - 1))' });
    await vscode.commands.executeCommand('repl.action.acceptInput');
    await sleep(1200);
    await vscode.window.showTextDocument(ed.document, { preserveFocus: false });
    await placeCaret(ed, at.translate(0, '        su'.length));
    await sleep(800);
    await shot('debug');
    await vscode.commands.executeCommand('workbench.action.debug.stop');
    tracker.dispose();
    await sleep(1000);
    vscode.debug.removeBreakpoints(vscode.debug.breakpoints);
    try { await vscode.commands.executeCommand('workbench.debug.viewlet.action.removeAllWatchExpressions'); } catch (_) { /* */ }

    // The states below aren't the README's (capture.js keeps them with
    // SHOTS_DIR): go to definition, rename, the registry's word, a SUB's
    // own variables while stepping.
    await vscode.commands.executeCommand('workbench.view.explorer');
    await vscode.commands.executeCommand('workbench.action.closeAllEditors');
    await vscode.commands.executeCommand('workbench.action.closePanel');
    ed = await open(path.join(ws, 'inventory.bas'));
    await sleep(800);

    // 7. Go to definition, peeked: PriceOf in its $INCLUDE file.
    at = lineOf(ed, 'PriceOf(item, 2)');
    await placeCaret(ed, at.translate(0, 3));
    await vscode.commands.executeCommand('editor.action.peekDefinition');
    await sleep(1800);
    await shot('definition');
    try { await vscode.commands.executeCommand('closeReferenceSearch'); } catch (_) { /* */ }
    await sleep(400);

    // 8. Rename `total`: every use renamed (selected, to be seen). (The
    // rename box needs the window focused, which a capture can't ask.)
    ed = await vscode.window.showTextDocument(ed.document, { preserveFocus: false });
    at = lineOf(ed, 'DIM total AS DOUBLE');
    const edit = await vscode.commands.executeCommand('vscode.executeDocumentRenameProvider', ed.document.uri, at.translate(0, 'DIM to'.length), 'grandTotal');
    await vscode.workspace.applyEdit(edit);
    await sleep(300);
    ed.selections = ed.document.getText().split('\n').flatMap((l, i) => {
        const out = [];
        for (let k = l.indexOf('grandTotal'); k >= 0; k = l.indexOf('grandTotal', k + 1)) out.push(new vscode.Selection(i, k, i, k + 'grandTotal'.length));
        return out;
    });
    await sleep(800);
    await shot('renamed');
    await restore();

    // 9. The language registry's word: a RapidQ-compatible project gets
    // RapidR's extensions reported; what RapidR doesn't have yet always is.
    await vscode.workspace.getConfiguration('rapidr').update('rapidqCompatible', true, vscode.ConfigurationTarget.Global);
    await sleep(2500);
    at = lineOf(ed, 'Form.ShowModal');
    await ed.edit((e) => e.insert(new vscode.Position(at.line, 0), 'DIM chart AS RPLOT\nForm.ShapeForm "logo.bmp", 0\nForm.Anchors = 0\nItemList.Circle 4, 4, 40, 40, 0, 0\n'));
    await until(() => vscode.languages.getDiagnostics(ed.document.uri).length >= 4, 30000);
    await vscode.commands.executeCommand('workbench.actions.view.problems');
    await sleep(800);
    await vscode.window.showTextDocument(ed.document);
    at = lineOf(ed, 'Form.ShapeForm');
    await placeCaret(ed, at.translate(0, 'Form.Sha'.length));
    await vscode.commands.executeCommand('editor.action.showHover');
    await sleep(1500);
    await shot('registry');
    await restore();
    await vscode.workspace.getConfiguration('rapidr').update('rapidqCompatible', undefined, vscode.ConfigurationTarget.Global);
    await vscode.commands.executeCommand('workbench.action.closePanel');
    await sleep(1500);

    // 10. A member's hover from the registry: type, default, docs.
    // (another editor first: the last hover goes)
    await open(path.join(ws, 'stock.inc'));
    await sleep(500);
    ed = await open(path.join(ws, 'inventory.bas'));
    await sleep(800);
    at = lineOf(ed, 'Form.Caption = "Inventory: "');
    await placeCaret(ed, at.translate(0, 'Form.Cap'.length));
    await vscode.commands.executeCommand('editor.action.showHover');
    await sleep(1500);
    await shot('hover-member');

    // 11. Debugging a SUB with its own variables: stop, step, a watch,
    // Locals (n, its STATIC calls and its own last) and Globals (Total).
    ed = await open(path.join(ws, 'counter.bas'));
    at = lineOf(ed, '    Total = Total + last');
    const bp = new vscode.SourceBreakpoint(new vscode.Location(ed.document.uri, at));
    vscode.debug.addBreakpoints([bp]);
    stops = 0;
    const tracker2 = vscode.debug.registerDebugAdapterTrackerFactory('rapidr', {
        createDebugAdapterTracker: () => ({ onDidSendMessage: (m) => { if (m.type === 'event' && m.event === 'stopped') stops++; } }),
    });
    await vscode.debug.startDebugging(vscode.workspace.workspaceFolders[0], { type: 'rapidr', request: 'launch', name: 'counter.bas', program: ed.document.uri.fsPath });
    await until(() => stops >= 1);
    await vscode.commands.executeCommand('workbench.action.debug.continue');
    await until(() => stops >= 2);
    vscode.debug.removeBreakpoints([bp]);
    await vscode.commands.executeCommand('workbench.action.debug.stepOver');
    await until(() => stops >= 3);
    await sleep(600);
    await vscode.commands.executeCommand('workbench.view.debug');
    await sleep(500);
    for (const expr of ['calls * 100 + last']) {
        ed = await vscode.window.showTextDocument(ed.document, { preserveFocus: false });
        await ed.edit((e) => e.insert(new vscode.Position(ed.document.lineCount - 1, 0), `' ${expr}\n`));
        const i = ed.document.getText().indexOf(expr);
        ed.selection = new vscode.Selection(ed.document.positionAt(i), ed.document.positionAt(i + expr.length));
        await sleep(200);
        await vscode.commands.executeCommand('editor.debug.action.selectionToWatch');
        await sleep(300);
        await vscode.commands.executeCommand('undo');
    }
    // (Globals open too: the last root of the Variables tree)
    try {
        await vscode.commands.executeCommand('workbench.debug.action.focusVariablesView');
        await sleep(300);
        await vscode.commands.executeCommand('list.focusLast');
        await vscode.commands.executeCommand('list.expand');
        await sleep(800);
    } catch (_) { /* */ }
    await vscode.window.showTextDocument(ed.document, { preserveFocus: false });
    await sleep(800);
    await shot('debug-own-variables');
    await vscode.commands.executeCommand('workbench.action.debug.stop');
    tracker2.dispose();
    await sleep(1000);
    fs.writeFileSync(path.join(SIG, 'ready-last'), '');
    await sleep(1500);
};
