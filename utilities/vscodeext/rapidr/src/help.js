// RapidR's Help in VS Code: the page Studio's Help pane shows for the word
// at the caret (rapidr lsp's `rapidr/helpPage`: what it is, the syntax, what
// it does, a component's members as links, the program's own names with
// where they are declared), in a panel beside the editor, with a search box
// (`rapidr/helpSuggest`), Back, Insert into the code, Go to Declaration and
// Find References. One implementation of the pages: the language server's;
// this file only shows them.
'use strict';

const vscode = require('vscode');
const crypto = require('crypto');
const { renderMarkdown, escapeHtml } = require('./markdown');

const LANGUAGE = 'rapidr';
const VIEW_TYPE = 'rapidr.help';

class HelpPanel {
    /**
     * @param {() => (import('vscode-languageclient/node').LanguageClient | null)} getClient
     * @param {(line: string) => void} log
     */
    constructor(getClient, log) {
        this.getClient = getClient;
        this.log = log;
        this.panel = null;
        /** The requests shown, the last one now (Back goes to the one before). */
        this.history = [];
        /** The last RapidR editor: where Insert writes and Go to Declaration is relative to. */
        this.editor = vscode.window.activeTextEditor && vscode.window.activeTextEditor.document.languageId === LANGUAGE ? vscode.window.activeTextEditor : null;
        /** What the panel shows now (for the tests): { kind, title, markdown, html }. */
        this.state = null;
        this.disposables = [
            vscode.window.onDidChangeActiveTextEditor((e) => {
                if (e && e.document.languageId === LANGUAGE) this.editor = e;
            }),
        ];
    }

    dispose() {
        for (const d of this.disposables) d.dispose();
        if (this.panel) this.panel.dispose();
    }

    supported() {
        const client = this.getClient();
        const caps = client && client.initializeResult && client.initializeResult.capabilities;
        return !!(caps && caps.experimental && caps.experimental.rapidr && caps.experimental.rapidr.helpPage);
    }

    /**
     * Shows the page for what `target` says: { word, of } (a word, and the
     * component it is a member of), { uri, line, character } (a position in a
     * document), or nothing (the word at the caret of the RapidR editor;
     * none: the start page).
     */
    async show(target) {
        const request = this.requestFor(target);
        this.history.push(request);
        if (this.history.length > 50) this.history.shift();
        await this.render(request, true);
    }

    requestFor(target) {
        if (target && typeof target.word === 'string') return { word: target.word, of: typeof target.of === 'string' ? target.of : '' };
        let uri = null;
        let position = null;
        if (target && typeof target.uri === 'string' && Number.isInteger(target.line)) {
            uri = target.uri;
            position = { line: target.line, character: Number.isInteger(target.character) ? target.character : 0 };
        } else {
            const editor = vscode.window.activeTextEditor && vscode.window.activeTextEditor.document.languageId === LANGUAGE ? vscode.window.activeTextEditor : this.editor;
            if (editor) {
                uri = editor.document.uri.toString();
                position = { line: editor.selection.active.line, character: editor.selection.active.character };
            }
        }
        return uri ? { uri, position } : { word: '', of: '' };
    }

    async ask(request) {
        const client = this.getClient();
        if (!client) throw new Error('the language server is not running');
        // (F1 is VS Code's own, the Command Palette: Help here is Shift+F1)
        const params = { canInsert: !!this.editor, key: 'Shift+F1' };
        if (request.uri) {
            params.textDocument = { uri: request.uri };
            params.position = request.position;
        } else {
            params.word = request.word;
            params.of = request.of;
            // (the program's own names: the open RapidR editor's)
            if (this.editor) params.textDocument = { uri: this.editor.document.uri.toString() };
        }
        return client.sendRequest('rapidr/helpPage', params);
    }

