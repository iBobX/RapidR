// RapidR for VS Code: a thin client. Code completion, diagnostics, navigation,
// rename, formatting, parameter names, colours and Help come from `rapidr lsp`
// (the language server, Rust); debugging from `rapidr dap` (the debug
// adapter, Rust); run / build commands run `rapidr` in a terminal. This file
// finds a rapidr that is new enough, and wires those up.
'use strict';

const vscode = require('vscode');
const fs = require('fs');
const nodePath = require('path');
const { LanguageClient, TransportKind, RevealOutputChannelOn, ErrorAction, CloseAction } = require('vscode-languageclient/node');
const { chooseRapidr, rapidrVersion } = require('./locate');
const { majorMinor } = require('./version');
const { shellKind, commandLine } = require('./shell');
const { readProject, projectPaths } = require('./project');
const { HelpPanel } = require('./help');
const { registerMcp } = require('./mcp');
const { suggestsTypes } = require('./triggers');

const DOWNLOAD_URL = 'https://github.com/iBobX/RapidR/releases';
const LANGUAGE = 'rapidr';
const TERMINAL_NAME = 'RapidR';
const SELECTOR = [
    { language: LANGUAGE, scheme: 'file' },
    { language: LANGUAGE, scheme: 'untitled' },
];

/** What we know of rapidr: { path, version, source, state, error, skipped, alternative }. */
let rapidr = { path: null, version: null, source: null, state: 'none', error: null, skipped: [], alternative: null };
let extensionVersion = '0.0.0';
let serverError = null;
let client = null;
let serverReady = Promise.resolve();
let output;
let status;
let watcher;
let missingNotified = false;
let globalState;
let help;
let mcp = null;
let hasProject = false;
/** How many times typing `AS ` opened the list of types (the tests read it). */
let typeSuggestions = 0;

// ---------------------------------------------------------------- rapidr

/** One line of the "RapidR Language Server" output, with the time. */
function log(line) {
    if (output) output.appendLine(line);
}

async function resolveRapidr() {
    const cfg = vscode.workspace.getConfiguration('rapidr');
    const folders = (vscode.workspace.workspaceFolders || []).filter((f) => f.uri.scheme === 'file').map((f) => f.uri.fsPath);
    const setting = cfg.get('path', '');
    log(`Looking for rapidr (RapidR for VS Code ${extensionVersion}${setting ? `; rapidr.path is "${setting}"` : ''})`);
    const found = await chooseRapidr({ setting, workspaceFolders: folders, trusted: vscode.workspace.isTrusted }, extensionVersion, rapidrVersion);
    for (const s of found.skipped) log(`  not used: ${s.path}${s.version ? ` (RapidR ${s.version})` : ''}: ${s.error}`);
    rapidr = {
        path: found.path,
        version: found.version,
        source: found.source,
        state: found.state,
        error: found.error,
        skipped: found.skipped,
        alternative: found.alternative,
    };
    if (found.path) {
        log(`rapidr ${found.version}: ${found.path} (from ${found.source})`);
        if (found.state === 'older') {
            log(`  RapidR ${found.version} is older than this extension (${extensionVersion}): some features need RapidR ${majorMinor(extensionVersion).replace(/\.0$/, '')} or newer`);
        }
    } else {
        log(`RapidR was not found or is too old. ${found.error || 'Looked in: ' + (found.tried || []).join(', ')}`);
        if (found.alternative) log(`  The installed RapidR ${found.alternative.version} (${found.alternative.path}) would do: "Use the installed RapidR" switches to it.`);
    }
    await vscode.commands.executeCommand('setContext', 'rapidr.missing', !rapidr.path);
    updateStatus();
    // (not awaited: a notification stays until it is dismissed, and the server starts now)
    if (rapidr.path) announceSkipped().catch(() => {});
    if (mcp) mcp.refresh();
    return rapidr;
}

/**
 * An older rapidr was passed over for the one in use (the 0.1.0 in
 * ~/.cargo/bin first on PATH, say): said once for each such rapidr, so a
 * user who wonders why `rapidr` in a terminal says something else is told.
 */
