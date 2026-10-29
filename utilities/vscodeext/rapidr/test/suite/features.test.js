// What the extension adds around the language server, in a real VS Code
// (test/runTest.js): finding a rapidr that is new enough, the default
// formatter and format on save, the list of types after `AS `, the
// parameter names, colours and highlights, quick fixes, Help, projects
// (.rrproj) and $INCLUDE links. Asked the way VS Code's own UI asks (the
// vscode.execute…Provider commands).
'use strict';

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');
const vscode = require('vscode');
const { EXTENSION_ID, FIXTURES, activate, open, at, sleep, eventually, labels, hoverText, revert } = require('./util');

let api;
let scratch;

/** A file of its own, in a folder of its own: the fixtures are never written. */
function scratchFile(name, text) {
    const file = path.join(scratch, name);
    fs.writeFileSync(file, text);
    return file;
}

/** A shell script standing in for rapidr: `version` says what it is told, the rest goes to the real one. */
function fakeRapidr(name, version, real) {
    const body = real
        ? `#!/bin/sh\nif [ "$1" = "version" ]; then echo "RapidR ${version}"; else exec '${real}' "$@"; fi\n`
        : `#!/bin/sh\necho "RapidR ${version}"\n`;
    const file = scratchFile(name, body);
    fs.chmodSync(file, 0o755);
    return file;
}

