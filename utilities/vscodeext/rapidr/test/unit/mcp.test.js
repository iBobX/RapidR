// `rapidr mcp` as the experimental MCP server of the editor's AI agents:
// knowing whether a rapidr has the command (src/locate.js).
'use strict';

const assert = require('assert');
const { usageHasMcp, rapidrHasMcp } = require('../../src/locate');
const { testRapidr } = require('./lsp-client');

describe('rapidr mcp (experimental)', () => {
    it('is in a usage text only when the usage lists the command', () => {
        assert.strictEqual(usageHasMcp('Usage:\n  rapidr version\n  rapidr lsp   The language server\n  rapidr dap\n'), false);
        assert.strictEqual(usageHasMcp('Usage:\n  rapidr lsp\n  rapidr mcp [--language]   The MCP server\n'), true);
        assert.strictEqual(usageHasMcp('  rapidr mcpx\n'), false, 'a command that only starts with mcp');
        assert.strictEqual(usageHasMcp('see rapidr mcp in the manual'), false, 'a sentence is not a command line');
    });

    it('is asked of the real rapidr: none has it yet', async function () {
        const rapidr = testRapidr();
        if (!rapidr) return this.skip();
        assert.strictEqual(await rapidrHasMcp(rapidr), false);
    });
});