async function announceSkipped() {
    const old = rapidr.skipped.find((s) => s.version && /too old/.test(s.error));
    if (!old || rapidr.source === 'setting') return;
    const key = `rapidr.skippedNotice.${old.path}@${old.version}`;
    if (globalState && globalState.get(key)) return;
    if (globalState) await globalState.update(key, true);
    const choice = await vscode.window.showInformationMessage(
        `RapidR ${old.version} at ${old.path} is too old for this extension (it has no language server). Using RapidR ${rapidr.version} instead: ${rapidr.path}.`,
        'Show Output',
    );
    if (choice === 'Show Output') output.show(true);
}

/**
 * "RapidR was not found" (or "too old"), with what to do about it. Once per
 * session unless `force`.
 */
async function notifyMissing(force) {
    if (missingNotified && !force) return;
    missingNotified = true;
    const alt = rapidr.alternative;
    const detail = rapidr.error ? ` ${rapidr.error}` : ' Install RapidR, or tell VS Code where rapidr is.';
    const buttons = [];
    if (alt) buttons.push('Use the installed RapidR');
    buttons.push('Choose…', 'Download');
    const tooOld = rapidr.skipped.some((s) => /too old/.test(s.error)) || /too old/.test(rapidr.error || '');
    const lead = tooOld ? 'The RapidR found is too old for this extension.' : 'RapidR was not found.';
    const choice = await vscode.window.showWarningMessage(`${lead}${detail}`, ...buttons);
    if (choice === 'Use the installed RapidR') await useAutomatic();
    else if (choice === 'Choose…') await locateRapidr();
    else if (choice === 'Download') await vscode.env.openExternal(vscode.Uri.parse(DOWNLOAD_URL));
}

/** Clears `rapidr.path`, wherever it is set, so the search finds the best rapidr by itself. */
async function useAutomatic() {
    const cfg = vscode.workspace.getConfiguration('rapidr');
    const at = cfg.inspect('path') || {};
    if (at.globalValue !== undefined) await cfg.update('path', undefined, vscode.ConfigurationTarget.Global);
    if (at.workspaceValue !== undefined) await cfg.update('path', undefined, vscode.ConfigurationTarget.Workspace);
    if (at.workspaceFolderValue !== undefined) await cfg.update('path', undefined, vscode.ConfigurationTarget.WorkspaceFolder);
}

