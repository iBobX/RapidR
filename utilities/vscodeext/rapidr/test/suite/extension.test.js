// The extension end to end, in a real VS Code (test/runTest.js): what a user
// gets through `rapidr lsp` — completion, hover, signature help, definition,
// references, rename, outline, formatting, diagnostics — asked the way VS
// Code's own UI asks (the vscode.execute…Provider commands), on the fixture
// workspace (test/fixtures/workspace).
'use strict';

const assert = require('assert');
const path = require('path');
const vscode = require('vscode');

const FIXTURES = path.resolve(__dirname, '..', 'fixtures', 'workspace');
const EXTENSION_ID = 'rapidr.rapidr';

let api;

async function open(name) {
    const doc = await vscode.workspace.openTextDocument(path.join(FIXTURES, name));
    await vscode.window.showTextDocument(doc);
    return doc;
}

/** The position `offset` characters into the first `text` in the document. */
function at(doc, text, offset = 0) {
    const i = doc.getText().indexOf(text);
    assert.ok(i >= 0, `"${text}" is not in ${doc.fileName}`);
    return doc.positionAt(i + offset);
}

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
        await new Promise((r) => setTimeout(r, 250));
    }
}

function labels(list) {
    return (list ? list.items : []).map((i) => (typeof i.label === 'string' ? i.label : i.label.label));
}

function hasLabel(list, name) {
    return labels(list).some((l) => l.toUpperCase() === name.toUpperCase());
}

function hoverText(hovers) {
    return (hovers || [])
        .flatMap((h) => h.contents)
        .map((c) => (typeof c === 'string' ? c : c.value || ''))
        .join('\n');
}

function flattenSymbols(symbols) {
    const out = [];
    const walk = (list) => {
        for (const s of list || []) {
            out.push(s.name);
            walk(s.children);
        }
    };
    walk(symbols);
    return out;
}

function targetUri(loc) {
    return loc.targetUri || loc.uri;
}

