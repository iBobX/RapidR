// A minimal LSP client over stdio, to ask `rapidr lsp` things in a unit test
// without VS Code (not a test file).
'use strict';

const { spawn } = require('child_process');
const fs = require('fs');
const path = require('path');

/** The rapidr the tests use: RAPIDR_PATH, else this checkout's target/debug/rapidr; null when there is none. */
function testRapidr() {
    const exe = process.platform === 'win32' ? 'rapidr.exe' : 'rapidr';
    const file = path.resolve(process.env.RAPIDR_PATH || path.join(__dirname, '..', '..', '..', '..', '..', 'target', 'debug', exe));
    return fs.existsSync(file) ? file : null;
}

class LspClient {
    constructor(exe) {
        this.child = spawn(exe, ['lsp'], { env: { ...process.env, RAPIDR_PRINT_TO: process.env.RAPIDR_PRINT_TO || '' } });
        this.buffer = Buffer.alloc(0);
        this.id = 0;
        this.pending = new Map();
        this.notifications = [];
        this.child.stdout.on('data', (d) => {
            this.buffer = Buffer.concat([this.buffer, d]);
            this.pump();
        });
        this.child.on('error', () => {});
    }

    pump() {
        for (;;) {
            const end = this.buffer.indexOf('\r\n\r\n');
            if (end < 0) return;
            const length = Number(/Content-Length: (\d+)/i.exec(this.buffer.slice(0, end).toString())[1]);
            if (this.buffer.length < end + 4 + length) return;
            const message = JSON.parse(this.buffer.slice(end + 4, end + 4 + length).toString());
            this.buffer = this.buffer.slice(end + 4 + length);
            if (message.id !== undefined && (message.result !== undefined || message.error)) {
                const resolve = this.pending.get(message.id);
                if (resolve) {
                    this.pending.delete(message.id);
                    resolve(message);
                }
            } else this.notifications.push(message);
        }
    }

    send(message) {
        const text = JSON.stringify(message);
        this.child.stdin.write(`Content-Length: ${Buffer.byteLength(text)}\r\n\r\n${text}`);
    }

    request(method, params) {
        const id = ++this.id;
        return new Promise((resolve) => {
            this.pending.set(id, resolve);
            this.send({ jsonrpc: '2.0', id, method, params });
        });
    }

    notify(method, params) {
        this.send({ jsonrpc: '2.0', method, params });
    }

    async initialize(root, options = {}) {
        const uri = 'file://' + root;
        const reply = await this.request('initialize', { processId: process.pid, rootUri: uri, capabilities: {}, workspaceFolders: [{ uri, name: 'ws' }], initializationOptions: options });
        this.notify('initialized', {});
        return reply.result;
    }

    open(uri, text) {
        this.notify('textDocument/didOpen', { textDocument: { uri, languageId: 'rapidr', version: 1, text } });
    }

    /** The diagnostics last published for `uri`, once the server has had `ms` to analyse. */
    async diagnostics(uri, ms = 150) {
        await new Promise((r) => setTimeout(r, ms));
        const last = this.notifications.filter((m) => m.method === 'textDocument/publishDiagnostics' && m.params.uri === uri).pop();
        return last ? last.params.diagnostics : [];
    }

    close() {
        this.child.kill();
    }
}

module.exports = { LspClient, testRapidr };
