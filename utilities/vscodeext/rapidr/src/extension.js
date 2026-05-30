const vscode = require('vscode');
const { RapidRCompletionProvider } = require('./completionProvider');
const { RapidRHoverProvider } = require('./hoverProvider');
const { RapidRSignatureHelpProvider } = require('./signatureProvider');
const { RapidRDocumentSymbolProvider } = require('./symbolProvider');

const RAPIDR_MODE = { language: 'rapidr', scheme: 'file' };

function activate(context) {
    // Register completion provider
    context.subscriptions.push(
        vscode.languages.registerCompletionItemProvider(
            RAPIDR_MODE,
            new RapidRCompletionProvider(),
            '.', '(', '$'
        )
    );

    // Register hover provider
    context.subscriptions.push(
        vscode.languages.registerHoverProvider(
            RAPIDR_MODE,
            new RapidRHoverProvider()
        )
    );

    // Register signature help provider
    context.subscriptions.push(
        vscode.languages.registerSignatureHelpProvider(
            RAPIDR_MODE,
            new RapidRSignatureHelpProvider(),
            '(', ','
        )
    );

    // Register document symbol provider (Outline view)
    context.subscriptions.push(
        vscode.languages.registerDocumentSymbolProvider(
            RAPIDR_MODE,
            new RapidRDocumentSymbolProvider()
        )
    );

    // Register compile command
    context.subscriptions.push(
        vscode.commands.registerCommand('rapidr.compile', () => compileFile(false))
    );

    // Register compile and run command
    context.subscriptions.push(
        vscode.commands.registerCommand('rapidr.compileAndRun', () => compileFile(true))
    );

    // Register compile to executable command
    context.subscriptions.push(
        vscode.commands.registerCommand('rapidr.compileToExe', () => compileToExe())
    );

    // Register compile for web (WASM) command
    context.subscriptions.push(
        vscode.commands.registerCommand('rapidr.compileWeb', () => compileWeb(false))
    );

    // Register compile for web and serve command
    context.subscriptions.push(
        vscode.commands.registerCommand('rapidr.compileWebAndServe', () => compileWeb(true))
    );

    // Bytecode pipeline (Phase 7)
    context.subscriptions.push(
        vscode.commands.registerCommand('rapidr.buildBc', () => bcCommand('build-bc'))
    );
    context.subscriptions.push(
        vscode.commands.registerCommand('rapidr.runBc', () => bcRunCommand())
    );
    context.subscriptions.push(
        vscode.commands.registerCommand('rapidr.bundleBc', () => bcCommand('bundle-bc'))
    );

    // Phase 8: unified compiled vs interpreted build modes
    context.subscriptions.push(
        vscode.commands.registerCommand('rapidr.buildInterp', () => interpBuildCommand(false))
    );
    context.subscriptions.push(
        vscode.commands.registerCommand('rapidr.buildWebInterp', () => interpBuildCommand(true))
    );

    // Setup diagnostics
    const diagnostics = vscode.languages.createDiagnosticCollection('rapidr');
    context.subscriptions.push(diagnostics);

    // Validate on save
    context.subscriptions.push(
        vscode.workspace.onDidSaveTextDocument(doc => {
            if (doc.languageId === 'rapidr') {
                validateDocument(doc, diagnostics);
            }
        })
    );

    // Validate on open
    context.subscriptions.push(
        vscode.workspace.onDidOpenTextDocument(doc => {
            if (doc.languageId === 'rapidr') {
                validateDocument(doc, diagnostics);
            }
        })
    );

    // Validate currently open document
    if (vscode.window.activeTextEditor && vscode.window.activeTextEditor.document.languageId === 'rapidr') {
        validateDocument(vscode.window.activeTextEditor.document, diagnostics);
    }

    // Status bar item
    const statusItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
    statusItem.text = '$(play) RapidR';
    statusItem.tooltip = 'Compile and Run (F5)';
    statusItem.command = 'rapidr.compileAndRun';
    context.subscriptions.push(statusItem);

    context.subscriptions.push(
        vscode.window.onDidChangeActiveTextEditor(editor => {
            if (editor && editor.document.languageId === 'rapidr') {
                statusItem.show();
            } else {
                statusItem.hide();
            }
        })
    );

    if (vscode.window.activeTextEditor && vscode.window.activeTextEditor.document.languageId === 'rapidr') {
        statusItem.show();
    }

    console.log('RapidR extension activated');
}

function findCompilerPath() {
    const path = require('path');
    const fs = require('fs');
    const config = vscode.workspace.getConfiguration('rapidr');
    const configuredPath = config.get('compilerPath');
    if (configuredPath && fs.existsSync(configuredPath)) {
        return configuredPath;
    }
    // Auto-detect: look for rapidr binary relative to the workspace, then in PATH
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (workspaceFolders) {
        for (const folder of workspaceFolders) {
            const candidates = [
                path.join(folder.uri.fsPath, 'rapidr'),
                path.join(folder.uri.fsPath, '..', 'rapidr'),
                path.join(folder.uri.fsPath, '..', '..', 'rapidr'),
            ];
            for (const candidate of candidates) {
                try {
                    const resolved = fs.realpathSync(candidate);
                    if (fs.existsSync(resolved)) {
                        return resolved;
                    }
                } catch {
                    // keep searching
                }
            }
        }
    }
    // Fall back to PATH
    return 'rapidr';
}