describe('RapidR for VS Code, around the server', () => {
    before(async () => {
        api = await activate();
        scratch = fs.mkdtempSync(path.join(os.tmpdir(), 'rvsc-scratch-'));
    });

    after(() => {
        fs.rmSync(scratch, { recursive: true, force: true });
    });

    describe('finding rapidr', function () {
        before(function () {
            if (process.platform === 'win32') this.skip(); // (the stand-ins are shell scripts)
        });

        afterEach(async () => {
            await vscode.workspace.getConfiguration('rapidr').update('path', undefined, vscode.ConfigurationTarget.Global);
            await eventually(() => api.rapidr().path && api.rapidr().state === 'current', 'rapidr found again');
            await sleep(300);
            await api.whenServerReady();
        });

        it('runs a rapidr as new as the extension: its version is known, the state is current', () => {
            const r = api.rapidr();
            assert.match(r.version, /^\d+\.\d+\.\d+/);
            assert.strictEqual(r.state, 'current');
            assert.deepStrictEqual(r.skipped, []);
        });

        it('doesn\'t use a rapidr too old for the language server (0.1.0), says so, and offers the installed one', async () => {
            const old = fakeRapidr('rapidr-0.1.0', '0.1.0');
            await vscode.workspace.getConfiguration('rapidr').update('path', old, vscode.ConfigurationTarget.Global);
            const r = await eventually(() => {
                const x = api.rapidr();
                return x.path === null && x.error ? x : null;
            }, 'the too-old rapidr rejected');
            assert.match(r.error, /0\.1\.0 is too old/, r.error);
            assert.ok(r.skipped.some((s) => s.path === old && s.version === '0.1.0'));
            assert.ok(r.alternative, 'the automatic search finds a rapidr to offer');
            assert.strictEqual(r.alternative.path, process.env.RAPIDR_PATH);
            assert.strictEqual(r.alternative.state, 'current');
        });

        it('takes a rapidr older than the extension that still has the language server, and the status says so', async () => {
            const real = process.env.RAPIDR_PATH;
            const older = fakeRapidr('rapidr-2.118', '2.118.0', real);
            await vscode.workspace.getConfiguration('rapidr').update('path', older, vscode.ConfigurationTarget.Global);
            const r = await eventually(() => {
                const x = api.rapidr();
                return x.path === older ? x : null;
            }, 'the older rapidr in use');
            assert.strictEqual(r.state, 'older');
            assert.strictEqual(r.version, '2.118.0');
            // (and it really serves: the language server answers through it)
            await api.whenServerReady();
            const doc = await open('main.bas');
            const hover = await eventually(async () => {
                const t = hoverText(await vscode.commands.executeCommand('vscode.executeHoverProvider', doc.uri, at(doc, 'MID$(', 1)));
                return /MID\$/i.test(t) ? t : null;
            }, 'hover through the older rapidr');
            assert.ok(hover);
        });
    });

    describe('formatting', () => {
        const messy = 'dim   x as integer\nif x=1 then\nprint   "a"\nend if\n';

        it('is the default formatter for RapidR files, under this extension\'s own id', () => {
            const pkg = vscode.extensions.getExtension(EXTENSION_ID).packageJSON;
            assert.strictEqual(pkg.contributes.configurationDefaults['[rapidr]']['editor.defaultFormatter'], `${pkg.publisher}.${pkg.name}`);
            assert.strictEqual(vscode.workspace.getConfiguration('editor', { languageId: 'rapidr' }).get('defaultFormatter'), EXTENSION_ID);
        });

        it('Format Document uses it without asking, and cases keywords and indents blocks', async () => {
            const file = scratchFile('messy.bas', messy);
            const doc = await open(file);
            await vscode.commands.executeCommand('editor.action.formatDocument');
            await eventually(() => doc.getText() !== messy, 'the document formatted');
            assert.strictEqual(doc.getText(), 'DIM   x AS INTEGER\nIF x=1 THEN\n    PRINT   "a"\nEND IF\n');
        });

        it('formats on save when editor.formatOnSave is on for RapidR', async () => {
            const cfg = vscode.workspace.getConfiguration('editor', { languageId: 'rapidr' });
            await cfg.update('formatOnSave', true, vscode.ConfigurationTarget.Global, true);
            try {
                const file = scratchFile('onsave.bas', 'dim a as integer\n');
                const doc = await open(file);
                const editor = vscode.window.activeTextEditor;
                await editor.edit((e) => e.insert(new vscode.Position(1, 0), 'for a = 1 to 3\nprint a\nnext a\n'));
                await doc.save();
                await eventually(() => fs.readFileSync(file, 'utf8').includes('PRINT a'), 'the saved file formatted');
                assert.strictEqual(fs.readFileSync(file, 'utf8'), 'DIM a AS INTEGER\nFOR a = 1 TO 3\n    PRINT a\nNEXT a\n');
            } finally {
                await cfg.update('formatOnSave', undefined, vscode.ConfigurationTarget.Global, true);
            }
        });
    });

    describe('typing', () => {
        it('lists the types and components after `DIM test AS `', async () => {
            const doc = await open(scratchFile('types-space.bas', "' the line ends with a space\n$APPTYPE GUI\nDIM test AS \n"));
            const pos = new vscode.Position(2, 'DIM test AS '.length);
            const list = await eventually(async () => {
                const l = await vscode.commands.executeCommand('vscode.executeCompletionItemProvider', doc.uri, pos);
                return labels(l).includes('INTEGER') ? l : null;
            }, 'completion after AS');
            for (const name of ['STRING', 'DOUBLE', 'RForm', 'RButton']) assert.ok(labels(list).includes(name), `no ${name} in ${labels(list).slice(0, 30).join(', ')}`);
        });

        it('opens that list by itself when the space after AS is typed, and not after other words', async () => {
            const doc = await open('types.bas');
            const editor = vscode.window.activeTextEditor;
            const end = doc.lineAt(doc.lineCount - 1).range.end;
            editor.selection = new vscode.Selection(end, end);
            const before = api.typeSuggestions();
            try {
                await editor.edit((e) => e.insert(end, '\nDIM other'));
                const tail = doc.lineAt(doc.lineCount - 1).range.end;
                editor.selection = new vscode.Selection(tail, tail);
                await vscode.commands.executeCommand('type', { text: ' ' });
                await sleep(200);
                assert.strictEqual(api.typeSuggestions(), before, 'a space after a name is not asking for types');
                for (const ch of 'AS ') {
                    await vscode.commands.executeCommand('type', { text: ch });
                    await sleep(100);
                }
                await eventually(() => api.typeSuggestions() === before + 1, 'the list opened after "AS "', 3000);
                await vscode.commands.executeCommand('hideSuggestWidget');
            } finally {
                await revert(doc);
            }
        });
    });

    describe('parameter names, colours and highlights', () => {
        let doc;
        before(async () => {
            doc = await open('hints.bas');
        });

        it('shows the parameter\'s name before a literal argument', async () => {
            const line = at(doc, 'DrawDot x, 40, 12').line;
            const hints = await eventually(async () => {
                const h = await vscode.commands.executeCommand('vscode.executeInlayHintProvider', doc.uri, new vscode.Range(0, 0, doc.lineCount, 0));
                return h && h.length >= 2 ? h : null;
            }, 'inlay hints');
            const label = (h) => (typeof h.label === 'string' ? h.label : h.label.map((p) => p.value).join(''));
            assert.deepStrictEqual(hints.map((h) => [h.position.line, label(h)]), [[line, 'y:'], [line, 'r:']]);
            assert.strictEqual(hints[0].kind, vscode.InlayHintKind.Parameter);
        });

        it('shows the colours the program writes, and the picker writes one the way the program does', async () => {
            const colors = await eventually(async () => {
                const c = await vscode.commands.executeCommand('vscode.executeDocumentColorProvider', doc.uri);
                return c && c.length >= 3 ? c : null;
            }, 'document colours');
            assert.strictEqual(colors.length, 3);
            const [rgb, hex, named] = colors;
            assert.strictEqual(doc.getText(rgb.range), 'RGB(255, 130, 20)');
            assert.ok(Math.abs(rgb.color.green - 130 / 255) < 0.01 && rgb.color.red === 1);
            assert.strictEqual(doc.getText(hex.range), '&H0000FF');
            assert.deepStrictEqual([hex.color.red, hex.color.green, hex.color.blue], [1, 0, 0], '&HBBGGRR: blue in the high byte');
            assert.strictEqual(doc.getText(named.range), 'clRed');
            const picked = new vscode.Color(0, 1, 0, 1);
            const forHex = await vscode.commands.executeCommand('vscode.executeColorPresentationProvider', picked, { uri: doc.uri, range: hex.range });
            assert.deepStrictEqual(forHex.map((p) => p.label), ['&H00FF00', 'RGB(0, 255, 0)'], 'the program\'s own form first');
            const forRgb = await vscode.commands.executeCommand('vscode.executeColorPresentationProvider', picked, { uri: doc.uri, range: rgb.range });
            assert.strictEqual(forRgb[0].label, 'RGB(0, 255, 0)');
        });

        it('highlights every use of the name under the caret', async () => {
            const at0 = at(doc, 'DIM total', 'DIM to'.length);
            const marks = await eventually(async () => {
                const m = await vscode.commands.executeCommand('vscode.executeDocumentHighlights', doc.uri, at0);
                return m && m.length >= 3 ? m : null;
            }, 'document highlights');
            assert.strictEqual(marks.length, 4, 'the DIM, `total = total + 1` (two) and `Str$(total)`');
            assert.ok(marks.every((m) => doc.getText(m.range).toLowerCase() === 'total'));
        });

        it('colours names by what they are (semantic tokens), with a legend', async () => {
            const legend = await vscode.commands.executeCommand('vscode.provideDocumentSemanticTokensLegend', doc.uri);
            assert.ok(legend.tokenTypes.includes('function') && legend.tokenTypes.includes('variable'));
            const tokens = await eventually(async () => {
                const t = await vscode.commands.executeCommand('vscode.provideDocumentSemanticTokens', doc.uri);
                return t && t.data.length > 0 ? t : null;
            }, 'semantic tokens');
            assert.strictEqual(tokens.data.length % 5, 0);
        });

        it('links an $INCLUDE to the file', async () => {
            const links = await vscode.commands.executeCommand('vscode.executeLinkProvider', doc.uri);
            const inc = links.find((l) => l.target && path.basename(l.target.fsPath) === 'helpers.inc');
            assert.ok(inc, `links: ${links.map((l) => l.target && l.target.fsPath).join(', ')}`);
            assert.strictEqual(doc.getText(inc.range), 'helpers.inc');
        });
    });

    describe('quick fixes', () => {
        it('offers the fix for NOT FILEEXISTS(x), which is always true', async () => {
            const file = scratchFile('fixes.bas', '$APPTYPE CONSOLE\nIF NOT FILEEXISTS("x.txt") THEN PRINT "missing"\n');
            const doc = await open(file);
            const diag = await eventually(() => vscode.languages.getDiagnostics(doc.uri).find((d) => /FILEEXISTS gives 1/.test(d.message)), 'the warning');
            const actions = await vscode.commands.executeCommand('vscode.executeCodeActionProvider', doc.uri, diag.range, vscode.CodeActionKind.QuickFix.value);
            const fix = actions.find((a) => /FILEEXISTS/.test(a.title));
            assert.ok(fix, `actions: ${actions.map((a) => a.title).join(' | ')}`);
            assert.ok(await vscode.workspace.applyEdit(fix.edit));
            assert.match(doc.getText(), /IF FILEEXISTS\("x.txt"\) = 0 THEN/);
        });
    });

    describe('Help', () => {
        let doc;
        before(async () => {
            doc = await open('hints.bas');
        });

        const help = () => api.help();

        it('adds "Open in RapidR Help" to a hover', async () => {
            const main = await open('main.bas');
            const text = await eventually(async () => {
                const t = hoverText(await vscode.commands.executeCommand('vscode.executeHoverProvider', main.uri, at(main, 'MID$(', 1)));
                return /MID\$/i.test(t) ? t : null;
            }, 'hover on MID$');
            assert.match(text, /Open in RapidR Help/);
            assert.match(text, /command:rapidr\.help\?/);
        });

        it('shows the page of the word at the caret: a property set in a CREATE body is its component\'s', async () => {
            const editor = await vscode.window.showTextDocument(doc);
            const p = at(doc, 'Caption = "Go"', 3);
            editor.selection = new vscode.Selection(p, p);
            await vscode.commands.executeCommand('rapidr.help');
            const s = help().state;
            assert.strictEqual(s.kind, 'entry');
            assert.strictEqual(s.title, 'RButton.Caption');
            assert.match(s.html, /<a href="#" data-link="help:RButton">RButton<\/a>/);
            assert.match(s.markdown, /Property of RButton/);
        });

        it('follows a link to another entry, and Back returns', async () => {
            await help().message({ type: 'link', target: 'help:OnClick@RButton' });
            assert.strictEqual(help().state.title, 'RButton.OnClick');
            await help().message({ type: 'link', target: 'help:RButton' });
            const s = help().state;
            assert.strictEqual(s.title, 'RButton');
            assert.match(s.html, /<h3>Properties \(\d+\)<\/h3>/);
            assert.match(s.html, /data-link="help:Caption@RButton"/);
            await help().message({ type: 'back' });
            assert.strictEqual(help().state.title, 'RButton.OnClick');
            await help().message({ type: 'back' });
            assert.strictEqual(help().state.title, 'RButton.Caption');
        });

        it('starts with a page that names the key to ask for Help with (VS Code\'s own F1 is the Command Palette)', async () => {
            await help().show({ word: '' });
            assert.strictEqual(help().state.kind, 'start');
            assert.match(help().state.html, /press <strong>Shift\+F1<\/strong>/);
        });

        it('searches by a typed word', async () => {
            await help().message({ type: 'search', query: 'rbutton' });
            assert.strictEqual(help().state.title, 'RButton');
            await help().message({ type: 'search', query: 'RButon' });
            assert.strictEqual(help().state.kind, 'none');
            assert.match(help().state.html, /Did you mean/);
        });

        it('shows one of the program\'s own names with where it is declared, and goes there', async () => {
            const editor = await vscode.window.showTextDocument(doc);
            const use = at(doc, 'total = total + 1', 1);
            editor.selection = new vscode.Selection(use, use);
            await vscode.commands.executeCommand('rapidr.help');
            const s = help().state;
            assert.strictEqual(s.kind, 'mine');
            assert.strictEqual(s.title, 'total');
            assert.match(s.markdown, /DIM total AS INTEGER/);
            await vscode.window.showTextDocument(await vscode.workspace.openTextDocument(path.join(FIXTURES, 'main.bas')));
            await help().message({ type: 'link', target: 'help-goto:' });
            const now = vscode.window.activeTextEditor;
            assert.strictEqual(path.basename(now.document.fileName), 'hints.bas');
            assert.strictEqual(now.selection.active.line, at(doc, 'DIM total').line);
        });

        it('inserts the syntax into the code', async () => {
            const editor = await vscode.window.showTextDocument(doc);
            const end = doc.lineAt(doc.lineCount - 1).range.end;
            editor.selection = new vscode.Selection(end, end);
            try {
                await help().show({ word: 'RButton' });
                assert.match(help().state.markdown, /\[Insert into the code\]\(help-insert:\)/);
                await help().message({ type: 'link', target: 'help-insert:' });
                await eventually(() => doc.getText().endsWith('CREATE Button1 AS RButton\n\nEND CREATE'), 'the syntax inserted');
            } finally {
                await revert(doc);
            }
        });

        it('only follows what a page may: help links and https, nothing else', async () => {
            const before = vscode.window.activeTextEditor && vscode.window.activeTextEditor.document.fileName;
            await help().message({ type: 'link', target: 'file:///etc/passwd' });
            await help().message({ type: 'link', target: 'command:workbench.action.closeAllEditors' });
            await help().message({ type: 'nonsense' });
            await help().message(null);
            assert.strictEqual(vscode.window.activeTextEditor && vscode.window.activeTextEditor.document.fileName, before);
        });
    });

    describe('projects (.rrproj)', () => {
        const project = vscode.Uri.file(path.join(FIXTURES, 'Demo.rrproj'));

        it('opens a .rrproj as a RapidR project', async () => {
            const doc = await vscode.workspace.openTextDocument(project);
            assert.strictEqual(doc.languageId, 'rapidr-project');
        });

        it('opens the project\'s main file', async () => {
            await vscode.commands.executeCommand('rapidr.openProjectMain', project);
            assert.strictEqual(path.basename(vscode.window.activeTextEditor.document.fileName), 'hello.bas');
        });

        it('debugs the project: the main file, the project\'s arguments, its folder', async () => {
            const launches = [];
            const tracker = vscode.debug.registerDebugAdapterTrackerFactory('rapidr', {
                createDebugAdapterTracker: () => ({
                    onWillReceiveMessage: (m) => {
                        if (m.type === 'request' && m.command === 'launch') launches.push(m.arguments);
                    },
                }),
            });
            try {
                await vscode.commands.executeCommand('rapidr.debugProject', project);
                const launch = await eventually(() => launches[0], 'the launch request');
                assert.strictEqual(launch.program, path.join(FIXTURES, 'hello.bas'));
                assert.deepStrictEqual(launch.args, ['one', 'two words']);
                assert.strictEqual(path.resolve(launch.cwd), FIXTURES);
                await eventually(() => !vscode.debug.activeDebugSession, 'the program to end', 30000);
            } finally {
                tracker.dispose();
            }
        });

        it('runs the project in the "RapidR" terminal', async () => {
            await vscode.commands.executeCommand('rapidr.runProject', project);
            const term = await eventually(() => vscode.window.terminals.find((t) => t.name === 'RapidR' && t.exitStatus === undefined), 'the RapidR terminal', 5000);
            assert.ok(term);
        });
    });
});
