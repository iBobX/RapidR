// RapidR's version numbers (src/version.js) and which rapidr the extension
// runs (chooseRapidr in src/locate.js), with a fake file system and fake
// versions: no VS Code, no rapidr needed.
'use strict';

const assert = require('assert');
const { MINIMUM_RAPIDR, parseVersion, compareVersions, majorMinor, assess } = require('../../src/version');
const { chooseRapidr, candidates } = require('../../src/locate');

describe('versions', () => {
    it('reads what `rapidr version` prints', () => {
        assert.deepStrictEqual(parseVersion('2.121.3'), [2, 121, 3]);
        assert.deepStrictEqual(parseVersion('v2.121.3-dev'), [2, 121, 3]);
        assert.deepStrictEqual(parseVersion('0.1.0'), [0, 1, 0]);
        assert.deepStrictEqual(parseVersion('2.121'), [2, 121, 0]);
        assert.strictEqual(parseVersion('rapidr'), null);
        assert.strictEqual(parseVersion(''), null);
    });

    it('compares numbers, not text (2.99 is older than 2.121)', () => {
        assert.strictEqual(compareVersions('2.121.3', '2.99.9'), 1);
        assert.strictEqual(compareVersions('2.99.9', '2.121.0'), -1);
        assert.strictEqual(compareVersions('2.121.3', '2.121.3'), 0);
        assert.strictEqual(compareVersions('3.0.0', '2.999.9'), 1);
        assert.strictEqual(compareVersions('junk', '0.0.1'), -1);
    });

    it('needs the extension\'s major.minor, and at least the language server\'s release', () => {
        assert.strictEqual(majorMinor('2.121.3'), '2.121.0');
        assert.strictEqual(assess('2.121.3', '2.121.3'), 'current');
        assert.strictEqual(assess('2.121.0', '2.121.3'), 'current', 'a patch release is the same release');
        assert.strictEqual(assess('2.122.0', '2.121.3'), 'current');
        assert.strictEqual(assess('2.119.0', '2.121.3'), 'older');
        assert.strictEqual(assess(MINIMUM_RAPIDR, '2.121.3'), 'older');
        assert.strictEqual(assess('2.116.9', '2.121.3'), 'unusable');
        assert.strictEqual(assess('0.1.0', '2.121.3'), 'unusable', 'the ancient cargo-installed rapidr');
        assert.strictEqual(assess('nonsense', '2.121.3'), 'unusable');
    });
});

function fake(files, dirs = []) {
    const set = new Set(files);
    const dset = new Set(dirs);
    return { isExecutable: (f) => set.has(f), isDirectory: (f) => dset.has(f), realpath: (f) => f };
}

/** versionOf(file) from a table; a missing entry doesn't run. */
const versions = (table) => (file) => (file in table ? Promise.resolve(table[file]) : Promise.reject(new Error('no such program')));

const OLD = '/Users/u/.cargo/bin/rapidr';
const STUDIO = '/Applications/RapidR Studio.app/Contents/MacOS/rapidr';
const RUNTIME = '/Applications/RapidR Runtime.app/Contents/MacOS/rapidr';
const MAC = { platform: 'darwin', home: '/Users/u' };