async function locateRapidr() {
    const picked = await vscode.window.showOpenDialog({
        title: 'Locate rapidr',
        openLabel: 'Use this rapidr',
        canSelectFiles: true,
        canSelectFolders: process.platform !== 'darwin',
        canSelectMany: false,
        filters: process.platform === 'win32' ? { 'rapidr.exe': ['exe'], 'All files': ['*'] } : undefined,
    });
    if (!picked || picked.length === 0) return;
    const chosen = picked[0].fsPath;
    const found = await chooseRapidr({ setting: chosen }, extensionVersion, rapidrVersion);
    if (!found.path) {
        vscode.window.showErrorMessage(`${found.error || `${chosen} isn't rapidr (the RapidR executable).`}`);
        return;
    }
    // (the change event resolves rapidr again and restarts the server)
    await vscode.workspace.getConfiguration('rapidr').update('path', found.path, vscode.ConfigurationTarget.Global);
    vscode.window.showInformationMessage(`Using RapidR ${found.version}: ${found.path}`);
}

// ---------------------------------------------------------------- the language server

function startServer() {
    if (!rapidr.path) {
        serverReady = Promise.reject(new Error('RapidR was not found'));
        serverReady.catch(() => {});
        return serverReady;
    }
    const cfg = vscode.workspace.getConfiguration('rapidr');
    const serverOptions = {
        command: rapidr.path,
        args: ['lsp'],
        transport: TransportKind.stdio,
        options: { env: { ...process.env } },
    };
    let started = false;
    let crashes = [];
    const clientOptions = {
        documentSelector: SELECTOR,
        outputChannel: output,
        traceOutputChannel: output,
        revealOutputChannelOn: RevealOutputChannelOn.Never,
        initializationOptions: {
            rapidqCompatible: cfg.get('rapidqCompatible', false),
            keywordCase: cfg.get('keywordCase', 'upper'),
            identifierCase: cfg.get('identifierCase', 'preserve'),
        },
        synchronize: { fileEvents: watcher },
        middleware: { provideHover: provideHoverWithHelpLink },
        errorHandler: {
            error: () => ({ action: ErrorAction.Continue }),
            closed: () => {
                // (a server that never started is reported once, below)
                if (!started) return { action: CloseAction.DoNotRestart, handled: true };
                const now = Date.now();
                crashes = crashes.filter((t) => now - t < 3 * 60 * 1000).concat(now);
                if (crashes.length <= 4) {
                    log('rapidr lsp stopped: restarting it');
                    return { action: CloseAction.Restart, handled: true };
                }
                serverError = 'it stopped 5 times in 3 minutes';
                updateStatus();
                vscode.window.showErrorMessage(
                    "RapidR's language server stopped 5 times in 3 minutes and won't be restarted (RapidR: Restart Language Server tries again).",
                    'Show Output',
                ).then((choice) => choice && output.show(true));
                return { action: CloseAction.DoNotRestart, handled: true };
            },
        },
    };
    const c = new LanguageClient('rapidr', 'RapidR Language Server', serverOptions, clientOptions);
    client = c;
    serverError = null;
    serverReady = c.start().then(
        () => {
            started = true;
            log(`rapidr lsp started (${rapidr.path})`);
            updateStatus();
        },
        async (err) => {
            serverError = err && err.message ? err.message : String(err);
            log(`rapidr lsp didn't start: ${serverError}`);
            if (client === c) client = null;
            try { await c.dispose(); } catch { /* already gone */ }
            updateStatus();
            vscode.window.showErrorMessage(
                `RapidR's language server didn't start (rapidr ${rapidr.version} at ${rapidr.path}). A RapidR older than the language server has no "rapidr lsp": update it.`,
                'Show Output', 'Download',
            ).then((choice) => {
                if (choice === 'Show Output') output.show(true);
                else if (choice === 'Download') vscode.env.openExternal(vscode.Uri.parse(DOWNLOAD_URL));
            });
            throw err;
        },
    );
    serverReady.catch(() => {});
    return serverReady;
}

/** The server's hover, and a link to the whole Help page of what is under the caret. */
async function provideHoverWithHelpLink(document, position, token, next) {
    const hover = await next(document, position, token);
    if (!hover || !help || !help.supported()) return hover;
    const args = encodeURIComponent(JSON.stringify([{ uri: document.uri.toString(), line: position.line, character: position.character }]));
    const link = new vscode.MarkdownString(`[Open in RapidR Help](command:rapidr.help?${args} "The whole Help page: what it is, the syntax, its members")`);
    link.isTrusted = { enabledCommands: ['rapidr.help'] };
    hover.contents = [...hover.contents, link];
    return hover;
}

async function stopServer() {
    const c = client;
    client = null;
    if (!c) return;
    try {
        await c.stop();
    } catch (err) {
        log(`rapidr lsp: stopping: ${err.message || err}`);
    }
    try { await c.dispose(); } catch { /* stopped */ }
}

async function restartServer() {
    await stopServer();
    return startServer();
}

// ---------------------------------------------------------------- status bar

function updateStatus() {
    if (!status) return;
    const editor = vscode.window.activeTextEditor;
    const show = editor && editor.document.languageId === LANGUAGE;
    if (!rapidr.path) {
        status.text = '$(warning) RapidR';
        status.tooltip = `RapidR was not found or is too old${rapidr.error ? ': ' + rapidr.error : ''}. Click for options.`;
        status.backgroundColor = new vscode.ThemeColor('statusBarItem.warningBackground');
    } else if (serverError) {
        status.text = `$(warning) RapidR ${rapidr.version}`;
        status.tooltip = `rapidr ${rapidr.version} (${rapidr.path}): the language server didn't start. Click for options.`;
        status.backgroundColor = new vscode.ThemeColor('statusBarItem.warningBackground');
    } else if (rapidr.state === 'older') {
        status.text = `$(warning) RapidR ${rapidr.version}`;
        status.tooltip = `rapidr ${rapidr.version} (${rapidr.path}) is older than this extension (${extensionVersion}): some features need a newer RapidR. Click for options.`;
        status.backgroundColor = new vscode.ThemeColor('statusBarItem.warningBackground');
    } else {
        status.text = `$(play-circle) RapidR ${rapidr.version}`;
        status.tooltip = `rapidr ${rapidr.version}: ${rapidr.path}\nClick for RapidR's commands.`;
        status.backgroundColor = undefined;
    }
    status.accessibilityInformation = { label: String(status.text).replace(/\$\([^)]*\)\s*/g, '') + ': RapidR commands' };
    if (show) status.show();
    else status.hide();
}