describe('RapidR for VS Code', () => {
    before(async () => {
        const ext = vscode.extensions.getExtension(EXTENSION_ID);
        assert.ok(ext, `${EXTENSION_ID} is not loaded`);
        api = await ext.activate();
        assert.ok(api.rapidr().path, `rapidr was not found: ${api.rapidr().error}`);
        await api.whenServerReady();
    });

    describe('contributions', () => {
        it('opens .bas and .inc files as RapidR', async () => {
            assert.strictEqual((await open('main.bas')).languageId, 'rapidr');
            assert.strictEqual((await open('helpers.inc')).languageId, 'rapidr');
        });

        it('registers the rapidr debug type, breakpoints and launch configuration', () => {
            const pkg = vscode.extensions.getExtension(EXTENSION_ID).packageJSON;
            const dbg = pkg.contributes.debuggers.find((d) => d.type === 'rapidr');
            assert.ok(dbg, 'no "rapidr" debugger');
            assert.deepStrictEqual(dbg.languages, ['rapidr']);
            assert.ok(pkg.contributes.breakpoints.some((b) => b.language === 'rapidr'));
            const launch = dbg.configurationAttributes.launch.properties;
            for (const k of ['program', 'args', 'cwd', 'stopOnEntry', 'noDebug', 'env']) assert.ok(launch[k], `launch has no ${k}`);
        });

        it('registers its commands', async () => {
            const all = await vscode.commands.getCommands(true);
            for (const c of ['rapidr.run', 'rapidr.debug', 'rapidr.buildNative', 'rapidr.buildStandalone', 'rapidr.bundleWeb',
                'rapidr.restartServer', 'rapidr.showOutput', 'rapidr.locate']) {
                assert.ok(all.includes(c), `${c} is not registered`);
            }
        });

        it('knows the rapidr it runs', () => {
            assert.match(api.rapidr().version, /^\d+\.\d+/);
        });

        it('runs a program in the "RapidR" terminal', async () => {
            await open('hello.bas');
            await vscode.commands.executeCommand('rapidr.run');
            const term = await eventually(() => vscode.window.terminals.find((t) => t.name === 'RapidR'), 'the RapidR terminal', 5000);
            assert.ok(term);
            // (running again replaces it: one RapidR terminal)
            await vscode.commands.executeCommand('rapidr.run');
            await new Promise((r) => setTimeout(r, 500));
            assert.strictEqual(vscode.window.terminals.filter((t) => t.name === 'RapidR' && t.exitStatus === undefined).length, 1);
        });
    });

    describe('IntelliSense (rapidr lsp)', () => {
        let doc;
        before(async () => {
            doc = await open('main.bas');
        });

        it('completes a component\'s members after "Form."', async () => {
            const pos = at(doc, 'Form.Caption', 'Form.'.length);
            const list = await eventually(async () => {
                const l = await vscode.commands.executeCommand('vscode.executeCompletionItemProvider', doc.uri, pos, '.');
                return hasLabel(l, 'Caption') ? l : null;
            }, 'completion after Form.');
            assert.ok(hasLabel(list, 'ShowModal'), `no ShowModal in ${labels(list).slice(0, 40).join(', ')}`);
        });

        it('completes builtins, the program\'s SUBs and its variables', async () => {
            const line = at(doc, 'counter = counter + 1').line + 1; // (the blank line in SUB Greet)
            const pos = new vscode.Position(line, 0);
            const list = await eventually(async () => {
                const l = await vscode.commands.executeCommand('vscode.executeCompletionItemProvider', doc.uri, pos);
                return hasLabel(l, 'MID$') ? l : null;
            }, 'completion of MID$');
            assert.ok(hasLabel(list, 'Greet'), 'the SUB Greet is not offered');
            assert.ok(hasLabel(list, 'counter'), 'the variable counter is not offered');
            assert.ok(hasLabel(list, 'Shout'), 'the FUNCTION Shout (from helpers.inc) is not offered');
        });

        it('hovers a builtin', async () => {
            const pos = at(doc, 'MID$(', 1);
            const text = await eventually(async () => {
                const t = hoverText(await vscode.commands.executeCommand('vscode.executeHoverProvider', doc.uri, pos));
                return /MID\$/i.test(t) ? t : null;
            }, 'hover on MID$');
            assert.ok(text.length > 4);
        });

        it('hovers the program\'s own FUNCTION', async () => {
            const pos = at(doc, 'Shout(', 2);
            const text = await eventually(async () => {
                const t = hoverText(await vscode.commands.executeCommand('vscode.executeHoverProvider', doc.uri, pos));
                return /Shout/i.test(t) ? t : null;
            }, 'hover on Shout');
            assert.match(text, /STRING/i);
        });

        it('helps with a call\'s parameters', async () => {
            const pos = at(doc, 'MID$(', 'MID$('.length);
            const help = await eventually(
                () => vscode.commands.executeCommand('vscode.executeSignatureHelpProvider', doc.uri, pos, '('),
                'signature help in MID$(');
            assert.ok(help.signatures.length > 0);
            assert.match(help.signatures[help.activeSignature || 0].label, /MID\$/i);
        });

        it('goes to a definition in an $INCLUDEd file', async () => {
            const pos = at(doc, 'Shout(', 2);
            const locs = await eventually(async () => {
                const l = await vscode.commands.executeCommand('vscode.executeDefinitionProvider', doc.uri, pos);
                return l && l.length > 0 ? l : null;
            }, 'definition of Shout');
            const uri = targetUri(locs[0]);
            assert.strictEqual(path.basename(uri.fsPath), 'helpers.inc');
            const range = locs[0].targetSelectionRange || locs[0].targetRange || locs[0].range;
            assert.strictEqual(range.start.line, 2, 'not FUNCTION Shout\'s line');
        });

        it('finds references', async () => {
            const pos = at(doc, 'DIM counter', 'DIM c'.length);
            const refs = await eventually(async () => {
                const r = await vscode.commands.executeCommand('vscode.executeReferenceProvider', doc.uri, pos);
                return r && r.length >= 4 ? r : null;
            }, 'references of counter');
            assert.ok(refs.every((r) => path.basename(r.uri.fsPath) === 'main.bas'));
        });

        it('renames a variable everywhere', async () => {
            const pos = at(doc, 'DIM counter', 'DIM c'.length);
            const edit = await eventually(
                () => vscode.commands.executeCommand('vscode.executeDocumentRenameProvider', doc.uri, pos, 'clicks'),
                'rename of counter');
            const edits = edit.get(doc.uri);
            assert.strictEqual(edits.length, 4, `${edits.length} edits`);
            assert.ok(edits.every((e) => e.newText === 'clicks'));
        });

        it('outlines the program', async () => {
            const symbols = await eventually(async () => {
                const s = await vscode.commands.executeCommand('vscode.executeDocumentSymbolProvider', doc.uri);
                return s && s.length > 0 ? s : null;
            }, 'document symbols');
            const names = flattenSymbols(symbols).map((n) => n.toUpperCase());
            for (const n of ['GREET', 'FORM', 'GREETBUTTON', 'COUNTER']) {
                assert.ok(names.some((x) => x === n || x.startsWith(n + ' ') || x.startsWith(n + '(')), `no ${n} in ${names.join(', ')}`);
            }
        });

        it('formats: only layout and letter case change', async () => {
            const edits = await vscode.commands.executeCommand('vscode.executeFormatDocumentProvider', doc.uri, { tabSize: 4, insertSpaces: true });
            assert.ok(edits === undefined || Array.isArray(edits));
            // (applied back to front on the text, without touching the document)
            let text = doc.getText();
            const sorted = [...(edits || [])].sort((a, b) => doc.offsetAt(b.range.start) - doc.offsetAt(a.range.start));
            for (const e of sorted) text = text.slice(0, doc.offsetAt(e.range.start)) + e.newText + text.slice(doc.offsetAt(e.range.end));
            const essence = (s) => s.replace(/\s+/g, '').toUpperCase();
            assert.strictEqual(essence(text), essence(doc.getText()));
        });
    });

    describe('debugging (rapidr dap)', () => {
        it('stops at a breakpoint, shows the stack and variables, evaluates, and runs to the end', async () => {
            const doc = await open('debug.bas');
            const line = at(doc, 'total = total + i').line;
            const bp = new vscode.SourceBreakpoint(new vscode.Location(doc.uri, new vscode.Position(line, 0)));
            vscode.debug.addBreakpoints([bp]);
            const events = [];
            const tracker = vscode.debug.registerDebugAdapterTrackerFactory('rapidr', {
                createDebugAdapterTracker: () => ({
                    onDidSendMessage: (m) => {
                        if (m.type === 'event') events.push(m);
                    },
                }),
            });
            try {
                const started = await vscode.debug.startDebugging(undefined, { type: 'rapidr', request: 'launch', name: 'debug.bas', program: doc.uri.fsPath });
                assert.ok(started, 'the debug session did not start');
                const stopped = await eventually(() => events.find((e) => e.event === 'stopped'), 'a stop at the breakpoint');
                const session = vscode.debug.activeDebugSession;
                const threadId = stopped.body.threadId || 1;
                const st = await session.customRequest('stackTrace', { threadId });
                const top = st.stackFrames[0];
                assert.strictEqual(top.line, line + 1, 'stopped on the breakpoint\'s line');
                assert.match(top.name, /AddUp/i);
                assert.strictEqual(path.basename(top.source.path), 'debug.bas');
                assert.ok(st.stackFrames.length >= 2, 'the main program is below AddUp');
                const scopes = await session.customRequest('scopes', { frameId: top.id });
                const locals = await session.customRequest('variables', { variablesReference: scopes.scopes[0].variablesReference });
                const names = locals.variables.map((v) => v.name.toLowerCase());
                assert.ok(names.includes('i') && names.includes('n'), `locals: ${names.join(', ')}`);
                const ev = await session.customRequest('evaluate', { expression: 'n', frameId: top.id, context: 'watch' });
                assert.strictEqual(ev.result, '3');
                // Watches are expressions, evaluated by the VM in the frame.
                const watch = (expression) => session.customRequest('evaluate', { expression, frameId: top.id, context: 'watch' }).then((r) => r.result);
                assert.strictEqual(await watch('n * 10 + i'), '31');
                assert.strictEqual(await watch('total + i'), '1');
                assert.strictEqual(await watch('"i=" + STR$(i)'), '"i=1"');
                // The debug console prints an expression, runs a statement.
                await session.customRequest('evaluate', { expression: 'total = 100', frameId: top.id, context: 'repl' });
                const repl = await session.customRequest('evaluate', { expression: 'total * 2', frameId: top.id, context: 'repl' });
                assert.strictEqual(repl.result, '200');
                vscode.debug.removeBreakpoints([bp]);
                await session.customRequest('continue', { threadId });
                await eventually(() => events.find((e) => e.event === 'terminated'), 'the end of the program');
                const out = events.filter((e) => e.event === 'output').map((e) => e.body.output).join('');
                assert.match(out, /total\s*106/, `output: ${out}`);
            } finally {
                tracker.dispose();
                vscode.debug.removeBreakpoints(vscode.debug.breakpoints);
            }
        });
    });

    describe('diagnostics (rapidr lsp)', () => {
        it('reports an error in RapidQ\'s words, on its line', async () => {
            const doc = await open('errors.bas');
            const diags = await eventually(() => {
                const d = vscode.languages.getDiagnostics(doc.uri);
                return d.length > 0 ? d : null;
            }, 'diagnostics for errors.bas');
            const d = diags.find((x) => /Undeclared identifier undeclaredThing/i.test(x.message));
            assert.ok(d, `messages: ${diags.map((x) => x.message).join(' | ')}`);
            assert.strictEqual(d.severity, vscode.DiagnosticSeverity.Error);
            assert.strictEqual(d.range.start.line, at(doc, 'undeclaredThing = 2').line);
        });

        it('sees an $INCLUDEd file as the editor has it, saved or not', async () => {
            const main = await open('unsaved.bas');
            await eventually(() => vscode.languages.getDiagnostics(main.uri).some((d) => /Triple/.test(d.message)), 'the error before the edit');
            const inc = await open('unsaved.inc');
            const editor = vscode.window.activeTextEditor;
            await editor.edit((e) => e.insert(new vscode.Position(1, 0), 'FUNCTION Triple(n AS INTEGER) AS INTEGER\n    Triple = n * 3\nEND FUNCTION\n'));
            assert.ok(inc.isDirty, 'the include is edited and not saved');
            try {
                await eventually(() => vscode.languages.getDiagnostics(main.uri).length === 0, 'no error once the include has Triple');
                const pos = at(main, 'Triple(', 2);
                const locs = await vscode.commands.executeCommand('vscode.executeDefinitionProvider', main.uri, pos);
                assert.strictEqual(path.basename(targetUri(locs[0]).fsPath), 'unsaved.inc');
                const range = locs[0].targetSelectionRange || locs[0].targetRange || locs[0].range;
                assert.strictEqual(range.start.line, 1, 'the line the editor has it on');
            } finally {
                await vscode.window.showTextDocument(inc);
                await vscode.commands.executeCommand('workbench.action.files.revert');
            }
        });

        it('reports no error in a correct program', async () => {
            const doc = await open('main.bas');
            // (give the server the time it took for errors.bas)
            await new Promise((r) => setTimeout(r, 1500));
            const errors = vscode.languages.getDiagnostics(doc.uri).filter((d) => d.severity === vscode.DiagnosticSeverity.Error);
            assert.deepStrictEqual(errors.map((d) => `${d.range.start.line + 1}: ${d.message}`), []);
        });
    });
});
