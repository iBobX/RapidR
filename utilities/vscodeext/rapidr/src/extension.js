// RapidR for VS Code: a thin client. IntelliSense, diagnostics, navigation,
// rename and formatting come from `rapidr lsp` (the language server, Rust);
// debugging from `rapidr dap` (the debug adapter, Rust); run / build
// commands run `rapidr` in a terminal. This file only finds rapidr and wires
// those up.
'use strict';

const vscode = require('vscode');
const nodePath = require('path');
const { LanguageClient, TransportKind, RevealOutputChannelOn, ErrorAction, CloseAction } = require('vscode-languageclient/node');
const { findRapidr, rapidrVersion } = require('./locate');
const { shellKind, commandLine } = require('./shell');

const DOWNLOAD_URL = 'https://github.com/iBobX/RapidR/releases';
const LANGUAGE = 'rapidr';
const TERMINAL_NAME = 'RapidR';
const SELECTOR = [
    { language: LANGUAGE, scheme: 'file' },
    { language: LANGUAGE, scheme: 'untitled' },
];

/** What we know of rapidr: { path, version, source, error }. */
let rapidr = { path: null, version: null, source: null, error: null };
let serverError = null;
let client = null;
let serverReady = Promise.resolve();
let output;
let status;
let watcher;
let missingNotified = false;

// ---------------------------------------------------------------- rapidr

async function resolveRapidr() {
    const cfg = vscode.workspace.getConfiguration('rapidr');
    const folders = (vscode.workspace.workspaceFolders || []).filter((f) => f.uri.scheme === 'file').map((f) => f.uri.fsPath);
    const found = findRapidr({ setting: cfg.get('path', ''), workspaceFolders: folders, trusted: vscode.workspace.isTrusted });
    rapidr = { path: null, version: null, source: null, error: found.error || null };
    if (found.path) {
        try {
            rapidr = { path: found.path, version: await rapidrVersion(found.path), source: found.source, error: null };
            output.appendLine(`rapidr ${rapidr.version}: ${rapidr.path} (from ${rapidr.source})`);
        } catch (err) {
            rapidr.error = `${found.path} didn't run: ${err.message}`;
        }
    }
    if (!rapidr.path) output.appendLine(`RapidR was not found. ${rapidr.error || 'Looked in: ' + (found.tried || []).join(', ')}`);
    await vscode.commands.executeCommand('setContext', 'rapidr.missing', !rapidr.path);
    updateStatus();
    return rapidr;
}