async function showCommands() {
    const items = rapidr.path
        ? [
            { label: '$(play) Run File', command: 'rapidr.run' },
            { label: '$(debug-alt) Debug File', command: 'rapidr.debug' },
            { label: '$(book) Help for the Word at the Cursor', command: 'rapidr.help' },
            ...(hasProject
                ? [
                    { label: '', kind: vscode.QuickPickItemKind.Separator },
                    { label: '$(play) Run Project', command: 'rapidr.runProject' },
                    { label: '$(debug-alt) Debug Project', command: 'rapidr.debugProject' },
                    { label: '$(go-to-file) Open the Project\'s Main File', command: 'rapidr.openProjectMain' },
                ]
                : []),
            { label: '', kind: vscode.QuickPickItemKind.Separator },
            { label: '$(tools) Build Native Executable', description: 'rapidr build', command: 'rapidr.buildNative' },
            { label: '$(package) Build Standalone Executable', description: 'rapidr build --interp', command: 'rapidr.buildStandalone' },
            { label: '$(globe) Bundle for the Web', description: 'rapidr bundle-bc', command: 'rapidr.bundleWeb' },
            { label: '', kind: vscode.QuickPickItemKind.Separator },
            { label: '$(refresh) Restart Language Server', command: 'rapidr.restartServer' },
            { label: '$(output) Show Language Server Output', command: 'rapidr.showOutput' },
            { label: '$(file-binary) Locate rapidr…', description: rapidr.path, command: 'rapidr.locate' },
        ]
        : [
            ...(rapidr.alternative
                ? [{ label: `$(check) Use the installed RapidR ${rapidr.alternative.version}`, description: rapidr.alternative.path, run: useAutomatic }]
                : []),
            { label: '$(cloud-download) Download RapidR', description: DOWNLOAD_URL, run: () => vscode.env.openExternal(vscode.Uri.parse(DOWNLOAD_URL)) },
            { label: '$(file-binary) Locate rapidr…', command: 'rapidr.locate' },
            { label: '$(output) Show Language Server Output', command: 'rapidr.showOutput' },
        ];
    const picked = await vscode.window.showQuickPick(items, { title: rapidr.path ? `RapidR ${rapidr.version}` : 'RapidR was not found', placeHolder: 'RapidR' });
    if (!picked) return;
    if (picked.run) await picked.run();
    else await vscode.commands.executeCommand(picked.command);
}

// ---------------------------------------------------------------- run / build

