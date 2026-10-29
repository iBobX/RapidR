// The static files of the extension against RapidR itself: the TextMate
// grammar has every name of the language registry (scripts/grammar.js adds
// what a new release brings), and every snippet expands to code the language
// server finds no error in. Needs a rapidr (RAPIDR_PATH, or this checkout's
// target/debug/rapidr); without one these are skipped.
'use strict';

const assert = require('assert');
const { execFileSync } = require('child_process');
const fs = require('fs');
const os = require('os');
const path = require('path');
const { missing, patternsByScope } = require('../../scripts/grammar');
const { LspClient, testRapidr } = require('./lsp-client');

const ext = path.resolve(__dirname, '..', '..');
const grammar = JSON.parse(fs.readFileSync(path.join(ext, 'syntaxes', 'rapidr.tmLanguage.json'), 'utf8'));

describe('the TextMate grammar', () => {
    it('is made of regular expressions that compile', () => {
        let n = 0;
        const walk = (o) => {
            if (Array.isArray(o)) o.forEach(walk);
            else if (o && typeof o === 'object') {
                if (typeof o.match === 'string') {
                    n++;
                    assert.doesNotThrow(() => new RegExp(o.match.replace(/^\(\?i\)/, ''), 'i'), o.match.slice(0, 80));
                }
                Object.values(o).forEach(walk);
            }
        };
        walk(grammar);
        assert.ok(n > 30);
    });

    it('has every name of the language registry', function () {
        const rapidr = testRapidr();
        if (!rapidr) return this.skip();
        const registry = JSON.parse(execFileSync(rapidr, ['lang', 'export', '--json'], { maxBuffer: 64 * 1024 * 1024 }).toString());
        const gaps = [...missing(registry, patternsByScope(grammar))].map(([scope, list]) => `${scope}: ${list.join(' ')}`);
        assert.deepStrictEqual(gaps, [], 'node scripts/grammar.js adds them');
    });
});

describe('the snippets', () => {
    const snippets = JSON.parse(fs.readFileSync(path.join(ext, 'snippets', 'rapidr.json'), 'utf8'));

    /** The text a snippet types before anything is edited: placeholders' defaults, the first choice. */
    function expand(body) {
        let t = Array.isArray(body) ? body.join('\n') : body;
        t = t.replace(/\$\{\d+\|([^,|}]*)[^}]*\|\}/g, '$1');
        for (let i = 0; i < 3; i++) t = t.replace(/\$\{\d+:([^${}]*)\}/g, '$1');
        return t.replace(/\$\{\d+\}|\$\d+/g, '').replace(/\\\$/g, '$');
    }

    it('have a prefix of their own and a body', () => {
        const seen = new Map();
        for (const [name, s] of Object.entries(snippets)) {
            assert.ok(s.prefix && s.body && s.description, name);
            for (const p of [].concat(s.prefix)) {
                assert.ok(!seen.has(p), `${name} and ${seen.get(p)} both have the prefix "${p}"`);
                seen.set(p, name);
            }
        }
    });

    it('expand to code with no error (a missing include file aside)', async function () {
        this.timeout(60000);
        const rapidr = testRapidr();
        if (!rapidr) return this.skip();
        const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rvsc-snippets-'));
        const client = new LspClient(rapidr);
        try {
            await client.initialize(dir);
            const bad = [];
            let n = 0;
            for (const [name, s] of Object.entries(snippets)) {
                const uri = `file://${dir}/snippet-${n++}.bas`;
                client.open(uri, expand(s.body));
                // (diagnostics are published once the server has nothing else waiting)
                const errors = (await client.diagnostics(uri, 120)).filter((d) => d.severity === 1 && !/Include file not found/.test(d.message));
                if (errors.length) bad.push(`${name} (${[].concat(s.prefix)[0]}): ${errors.map((e) => `${e.range.start.line + 1}: ${e.message}`).join('; ')}`);
            }
            assert.deepStrictEqual(bad, []);
        } finally {
            client.close();
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });
});
