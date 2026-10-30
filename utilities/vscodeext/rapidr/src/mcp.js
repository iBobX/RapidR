// EXPERIMENTAL, off by default: offers `rapidr mcp` to the editor's AI agents
// as an MCP server (vscode.lm.registerMcpServerDefinitionProvider, VS Code 1.101+).
//
// `rapidr mcp` doesn't exist yet (docs/security-audit.md §7 designs its
// authentication, ROADMAP AI-ARCH phase 2 builds it), so this is the plumbing
// only: with `rapidr.experimental.mcpServer` on, a rapidr whose usage lists
// `rapidr mcp` is offered as a stdio server; any other rapidr — all of
// them today — offers nothing, and the output channel says why. Nothing is
// started by the extension: the editor asks the user to trust and run a server
// before its first start (and again when `version`, the rapidr's, changes), and
// confirms tool calls unless a tool says it only reads.
'use strict';

const vscode = require('vscode');
const { rapidrHasMcp } = require('./locate');

const PROVIDER_ID = 'rapidr.language';
const SETTING = 'experimental.mcpServer';

/**
 * @param {vscode.ExtensionContext} context
 * @param {{ getRapidr: () => { path: string|null, version: string|null }, log: (line: string) => void }} host
 * @returns {{ definitions: () => Promise<object[]>, refresh: () => void } | null} null: this editor has no MCP API (before 1.101)
 */
function registerMcp(context, host) {
    if (!vscode.lm || typeof vscode.lm.registerMcpServerDefinitionProvider !== 'function' || typeof vscode.McpStdioServerDefinition !== 'function') {
        host.log('MCP: this VS Code has no MCP server API (1.101 or newer has): the experimental RapidR MCP server is not offered');
        return null;
    }
    const changed = new vscode.EventEmitter();
    let said = '';
    const say = (line) => {
        if (line !== said) host.log(line);
        said = line;
    };

    const enabled = () => vscode.workspace.getConfiguration('rapidr').get(SETTING, false) === true;

    async function definitions() {
        if (!enabled()) return [];
        if (!vscode.workspace.isTrusted) {
            say('MCP: not offered in an untrusted workspace');
            return [];
        }
        const r = host.getRapidr();
        if (!r.path) return [];
        if (!(await rapidrHasMcp(r.path))) {
            say(`MCP: rapidr ${r.version} has no "rapidr mcp" yet: no MCP server is offered`);
            return [];
        }
        say(`MCP: offering "${r.path} mcp" (experimental)`);
        const folder = (vscode.workspace.workspaceFolders || []).find((f) => f.uri.scheme === 'file');
        // (the editor passes its own environment on: `rapidr mcp` must not hand it to a program it runs)
        const server = new vscode.McpStdioServerDefinition('RapidR (language tools)', r.path, ['mcp'], {}, r.version || undefined);
        if (folder) server.cwd = folder.uri;
        return [server];
    }

    context.subscriptions.push(
        vscode.lm.registerMcpServerDefinitionProvider(PROVIDER_ID, {
            onDidChangeMcpServerDefinitions: changed.event,
            provideMcpServerDefinitions: () => definitions(),
            // (the editor's trust prompt is the consent; a server switched off since is not started)
            resolveMcpServerDefinition: (server) => (enabled() ? server : undefined),
        }),
        changed,
        vscode.workspace.onDidChangeConfiguration((e) => {
            if (e.affectsConfiguration(`rapidr.${SETTING}`)) changed.fire();
        }),
    );
    return { definitions, refresh: () => changed.fire() };
}

module.exports = { registerMcp, PROVIDER_ID };