/** The program a command acts on (the explorer's file, else the active editor's), saved. */
async function programFile(uri) {
    let doc = null;
    if (uri instanceof vscode.Uri) {
        doc = vscode.workspace.textDocuments.find((d) => d.uri.toString() === uri.toString()) || null;
        if (!doc && uri.scheme === 'file') {
            await saveRapidrDocuments();
            return uri.fsPath;
        }
    } else if (vscode.window.activeTextEditor) {
        doc = vscode.window.activeTextEditor.document;
    }
    if (!doc || doc.languageId !== LANGUAGE) {
        vscode.window.showInformationMessage('Open a RapidR program (.bas, .rr) first.');
        return null;
    }
    if (doc.isUntitled) {
        await vscode.commands.executeCommand('workbench.action.files.saveAs');
        const now = vscode.window.activeTextEditor && vscode.window.activeTextEditor.document;
        if (!now || now.isUntitled) return null;
        doc = now;
    }
    if (doc.isDirty) await doc.save();
    await saveRapidrDocuments();
    if (doc.uri.scheme !== 'file') {
        vscode.window.showErrorMessage('RapidR runs programs saved on disk.');
        return null;
    }
    return doc.uri.fsPath;
}

/** The program and the files it $INCLUDEs are read from disk: every changed RapidR file is saved. */
async function saveRapidrDocuments() {
    for (const d of vscode.workspace.textDocuments) {
        if (d.isDirty && !d.isUntitled && d.languageId === LANGUAGE) await d.save();
    }
}

/** `rapidr <args>` in the "RapidR" terminal (a new one: running again ends the previous run). */
function runInTerminal(args, cwd, focus) {
    for (const t of vscode.window.terminals) if (t.name === TERMINAL_NAME) t.dispose();
    const term = vscode.window.createTerminal({ name: TERMINAL_NAME, cwd, iconPath: new vscode.ThemeIcon('play-circle') });
    term.show(!focus);
    term.sendText(commandLine(shellKind(vscode.env.shell, process.platform), rapidr.path, args));
    return term;
}

async function withRapidr() {
    if (rapidr.path) return true;
    notifyMissing(true).catch(() => {});
    return false;
}

function terminalCommand(build) {
    return async (uri) => {
        if (!(await withRapidr())) return;
        const file = await programFile(uri);
        if (!file) return;
        const { args, focus } = build(file);
        runInTerminal(args, nodePath.dirname(file), focus);
    };
}

const runCommand = terminalCommand((file) => ({ args: ['run', file], focus: true }));
const buildNativeCommand = terminalCommand((file) => ({ args: ['build', file, '--release'] }));
const buildStandaloneCommand = terminalCommand((file) => ({ args: ['build', file, '--interp'] }));
const bundleWebCommand = terminalCommand((file) => {
    const stem = nodePath.basename(file, nodePath.extname(file));
    return { args: ['bundle-bc', file, '-o', nodePath.join(nodePath.dirname(file), `${stem}-web.zip`)] };
});

async function debugCommand(uri) {
    if (!(await withRapidr())) return;
    const file = await programFile(uri);
    if (!file) return;
    const folder = vscode.workspace.getWorkspaceFolder(vscode.Uri.file(file));
    await vscode.debug.startDebugging(folder, {
        type: LANGUAGE,
        request: 'launch',
        name: `RapidR: ${nodePath.basename(file)}`,
        program: file,
        cwd: nodePath.dirname(file),
    });
}

// ---------------------------------------------------------------- projects (.rrproj)

/** The .rrproj files of the workspace. */
async function findProjects() {
    return vscode.workspace.findFiles('**/*.rrproj', '**/{node_modules,.git,build,target}/**', 50);
}

async function refreshProjectContext() {
    hasProject = (await findProjects()).length > 0;
    await vscode.commands.executeCommand('setContext', 'rapidr.hasProject', hasProject);
}

/** The project a command acts on: the explorer's / editor's .rrproj, else the workspace's (one, or a pick). */
async function pickProject(uri) {
    if (uri instanceof vscode.Uri && uri.fsPath.toLowerCase().endsWith('.rrproj')) return uri;
    const active = vscode.window.activeTextEditor && vscode.window.activeTextEditor.document.uri;
    if (active && active.scheme === 'file' && active.fsPath.toLowerCase().endsWith('.rrproj')) return active;
    const all = await findProjects();
    if (all.length === 0) {
        vscode.window.showInformationMessage('No RapidR project (.rrproj) in this folder.');
        return null;
    }
    // (the project that holds the active file first)
    const here = active && active.scheme === 'file' ? active.fsPath : '';
    const owning = all.filter((u) => here.startsWith(nodePath.dirname(u.fsPath) + nodePath.sep));
    const choices = owning.length === 1 ? owning : all;
    if (choices.length === 1) return choices[0];
    const picked = await vscode.window.showQuickPick(
        choices.map((u) => ({ label: nodePath.basename(u.fsPath, '.rrproj'), description: vscode.workspace.asRelativePath(u), uri: u })),
        { title: 'Which RapidR project?' },
    );
    return picked ? picked.uri : null;
}