describe('chooseRapidr', () => {
    it('uses the installed RapidR when an ancient one is first on PATH (the 0.1.0 in ~/.cargo/bin)', async () => {
        const r = await chooseRapidr(
            { env: { PATH: '/Users/u/.cargo/bin:/usr/bin' }, ...MAC, ...fake([OLD, STUDIO, RUNTIME]) },
            '2.121.3',
            versions({ [OLD]: '0.1.0', [STUDIO]: '2.121.3', [RUNTIME]: '2.121.3' }),
        );
        assert.strictEqual(r.path, STUDIO);
        assert.strictEqual(r.version, '2.121.3');
        assert.strictEqual(r.source, 'install');
        assert.strictEqual(r.state, 'current');
        assert.deepStrictEqual(r.skipped.map((s) => [s.path, s.version]), [[OLD, '0.1.0']]);
        assert.match(r.skipped[0].error, /too old/);
    });

    it('takes the first current one, and asks no other (one process on the usual machine)', async () => {
        const asked = [];
        const r = await chooseRapidr(
            { env: { PATH: '/usr/local/bin' }, ...MAC, ...fake(['/usr/local/bin/rapidr', STUDIO]) },
            '2.121.3',
            (f) => { asked.push(f); return Promise.resolve('2.121.3'); },
        );
        assert.strictEqual(r.path, '/usr/local/bin/rapidr');
        assert.strictEqual(r.source, 'PATH');
        assert.deepStrictEqual(asked, ['/usr/local/bin/rapidr']);
        assert.deepStrictEqual(r.skipped, []);
    });

    it('prefers a current install to an older one that still has the language server', async () => {
        const r = await chooseRapidr(
            { env: { PATH: '/Users/u/bin' }, ...MAC, ...fake(['/Users/u/bin/rapidr', STUDIO]) },
            '2.121.3',
            versions({ '/Users/u/bin/rapidr': '2.119.0', [STUDIO]: '2.121.3' }),
        );
        assert.strictEqual(r.path, STUDIO);
        assert.strictEqual(r.state, 'current');
        assert.deepStrictEqual(r.skipped.map((s) => s.path), ['/Users/u/bin/rapidr']);
    });

    it('uses an older one that has the language server when nothing is current, saying so', async () => {
        const r = await chooseRapidr(
            { env: { PATH: '/Users/u/bin' }, ...MAC, ...fake(['/Users/u/bin/rapidr', STUDIO]) },
            '2.121.3',
            versions({ '/Users/u/bin/rapidr': '2.118.0', [STUDIO]: '2.119.0' }),
        );
        assert.strictEqual(r.path, STUDIO, 'the newest of the older ones');
        assert.strictEqual(r.state, 'older');
        assert.strictEqual(r.version, '2.119.0');
        assert.ok(!r.skipped.some((s) => s.path === STUDIO), 'the one in use is not a skipped one');
    });

    it('says what is wrong when every rapidr found is too old', async () => {
        const r = await chooseRapidr({ env: { PATH: '/Users/u/.cargo/bin' }, ...MAC, ...fake([OLD]) }, '2.121.3', versions({ [OLD]: '0.1.0' }));
        assert.strictEqual(r.path, null);
        assert.strictEqual(r.state, 'none');
        assert.match(r.error, /\.cargo\/bin\/rapidr.*too old.*2\.117\.0/);
    });

    it('says when a rapidr found doesn\'t run, and goes on to the next', async () => {
        const r = await chooseRapidr({ env: { PATH: '/broken' }, ...MAC, ...fake(['/broken/rapidr', STUDIO]) }, '2.121.3', versions({ [STUDIO]: '2.121.3' }));
        assert.strictEqual(r.path, STUDIO);
        assert.match(r.skipped[0].error, /didn't run/);
    });

    it('never replaces rapidr.path, and offers the installed RapidR when the setting is too old', async () => {
        const r = await chooseRapidr(
            { setting: OLD, env: { PATH: '/usr/bin' }, ...MAC, ...fake([OLD, STUDIO]) },
            '2.121.3',
            versions({ [OLD]: '0.1.0', [STUDIO]: '2.121.3' }),
        );
        assert.strictEqual(r.path, null, 'an explicit setting is never silently replaced');
        assert.deepStrictEqual(r.alternative, { path: STUDIO, version: '2.121.3', source: 'install', state: 'current' });
        assert.match(r.error, /too old/);
    });

    it('uses a rapidr.path that is older than the extension but has the language server', async () => {
        const r = await chooseRapidr({ setting: STUDIO, env: {}, ...MAC, ...fake([STUDIO]) }, '2.121.3', versions({ [STUDIO]: '2.119.0' }));
        assert.strictEqual(r.path, STUDIO);
        assert.strictEqual(r.state, 'older');
    });

    it('says a rapidr.path that isn\'t a program', async () => {
        const r = await chooseRapidr({ setting: '/nope/rapidr', env: {}, ...MAC, ...fake([STUDIO]) }, '2.121.3', versions({ [STUDIO]: '2.121.3' }));
        assert.strictEqual(r.path, null);
        assert.match(r.error, /rapidr\.path is set to "\/nope\/rapidr"/);
        assert.strictEqual(r.alternative.path, STUDIO);
    });

    it('lists the same file once, through links', () => {
        const real = (f) => (f === '/usr/local/bin/rapidr' ? STUDIO : f);
        const found = candidates({ env: { PATH: '/usr/local/bin' }, ...MAC, ...fake(['/usr/local/bin/rapidr', STUDIO]), realpath: real });
        assert.deepStrictEqual(found.list.map((c) => c.path), ['/usr/local/bin/rapidr']);
    });

    it('finds the Windows and Linux installs', async () => {
        const win = 'C:\\Program Files\\RapidR\\bin\\rapidr.exe';
        const w = await chooseRapidr(
            { env: { ProgramFiles: 'C:\\Program Files' }, platform: 'win32', home: 'C:\\Users\\u', ...fake([win]) },
            '2.121.3', versions({ [win]: '2.121.3' }),
        );
        assert.strictEqual(w.path, win);
        const l = await chooseRapidr({ env: { PATH: '/usr/bin' }, platform: 'linux', home: '/home/u', ...fake(['/usr/bin/rapidr']) }, '2.121.3', versions({ '/usr/bin/rapidr': '2.121.3' }));
        assert.strictEqual(l.path, '/usr/bin/rapidr');
    });
});
