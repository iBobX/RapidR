// Reading a .rrproj (src/project.js).
'use strict';

const assert = require('assert');
const fs = require('fs');
const path = require('path');
const { parseToml, readProject, projectPaths } = require('../../src/project');

describe('readProject', () => {
    it('reads the name, the main file and the run settings', () => {
        const p = readProject([
            '# RapidR Studio project (format 2)',
            '',
            'format = 2',
            'name = "Shelf # one"  # a comment',
            'main = "src/main.rr"',
            '',
            '[[files]]',
            'path = "main.rr"',
            'kind = "form"',
            '',
            '[run]',
            'args = ["--verbose", \'a b\', "say \\"hi\\""]',
            'cwd = "data"',
            '',
            '[compat]',
            'rapidq_compatible = true',
            '',
            '[build]',
            'targets = [',
            '  "native",',
            '  "web",',
            ']',
        ].join('\n'));
        assert.deepStrictEqual(p, {
            name: 'Shelf # one',
            main: 'src/main.rr',
            args: ['--verbose', 'a b', 'say "hi"'],
            cwd: 'data',
            rapidqCompatible: true,
            targets: ['native', 'web'],
        });
    });

    it('has defaults for what a project leaves out, and ignores what it doesn\'t know', () => {
        assert.deepStrictEqual(readProject('name = "X"\nmain = "m.bas"\nunknown_key = [1, 2]\n[future]\nx = 1\n'),
            { name: 'X', main: 'm.bas', args: [], cwd: '', rapidqCompatible: false, targets: [] });
        assert.deepStrictEqual(readProject(''), { name: '', main: '', args: [], cwd: '', rapidqCompatible: false, targets: [] });
    });

    it('finds the main file and the working folder next to the project', () => {
        const where = projectPaths(path.join('/work', 'Shelf', 'Shelf.rrproj'), { main: 'src/main.rr', cwd: 'data' });
        assert.strictEqual(where.main, path.join('/work', 'Shelf', 'src', 'main.rr'));
        assert.strictEqual(where.cwd, path.join('/work', 'Shelf', 'data'));
        assert.strictEqual(projectPaths('/work/A.rrproj', { main: '', cwd: '' }).cwd, '/work');
    });

    it('reads the projects Studio writes (the repository\'s fixtures)', () => {
        const dir = path.resolve(__dirname, '..', '..', '..', '..', '..', 'tests', 'fixtures');
        const shelf = path.join(dir, 'studio_forms', 'Shelf.rrproj');
        if (!fs.existsSync(shelf)) return; // (outside a checkout)
        const p = readProject(fs.readFileSync(shelf, 'utf8'));
        assert.strictEqual(p.name, 'Shelf');
        assert.strictEqual(p.main, 'main.rr');
        const notes = readProject(fs.readFileSync(path.join(dir, 'studio_app_native', 'Notes.rrproj'), 'utf8'));
        assert.deepStrictEqual(notes.targets, ['native']);
        assert.strictEqual(parseToml('x = 1.5\ny = -3\nz = false').y, -3);
    });
});