function compileFile(run) {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== 'rapidr') {
        vscode.window.showWarningMessage('No RapidR file is open.');
        return;
    }
    editor.document.save().then(() => {
        const filePath = editor.document.uri.fsPath;
        const compilerPath = findCompilerPath();
        const config = vscode.workspace.getConfiguration('rapidr');

        // Use the shortcut syntax: rapidr --release <file.rr>
        // This builds the file and places the binary alongside the source
        let cmd = `"${compilerPath}" --release "${filePath}"`;
        if (run || config.get('runAfterCompile')) {
            const path = require('path');
            const baseName = path.basename(filePath, path.extname(filePath));
            const binaryPath = path.join(path.dirname(filePath), baseName);
            cmd += ` && "${binaryPath}"`;
        }

        const terminal = vscode.window.createTerminal({ name: 'RapidR' });
        terminal.sendText(cmd);
        terminal.show();
    });
}

function compileToExe() {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== 'rapidr') {
        vscode.window.showWarningMessage('No RapidR file is open.');
        return;
    }
    editor.document.save().then(() => {
        const filePath = editor.document.uri.fsPath;
        const compilerPath = findCompilerPath();

        const cmd = `"${compilerPath}" --release "${filePath}"`;
        const terminal = vscode.window.createTerminal({ name: 'RapidR Build' });
        terminal.sendText(cmd);
        terminal.show();
    });
}

function compileWeb(serve) {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== 'rapidr') {
        vscode.window.showWarningMessage('No RapidR file is open.');
        return;
    }
    editor.document.save().then(() => {
        const path = require('path');
        const filePath = editor.document.uri.fsPath;
        const compilerPath = findCompilerPath();
        const baseName = path.basename(filePath, path.extname(filePath));
        const webDir = path.join(path.dirname(filePath), baseName + '_web');

        let cmd = `"${compilerPath}" --web "${filePath}"`;
        if (serve) {
            const config = vscode.workspace.getConfiguration('rapidr');
            const port = config.get('webServerPort') || 8080;
            cmd += ` && echo "\\nServing at http://localhost:${port}" && python3 -m http.server -d "${webDir}" ${port}`;
        }

        const terminal = vscode.window.createTerminal({ name: 'RapidR Web' });
        terminal.sendText(cmd);
        terminal.show();

        if (serve) {
            const config = vscode.workspace.getConfiguration('rapidr');
            const port = config.get('webServerPort') || 8080;
            // Open browser after a short delay to let the server start
            setTimeout(() => {
                vscode.env.openExternal(vscode.Uri.parse(`http://localhost:${port}`));
            }, 3000);
        }
    });
}