/** { file, project, paths } of a .rrproj, or null with the reason shown. */
async function loadProject(uri) {
    let text;
    try {
        text = Buffer.from(await vscode.workspace.fs.readFile(uri)).toString('utf8');
    } catch (err) {
        vscode.window.showErrorMessage(`${uri.fsPath} can't be read: ${err.message}`);
        return null;
    }
    const project = readProject(text);
    const paths = projectPaths(uri.fsPath, project);
    if (!project.main) {
        vscode.window.showErrorMessage(`${nodePath.basename(uri.fsPath)} names no main file (main = "…").`);
        return null;
    }
    if (!fs.existsSync(paths.main)) {
        vscode.window.showErrorMessage(`The project's main file ${paths.main} doesn't exist.`);
        return null;
    }
    return { file: uri.fsPath, project, paths };
}

async function runProjectCommand(uri) {
    if (!(await withRapidr())) return;
    const picked = await pickProject(uri);
    const loaded = picked && (await loadProject(picked));
    if (!loaded) return;
    await saveRapidrDocuments();
    runInTerminal(['run', loaded.paths.main, ...loaded.project.args], loaded.paths.cwd, true);
}

async function debugProjectCommand(uri) {
    if (!(await withRapidr())) return;
    const picked = await pickProject(uri);
    const loaded = picked && (await loadProject(picked));
    if (!loaded) return;
    await saveRapidrDocuments();
    const folder = vscode.workspace.getWorkspaceFolder(vscode.Uri.file(loaded.file));
    await vscode.debug.startDebugging(folder, {
        type: LANGUAGE,
        request: 'launch',
        name: `RapidR: ${loaded.project.name || nodePath.basename(loaded.file, '.rrproj')}`,
        program: loaded.paths.main,
        args: loaded.project.args,
        cwd: loaded.paths.cwd,
    });
}

async function buildProjectCommand(uri) {
    if (!(await withRapidr())) return;
    const picked = await pickProject(uri);
    const loaded = picked && (await loadProject(picked));
    if (!loaded) return;
    await saveRapidrDocuments();
    runInTerminal(['build', loaded.file], loaded.paths.dir, false);
}

async function openProjectMainCommand(uri) {
    const picked = await pickProject(uri);
    const loaded = picked && (await loadProject(picked));
    if (!loaded) return;
    await vscode.window.showTextDocument(await vscode.workspace.openTextDocument(loaded.paths.main));
}

// ---------------------------------------------------------------- typing

/**
 * `DIM test AS ` + a space: the list of types and components opens by itself
 * (the server's trigger characters are `.` and `$`; a space is not one).
 * Not when the editor's own trigger characters are off.
 */
function suggestAfterAs(e) {
    const doc = e.document;
    if (doc.languageId !== LANGUAGE || e.contentChanges.length !== 1) return;
    const change = e.contentChanges[0];
    if (change.text !== ' ' || change.rangeLength !== 0) return;
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document !== doc) return;
    if (!vscode.workspace.getConfiguration('editor', doc).get('suggestOnTriggerCharacters', true)) return;
    const at = change.range.start;
    if (!suggestsTypes(doc.lineAt(at.line).text.slice(0, at.character + 1))) return;
    typeSuggestions++;
    vscode.commands.executeCommand('editor.action.triggerSuggest');
}

// ---------------------------------------------------------------- $INCLUDE links