/** "RapidR was not found", with Download / Locate. Once per session unless `force`. */
async function notifyMissing(force) {
    if (missingNotified && !force) return;
    missingNotified = true;
    const detail = rapidr.error ? ` ${rapidr.error}` : ' Install RapidR, or tell VS Code where rapidr is.';
    const choice = await vscode.window.showWarningMessage(`RapidR was not found.${detail}`, 'Download RapidR', 'Locate rapidr…');
    if (choice === 'Download RapidR') await vscode.env.openExternal(vscode.Uri.parse(DOWNLOAD_URL));
    else if (choice === 'Locate rapidr…') await locateRapidr();
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
    const found = findRapidr({ setting: chosen });
    if (!found.path) {
        vscode.window.showErrorMessage(`${chosen} isn't rapidr (the RapidR executable).`);
        return;
    }
    try {
        const version = await rapidrVersion(found.path);
        // (the change event resolves rapidr again and restarts the server)
        await vscode.workspace.getConfiguration('rapidr').update('path', found.path, vscode.ConfigurationTarget.Global);
        vscode.window.showInformationMessage(`Using rapidr ${version}: ${found.path}`);
    } catch (err) {
        vscode.window.showErrorMessage(`${found.path} didn't run: ${err.message}`);
    }
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
        errorHandler: {
            error: () => ({ action: ErrorAction.Continue }),
            closed: () => {
                // (a server that never started is reported once, below)
                if (!started) return { action: CloseAction.DoNotRestart, handled: true };
                const now = Date.now();
                crashes = crashes.filter((t) => now - t < 3 * 60 * 1000).concat(now);
                if (crashes.length <= 4) {
                    output.appendLine('rapidr lsp stopped: restarting it');
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
            output.appendLine(`rapidr lsp started (${rapidr.path})`);
            updateStatus();
        },
        async (err) => {
            serverError = err && err.message ? err.message : String(err);
            output.appendLine(`rapidr lsp didn't start: ${serverError}`);
            if (client === c) client = null;
            try { await c.dispose(); } catch { /* already gone */ }
            updateStatus();
            vscode.window.showErrorMessage(
                `RapidR's language server didn't start (rapidr ${rapidr.version} at ${rapidr.path}). A RapidR older than the language server has no "rapidr lsp": update it.`,
                'Show Output', 'Download RapidR',
            ).then((choice) => {
                if (choice === 'Show Output') output.show(true);
                else if (choice === 'Download RapidR') vscode.env.openExternal(vscode.Uri.parse(DOWNLOAD_URL));
            });
            throw err;
        },
    );
    serverReady.catch(() => {});
    return serverReady;
}

async function stopServer() {
    const c = client;
    client = null;
    if (!c) return;
    try {
        await c.stop();
    } catch (err) {
        output.appendLine(`rapidr lsp: stopping: ${err.message || err}`);
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
        status.tooltip = `RapidR was not found${rapidr.error ? ': ' + rapidr.error : ''}. Click for options.`;
        status.backgroundColor = new vscode.ThemeColor('statusBarItem.warningBackground');
    } else if (serverError) {
        status.text = `$(warning) RapidR ${rapidr.version}`;
        status.tooltip = `rapidr ${rapidr.version} (${rapidr.path}): the language server didn't start. Click for options.`;
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
            { label: '$(tools) Build Native Executable', description: 'rapidr build --release', command: 'rapidr.buildNative' },
            { label: '$(package) Build Standalone Executable', description: 'rapidr build --interp', command: 'rapidr.buildStandalone' },
            { label: '$(globe) Bundle for the Web', description: 'rapidr bundle-bc', command: 'rapidr.bundleWeb' },
            { label: '', kind: vscode.QuickPickItemKind.Separator },
            { label: '$(refresh) Restart Language Server', command: 'rapidr.restartServer' },
            { label: '$(output) Show Language Server Output', command: 'rapidr.showOutput' },
            { label: '$(file-binary) Locate rapidr…', description: rapidr.path, command: 'rapidr.locate' },
        ]
        : [
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
    await notifyMissing(true);
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
            await notifyMissing(true);
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
    output = vscode.window.createOutputChannel('RapidR Language Server');
    status = vscode.window.createStatusBarItem('rapidr.status', vscode.StatusBarAlignment.Left, 50);
    status.name = 'RapidR';
    status.command = 'rapidr.showCommands';
    // ($INCLUDE files changed outside the editor: one watcher for every client)
    watcher = vscode.workspace.createFileSystemWatcher('**/*.{rr,bas,inc,RR,BAS,INC}');
    context.subscriptions.push(output, status, watcher);

    const command = (id, fn) => context.subscriptions.push(vscode.commands.registerCommand(id, fn));
    command('rapidr.run', runCommand);
    command('rapidr.debug', debugCommand);
    command('rapidr.buildNative', buildNativeCommand);
    command('rapidr.buildStandalone', buildStandaloneCommand);
    command('rapidr.bundleWeb', bundleWebCommand);
    command('rapidr.restartServer', async () => {
        await stopServer();
        await resolveRapidr();
        if (!rapidr.path) return notifyMissing(true);
        return startServer().catch(() => {});
    });
    command('rapidr.showOutput', () => output.show(true));
    command('rapidr.locate', locateRapidr);
    command('rapidr.showCommands', showCommands);

    context.subscriptions.push(
        vscode.debug.registerDebugConfigurationProvider(LANGUAGE, configurationProvider),
        vscode.debug.registerDebugConfigurationProvider(LANGUAGE, dynamicConfigurationProvider, vscode.DebugConfigurationProviderTriggerKind.Dynamic),
        vscode.debug.registerDebugAdapterDescriptorFactory(LANGUAGE, adapterFactory),
        vscode.window.onDidChangeActiveTextEditor(updateStatus),
        vscode.workspace.onDidOpenTextDocument(updateStatus),
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
            if (rapidr.path) return;
            await resolveRapidr();
            if (rapidr.path) startServer().catch(() => {});
        }),
    );

    await resolveRapidr();
    if (rapidr.path) startServer().catch(() => {});
    else notifyMissing(false);

    // (for the tests, and for other extensions)
    return {
        rapidr: () => ({ ...rapidr }),
        whenServerReady: () => serverReady,
        languageClient: () => client,
    };
}

function deactivate() {
    return stopServer();
}

module.exports = { activate, deactivate };