    async render(request, reveal) {
        let page;
        try {
            page = await this.ask(request);
        } catch (err) {
            this.log(`Help: ${err.message || err}`);
            page = { kind: 'none', insert: '', uri: null, line: 0, column: 0, markdown: `## Help isn't available\n\nRapidR's language server didn't answer: ${String(err.message || err)}. Run **RapidR: Restart Language Server**, or update RapidR.\n` };
        }
        const title = (/^#+\s+(.*)$/m.exec(page.markdown) || [])[1] || 'Help';
        const html = renderMarkdown(page.markdown);
        this.page = page;
        this.state = { kind: page.kind, title, markdown: page.markdown, html, history: this.history.length };
        const panel = this.ensurePanel(reveal);
        panel.title = `RapidR Help: ${title.replace(/\\(.)/g, '$1')}`;
        panel.webview.html = this.html(panel.webview, html, page, title);
    }

    ensurePanel(reveal) {
        if (this.panel) {
            if (reveal) this.panel.reveal(undefined, true);
            return this.panel;
        }
        this.panel = vscode.window.createWebviewPanel(VIEW_TYPE, 'RapidR Help', { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true }, { enableScripts: true, localResourceRoots: [] });
        this.panel.onDidDispose(() => {
            this.panel = null;
        });
        this.panel.webview.onDidReceiveMessage((m) => this.onMessage(m));
        return this.panel;
    }

    // ---------------------------------------------------------------- what the page asks for

    /** A message of the page's script (anything else is ignored). */
    async onMessage(m) {
        if (!m || typeof m !== 'object') return;
        try {
            switch (m.type) {
                case 'link':
                    return await this.follow(String(m.target || ''));
                case 'search':
                    return await this.show({ word: String(m.query || '').slice(0, 200), of: '' });
                case 'suggest':
                    return await this.suggest(String(m.query || '').slice(0, 200));
                case 'back':
                    if (this.history.length > 1) {
                        this.history.pop();
                        await this.render(this.history[this.history.length - 1], false);
                    }
                    return;
                default:
                    return;
            }
        } catch (err) {
            this.log(`Help: ${err.message || err}`);
        }
    }

    /** What a link of a page does: another entry, Insert, Go to Declaration, Find References, the web. */
    async follow(target) {
        const m = /^([a-z-]+):(.*)$/i.exec(target);
        if (!m) return;
        const [, scheme, rest] = m;
        switch (scheme.toLowerCase()) {
            case 'help': {
                const text = decodeURIComponent(rest);
                const [word, of] = text.split('@');
                return await this.show({ word: word || '', of: of || '' });
            }
            case 'help-insert':
                return await this.insert();
            case 'help-goto':
                return await this.goto(false);
            case 'help-refs':
                return await this.goto(true);
            case 'https':
            case 'http':
                return void (await vscode.env.openExternal(vscode.Uri.parse(target)));
            default:
        }
    }

    /** Writes the syntax of the page's entry into the RapidR editor, at its caret. */
    async insert() {
        const text = this.page && this.page.insert;
        const editor = this.editor && vscode.window.visibleTextEditors.find((e) => e.document === this.editor.document);
        if (!text || !editor) {
            vscode.window.showInformationMessage('Open a RapidR file in an editor to insert code into it.');
            return;
        }
        const shown = await vscode.window.showTextDocument(editor.document, { viewColumn: editor.viewColumn, preserveFocus: false });
        await shown.edit((b) => b.replace(shown.selection, text));
    }

    /** Where the page's name of the program is declared; with `references`, what uses it. */
    async goto(references) {
        const p = this.page;
        if (!p || !p.uri) return;
        const doc = await vscode.workspace.openTextDocument(vscode.Uri.parse(p.uri));
        const at = new vscode.Position(p.line, p.column);
        const shown = await vscode.window.showTextDocument(doc, { viewColumn: this.editor ? this.editor.viewColumn : undefined, selection: new vscode.Range(at, at), preserveFocus: false });
        if (references) await vscode.commands.executeCommand('editor.action.referenceSearch.trigger');
        else shown.revealRange(new vscode.Range(at, at), vscode.TextEditorRevealType.InCenterIfOutsideViewport);
    }

    async suggest(query) {
        const client = this.getClient();
        if (!client || !this.panel) return;
        const params = { word: query };
        if (this.editor) params.textDocument = { uri: this.editor.document.uri.toString() };
        const items = await client.sendRequest('rapidr/helpSuggest', params);
        this.panel.webview.postMessage({ type: 'suggestions', query, items: Array.isArray(items) ? items.slice(0, 25) : [] });
    }

    // ---------------------------------------------------------------- the page

    html(webview, body, page, title) {
        const nonce = crypto.randomBytes(16).toString('base64');
        const csp = `default-src 'none'; style-src 'nonce-${nonce}'; script-src 'nonce-${nonce}';`;
        const word = this.history.length ? this.history[this.history.length - 1].word || '' : '';
        return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta http-equiv="Content-Security-Policy" content="${csp}">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${escapeHtml(title)}</title>
<style nonce="${nonce}">
  body { font-family: var(--vscode-font-family); font-size: var(--vscode-font-size); color: var(--vscode-foreground); padding: 0 16px 24px; line-height: 1.5; }
  header { position: sticky; top: 0; z-index: 2; background: var(--vscode-editor-background); padding: 12px 0 8px; display: flex; gap: 8px; align-items: center; }
  header .box { position: relative; flex: 1; }
  input { width: 100%; box-sizing: border-box; padding: 4px 8px; color: var(--vscode-input-foreground); background: var(--vscode-input-background); border: 1px solid var(--vscode-input-border, transparent); border-radius: 2px; font: inherit; }
  input:focus { outline: 1px solid var(--vscode-focusBorder); outline-offset: -1px; }
  button { color: var(--vscode-button-secondaryForeground); background: var(--vscode-button-secondaryBackground); border: none; border-radius: 2px; padding: 4px 10px; font: inherit; cursor: pointer; }
  button:hover { background: var(--vscode-button-secondaryHoverBackground); }
  button:disabled { opacity: .5; cursor: default; }
  #list { position: absolute; left: 0; right: 0; top: 100%; background: var(--vscode-editorSuggestWidget-background); border: 1px solid var(--vscode-editorSuggestWidget-border, var(--vscode-widget-border, transparent)); box-shadow: 0 2px 8px var(--vscode-widget-shadow); max-height: 260px; overflow-y: auto; display: none; }
  #list div { padding: 2px 8px; display: flex; justify-content: space-between; gap: 12px; cursor: pointer; }
  #list div.sel, #list div:hover { background: var(--vscode-editorSuggestWidget-selectedBackground); color: var(--vscode-editorSuggestWidget-selectedForeground, inherit); }
  #list span { opacity: .7; }
  a { color: var(--vscode-textLink-foreground); text-decoration: none; }
  a:hover { color: var(--vscode-textLink-activeForeground); text-decoration: underline; }
  h2 { font-size: 1.5em; margin: .6em 0 .3em; font-weight: 600; }
  h3 { font-size: 1.15em; margin: 1.1em 0 .3em; font-weight: 600; }
  h4 { font-size: 1em; margin: 1em 0 .3em; font-weight: 600; }
  p { margin: .5em 0; }
  ul { margin: .4em 0; padding-left: 1.4em; }
  code { font-family: var(--vscode-editor-font-family); font-size: .95em; background: var(--vscode-textCodeBlock-background); padding: 0 4px; border-radius: 3px; }
  pre { background: var(--vscode-textCodeBlock-background); padding: 8px 12px; border-radius: 4px; overflow-x: auto; margin: .6em 0; }
  pre code { background: none; padding: 0; font-size: var(--vscode-editor-font-size, 13px); }
</style>
</head>
<body>
<header>
  <button id="back" title="Back" ${this.history.length > 1 ? '' : 'disabled'}>&larr; Back</button>
  <div class="box"><input id="q" type="text" autocomplete="off" spellcheck="false" placeholder="Search Help: a word, a component, a name in your program" value="${escapeHtml(word)}"><div id="list"></div></div>
</header>
<main>
${body}
</main>
<script nonce="${nonce}">
  const vscode = acquireVsCodeApi();
  const q = document.getElementById('q');
  const list = document.getElementById('list');
  let items = [];
  let selected = -1;
  let timer;
  document.addEventListener('click', (e) => {
    const a = e.target.closest('a[data-link]');
    if (a) { e.preventDefault(); vscode.postMessage({ type: 'link', target: a.dataset.link }); return; }
    if (!e.target.closest('.box')) list.style.display = 'none';
  });
  document.getElementById('back').addEventListener('click', () => vscode.postMessage({ type: 'back' }));
  function show() {
    list.innerHTML = '';
    items.forEach((it, i) => {
      const row = document.createElement('div');
      if (i === selected) row.className = 'sel';
      const n = document.createElement('b'); n.textContent = it.name;
      const w = document.createElement('span'); w.textContent = it.what;
      row.append(n, w);
      row.addEventListener('mousedown', (e) => { e.preventDefault(); vscode.postMessage({ type: 'search', query: it.name }); list.style.display = 'none'; });
      list.append(row);
    });
    list.style.display = items.length ? 'block' : 'none';
  }
  q.addEventListener('input', () => {
    clearTimeout(timer);
    selected = -1;
    if (q.value.trim() === '') { items = []; show(); return; }
    timer = setTimeout(() => vscode.postMessage({ type: 'suggest', query: q.value }), 120);
  });
  q.addEventListener('keydown', (e) => {
    if (e.key === 'ArrowDown') { selected = Math.min(items.length - 1, selected + 1); show(); e.preventDefault(); }
    else if (e.key === 'ArrowUp') { selected = Math.max(-1, selected - 1); show(); e.preventDefault(); }
    else if (e.key === 'Escape') { list.style.display = 'none'; }
    else if (e.key === 'Enter') {
      const query = selected >= 0 && items[selected] ? items[selected].name : q.value;
      list.style.display = 'none';
      vscode.postMessage({ type: 'search', query });
    }
  });
  window.addEventListener('message', (e) => {
    if (e.data && e.data.type === 'suggestions' && e.data.query === q.value) { items = e.data.items; selected = -1; show(); }
  });
</script>
</body>
</html>`;
    }
}

module.exports = { HelpPanel };