/** `$INCLUDE "file"` is a link to the file (beside the program, or in the workspace). */
const includeLinks = {
    provideDocumentLinks(document) {
        if (document.uri.scheme !== 'file') return [];
        const dirs = [nodePath.dirname(document.uri.fsPath), ...(vscode.workspace.workspaceFolders || []).filter((f) => f.uri.scheme === 'file').map((f) => f.uri.fsPath)];
        const links = [];
        for (let i = 0; i < document.lineCount; i++) {
            const text = document.lineAt(i).text;
            const m = /^\s*\$INCLUDE\s+"([^"]+)"/i.exec(text);
            if (!m) continue;
            const name = m[1];
            const target = dirs.map((d) => nodePath.resolve(d, name)).find((p) => fs.existsSync(p) && fs.statSync(p).isFile());
            if (!target) continue;
            const start = text.indexOf(name, m[0].indexOf('"'));
            const link = new vscode.DocumentLink(new vscode.Range(i, start, i, start + name.length), vscode.Uri.file(target));
            link.tooltip = 'Open the included file';
            links.push(link);
        }
        return links;
    },
};

// ---------------------------------------------------------------- debugging (rapidr dap)

const CURRENT_FILE = { type: LANGUAGE, request: 'launch', name: 'RapidR: current file', program: '${file}' };

const configurationProvider = {
    provideDebugConfigurations() {
        return [{ ...CURRENT_FILE }];
    },
    resolveDebugConfiguration(folder, config) {
        // F5 with no launch.json: the active editor's program
        if (!config.type && !config.request && !config.name) {
            const editor = vscode.window.activeTextEditor;
            if (editor && editor.document.languageId === LANGUAGE) Object.assign(config, CURRENT_FILE);
        }
        if (!config.program) {
            vscode.window.showInformationMessage('Open a RapidR program (.bas, .rr) to debug it, or give the launch configuration a "program".');
            return undefined;
        }
        return config;
    },
    async resolveDebugConfigurationWithSubstitutedVariables(folder, config) {
        if (!rapidr.path) {
            notifyMissing(true).catch(() => {});
            return undefined;
        }
        if (!nodePath.isAbsolute(config.program) && folder && folder.uri.scheme === 'file') {
            config.program = nodePath.join(folder.uri.fsPath, config.program);
        }
        if (!config.cwd) config.cwd = nodePath.dirname(config.program);
        return config;
    },
};

const dynamicConfigurationProvider = {
    provideDebugConfigurations() {
        const editor = vscode.window.activeTextEditor;
        if (!editor || editor.document.languageId !== LANGUAGE || editor.document.isUntitled) return [];
        return [{ ...CURRENT_FILE, name: `RapidR: ${nodePath.basename(editor.document.uri.fsPath)}`, program: editor.document.uri.fsPath }];
    },
};

const adapterFactory = {
    createDebugAdapterDescriptor() {
        if (!rapidr.path) throw new Error('RapidR was not found: set rapidr.path, or install RapidR (' + DOWNLOAD_URL + ').');
        return new vscode.DebugAdapterExecutable(rapidr.path, ['dap']);
    },
};

// ---------------------------------------------------------------- activation

