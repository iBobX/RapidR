// What package.json promises is what the .vsix carries: every file a
// contribution names (grammars, snippets, language configurations, icons) is
// listed by `vsce ls`, and the extension's id in `configurationDefaults` is its
// own. (The integration tests run from source: a file .vscodeignore leaves
// out would only be missing from the installed extension.)
'use strict';

const assert = require('assert');
const { execFileSync } = require('child_process');
const fs = require('fs');
const path = require('path');

const ext = path.resolve(__dirname, '..', '..');
const pkg = JSON.parse(fs.readFileSync(path.join(ext, 'package.json'), 'utf8'));

describe('package.json', () => {
    it('names, as the default formatter, this extension\'s own id', () => {
        assert.strictEqual(pkg.contributes.configurationDefaults['[rapidr]']['editor.defaultFormatter'], `${pkg.publisher}.${pkg.name}`);
    });

    it('has every command it puts in a menu, a keybinding or the palette', () => {
        const ids = new Set(pkg.contributes.commands.map((c) => c.command));
        const used = [
            ...pkg.contributes.keybindings.map((k) => k.command),
            ...Object.values(pkg.contributes.menus).flat().map((m) => m.command),
        ];
        for (const id of used) assert.ok(ids.has(id), `${id} is in a menu or keybinding and is not a command`);
        for (const id of ids) assert.ok(pkg.activationEvents.length > 0 && id.startsWith('rapidr.'), id);
    });

    it('is packaged with every file it contributes', () => {
        const vsce = require.resolve('@vscode/vsce/vsce', { paths: [ext] });
        const listed = new Set(
            execFileSync(process.execPath, [vsce, 'ls', '--no-dependencies'], { cwd: ext }).toString().split(/\r?\n/).map((l) => l.trim().replace(/\\/g, '/')).filter(Boolean),
        );
        const wanted = [];
        const c = pkg.contributes;
        for (const l of c.languages) {
            if (l.configuration) wanted.push(l.configuration);
            if (l.icon) wanted.push(l.icon.light, l.icon.dark);
        }
        for (const g of c.grammars) wanted.push(g.path);
        for (const s of c.snippets) wanted.push(s.path);
        wanted.push(pkg.icon);
        for (const w of wanted) {
            const file = w.replace(/^\.\//, '');
            assert.ok(fs.existsSync(path.join(ext, file)), `${file} doesn't exist`);
            assert.ok(listed.has(file), `${file} is not in the .vsix: .vscodeignore leaves it out`);
        }
    });
});