function validateDocument(document, diagnostics) {
    const text = document.getText();
    const lines = text.split('\n');
    const diags = [];
    const blockStack = [];

    for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        const trimmed = line.trim();
        const upper = trimmed.toUpperCase();

        // Skip comments and empty lines
        if (!trimmed || upper.startsWith("'") || upper.startsWith('REM ') || upper === 'REM') {
            continue;
        }
        // Skip directives
        if (trimmed.startsWith('$')) continue;

        // Track block structures for mismatched END detection
        if (/^(IF\b.*\bTHEN\s*$)/i.test(trimmed)) {
            blockStack.push({ type: 'IF', line: i });
        } else if (/^FOR\b/i.test(upper)) {
            blockStack.push({ type: 'FOR', line: i });
        } else if (/^WHILE\b/i.test(upper)) {
            blockStack.push({ type: 'WHILE', line: i });
        } else if (/^DO\b/i.test(upper)) {
            blockStack.push({ type: 'DO', line: i });
        } else if (/^SUB\b/i.test(upper)) {
            blockStack.push({ type: 'SUB', line: i });
        } else if (/^FUNCTION\b/i.test(upper)) {
            blockStack.push({ type: 'FUNCTION', line: i });
        } else if (/^SELECT\s+CASE\b/i.test(upper)) {
            blockStack.push({ type: 'SELECT', line: i });
        } else if (/^TYPE\b/i.test(upper)) {
            blockStack.push({ type: 'TYPE', line: i });
        } else if (/^CREATE\b/i.test(upper)) {
            blockStack.push({ type: 'CREATE', line: i });
        } else if (/^WITH\b/i.test(upper)) {
            blockStack.push({ type: 'WITH', line: i });
        }

        // Pop blocks
        if (/^END\s+IF\b/i.test(upper)) {
            if (blockStack.length > 0 && blockStack[blockStack.length - 1].type === 'IF') {
                blockStack.pop();
            }
        } else if (/^NEXT\b/i.test(upper)) {
            if (blockStack.length > 0 && blockStack[blockStack.length - 1].type === 'FOR') {
                blockStack.pop();
            }
        } else if (/^WEND\b/i.test(upper)) {
            if (blockStack.length > 0 && blockStack[blockStack.length - 1].type === 'WHILE') {
                blockStack.pop();
            }
        } else if (/^LOOP\b/i.test(upper)) {
            if (blockStack.length > 0 && blockStack[blockStack.length - 1].type === 'DO') {
                blockStack.pop();
            }
        } else if (/^END\s+SUB\b/i.test(upper)) {
            if (blockStack.length > 0 && blockStack[blockStack.length - 1].type === 'SUB') {
                blockStack.pop();
            }
        } else if (/^END\s+FUNCTION\b/i.test(upper)) {
            if (blockStack.length > 0 && blockStack[blockStack.length - 1].type === 'FUNCTION') {
                blockStack.pop();
            }
        } else if (/^END\s+SELECT\b/i.test(upper)) {
            if (blockStack.length > 0 && blockStack[blockStack.length - 1].type === 'SELECT') {
                blockStack.pop();
            }
        } else if (/^END\s+TYPE\b/i.test(upper)) {
            if (blockStack.length > 0 && blockStack[blockStack.length - 1].type === 'TYPE') {
                blockStack.pop();
            }
        } else if (/^END\s+CREATE\b/i.test(upper)) {
            if (blockStack.length > 0 && blockStack[blockStack.length - 1].type === 'CREATE') {
                blockStack.pop();
            }
        } else if (/^END\s+WITH\b/i.test(upper)) {
            if (blockStack.length > 0 && blockStack[blockStack.length - 1].type === 'WITH') {
                blockStack.pop();
            }
        }

        // Check for unclosed strings
        const inString = (trimmed.split('"').length - 1) % 2 !== 0;
        if (inString) {
            // Check it's not just in a comment
            const commentIdx = trimmed.indexOf("'");
            const firstQuote = trimmed.indexOf('"');
            if (commentIdx === -1 || firstQuote < commentIdx) {
                diags.push(new vscode.Diagnostic(
                    new vscode.Range(i, 0, i, line.length),
                    'Unterminated string literal',
                    vscode.DiagnosticSeverity.Error
                ));
            }
        }
    }

    // Report unclosed blocks
    for (const block of blockStack) {
        const endKeyword = block.type === 'FOR' ? 'NEXT' : block.type === 'WHILE' ? 'WEND' : block.type === 'DO' ? 'LOOP' : `END ${block.type}`;
        diags.push(new vscode.Diagnostic(
            new vscode.Range(block.line, 0, block.line, lines[block.line].length),
            `Unclosed ${block.type} block — missing ${endKeyword}`,
            vscode.DiagnosticSeverity.Warning
        ));
    }

    diagnostics.set(document.uri, diags);
}

// ---- Phase 7: bytecode pipeline helpers ----

function bcCommand(subcommand) {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== 'rapidr') {
        vscode.window.showWarningMessage('No RapidR file is open.');
        return;
    }
    editor.document.save().then(() => {
        const filePath = editor.document.uri.fsPath;
        const compilerPath = findCompilerPath();
        const cmd = `"${compilerPath}" ${subcommand} "${filePath}"`;
        const terminal = vscode.window.createTerminal({ name: `RapidR ${subcommand}` });
        terminal.sendText(cmd);
        terminal.show();
    });
}

function bcRunCommand() {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== 'rapidr') {
        vscode.window.showWarningMessage('No RapidR file is open.');
        return;
    }
    editor.document.save().then(() => {
        const path = require('path');
        const filePath = editor.document.uri.fsPath;
        const compilerPath = findCompilerPath();
        const stem = path.basename(filePath, path.extname(filePath));
        const dir = path.dirname(filePath);
        const rrbc = path.join(dir, `${stem}.rrbc`);
        // Compile to bytecode (placed next to source) then run it.
        const cmd = `"${compilerPath}" build-bc "${filePath}" -o "${rrbc}" && "${compilerPath}" run-bc "${rrbc}"`;
        const terminal = vscode.window.createTerminal({ name: 'RapidR run-bc' });
        terminal.sendText(cmd);
        terminal.show();
    });
}

// `rapidr build <file> --interp [--web]` — single self-contained
// native exe (desktop) or static web bundle .zip (web).
function interpBuildCommand(web) {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== 'rapidr') {
        vscode.window.showWarningMessage('No RapidR file is open.');
        return;
    }
    editor.document.save().then(() => {
        const filePath = editor.document.uri.fsPath;
        const compilerPath = findCompilerPath();
        const flags = web ? '--web --interp' : '--interp';
        const label = web ? 'RapidR build --web --interp' : 'RapidR build --interp';
        const cmd = `"${compilerPath}" build "${filePath}" ${flags}`;
        const terminal = vscode.window.createTerminal({ name: label });
        terminal.sendText(cmd);
        terminal.show();
    });
}

function deactivate() {}

module.exports = { activate, deactivate };