async function activate(context) {
    extensionVersion = context.extension.packageJSON.version;
    globalState = context.globalState;
    output = vscode.window.createOutputChannel('RapidR Language Server');
    status = vscode.window.createStatusBarItem('rapidr.status', vscode.StatusBarAlignment.Left, 50);
    status.name = 'RapidR';
    status.command = 'rapidr.showCommands';
    // ($INCLUDE files changed outside the editor: one watcher for every client)
    watcher = vscode.workspace.createFileSystemWatcher('**/*.{rr,bas,inc,RR,BAS,INC}');
    const projectWatcher = vscode.workspace.createFileSystemWatcher('**/*.rrproj');
    help = new HelpPanel(() => client, log);
    context.subscriptions.push(output, status, watcher, projectWatcher, help);
    // (experimental, off by default: `rapidr mcp` for the editor's AI agents; src/mcp.js)
    mcp = registerMcp(context, { getRapidr: () => rapidr, log });

    const command = (id, fn) => context.subscriptions.push(vscode.commands.registerCommand(id, fn));
    command('rapidr.run', runCommand);
    command('rapidr.debug', debugCommand);
    command('rapidr.buildNative', buildNativeCommand);
    command('rapidr.buildStandalone', buildStandaloneCommand);
    command('rapidr.bundleWeb', bundleWebCommand);
    command('rapidr.runProject', runProjectCommand);
    command('rapidr.debugProject', debugProjectCommand);
    command('rapidr.buildProject', buildProjectCommand);
    command('rapidr.openProjectMain', openProjectMainCommand);
    command('rapidr.help', async (target) => {
        if (!client) {
            vscode.window.showWarningMessage("RapidR's language server isn't running: Help comes from it. See RapidR: Show Language Server Output.", 'Show Output').then((c) => c && output.show(true));
            return;
        }
        await serverReady.catch(() => {});
        if (!help.supported()) {
            vscode.window.showWarningMessage(`This rapidr (${rapidr.version}) has no Help pages for the editor yet: update RapidR.`, 'Download').then((c) => c && vscode.env.openExternal(vscode.Uri.parse(DOWNLOAD_URL)));
            return;
        }
        await help.show(target && typeof target === 'object' && !(target instanceof vscode.Uri) ? target : undefined);
    });
    command('rapidr.restartServer', async () => {
        await stopServer();
        await resolveRapidr();
        if (!rapidr.path) {
            notifyMissing(true).catch(() => {});
            return undefined;
        }
        return startServer().catch(() => {});
    });
    command('rapidr.showOutput', () => output.show(true));
    command('rapidr.locate', locateRapidr);
    command('rapidr.showCommands', showCommands);

    context.subscriptions.push(
        vscode.debug.registerDebugConfigurationProvider(LANGUAGE, configurationProvider),
        vscode.debug.registerDebugConfigurationProvider(LANGUAGE, dynamicConfigurationProvider, vscode.DebugConfigurationProviderTriggerKind.Dynamic),
        vscode.debug.registerDebugAdapterDescriptorFactory(LANGUAGE, adapterFactory),
        vscode.languages.registerDocumentLinkProvider(SELECTOR, includeLinks),
        vscode.window.onDidChangeActiveTextEditor(updateStatus),
        vscode.workspace.onDidOpenTextDocument(updateStatus),
        vscode.workspace.onDidChangeTextDocument(suggestAfterAs),
        projectWatcher.onDidCreate(refreshProjectContext),
        projectWatcher.onDidDelete(refreshProjectContext),
        vscode.workspace.onDidChangeConfiguration(async (e) => {
            if (e.affectsConfiguration('rapidr.path')) {
                await stopServer();
                await resolveRapidr();
                if (rapidr.path) startServer().catch(() => {});
                else notifyMissing(true);
            } else if (['rapidqCompatible', 'keywordCase', 'identifierCase'].some((k) => e.affectsConfiguration(`rapidr.${k}`))) {
                restartServer().catch(() => {});
            }
        }),
        vscode.workspace.onDidGrantWorkspaceTrust(async () => {
            await refreshProjectContext();
            if (rapidr.path) return;
            await resolveRapidr();
            if (rapidr.path) startServer().catch(() => {});
        }),
    );

    await resolveRapidr();
    if (rapidr.path) startServer().catch(() => {});
    else notifyMissing(false);
    refreshProjectContext().catch(() => {});

    // (for the tests, and for other extensions)
    return {
        rapidr: () => ({ ...rapidr }),
        whenServerReady: () => serverReady,
        languageClient: () => client,
        help: () => ({ state: help.state, show: (t) => help.show(t), message: (m) => help.onMessage(m), page: help.page }),
        typeSuggestions: () => typeSuggestions,
        mcpDefinitions: () => (mcp ? mcp.definitions() : Promise.resolve(null)),
    };
}

function deactivate() {
    return stopServer();
}

module.exports = { activate, deactivate };
