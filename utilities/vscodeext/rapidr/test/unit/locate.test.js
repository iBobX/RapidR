// Finding rapidr (src/locate.js), with a fake file system: no VS Code needed.
'use strict';

const assert = require('assert');
const { findRapidr } = require('../../src/locate');

function fake(files, dirs = []) {
    const set = new Set(files);
    const dset = new Set(dirs);
    return {
        isExecutable: (f) => set.has(f),
        isDirectory: (f) => dset.has(f),
    };
}

describe('findRapidr', () => {
    it('uses rapidr.path first, and never another rapidr when it is wrong', () => {
        const fs = fake(['/opt/x/rapidr', '/usr/bin/rapidr']);
        const env = { PATH: '/usr/bin' };
        assert.deepStrictEqual(
            findRapidr({ setting: '/opt/x/rapidr', env, platform: 'linux', home: '/home/u', ...fs }),
            { path: '/opt/x/rapidr', source: 'setting' },
        );
        const wrong = findRapidr({ setting: '/nope/rapidr', env, platform: 'linux', home: '/home/u', ...fs });
        assert.strictEqual(wrong.path, null);
        assert.match(wrong.error, /rapidr\.path/);
    });

    it('accepts the folder, an install prefix or RapidR.app in rapidr.path', () => {
        const fs = fake(['/Applications/RapidR.app/Contents/MacOS/rapidr', '/opt/r/bin/rapidr'], ['/Applications/RapidR.app', '/opt/r']);
        assert.strictEqual(findRapidr({ setting: '/Applications/RapidR.app', env: {}, platform: 'darwin', home: '/Users/u', ...fs }).path,
            '/Applications/RapidR.app/Contents/MacOS/rapidr');
        assert.strictEqual(findRapidr({ setting: '/opt/r', env: {}, platform: 'linux', home: '/home/u', ...fs }).path, '/opt/r/bin/rapidr');
    });

    it('expands ~ in rapidr.path', () => {
        const fs = fake(['/home/u/bin/rapidr']);
        assert.strictEqual(findRapidr({ setting: '~/bin/rapidr', env: {}, platform: 'linux', home: '/home/u', ...fs }).path, '/home/u/bin/rapidr');
    });

    it('searches PATH in order', () => {
        const fs = fake(['/b/rapidr', '/c/rapidr']);
        const r = findRapidr({ env: { PATH: '/a:/b:/c' }, platform: 'linux', home: '/home/u', ...fs });
        assert.deepStrictEqual(r, { path: '/b/rapidr', source: 'PATH' });
    });

    it('searches PATH on Windows with PATHEXT and quoted folders', () => {
        const fs = fake(['C:\\Tools\\RapidR\\rapidr.exe']);
        const r = findRapidr({
            env: { Path: 'C:\\Windows;"C:\\Tools\\RapidR"', PATHEXT: '.COM;.EXE;.BAT' },
            platform: 'win32', home: 'C:\\Users\\u', ...fs,
        });
        assert.deepStrictEqual(r, { path: 'C:\\Tools\\RapidR\\rapidr.exe', source: 'PATH' });
    });

    it('finds RapidR Studio.app, and the older RapidR.app', () => {
        assert.strictEqual(findRapidr({ env: {}, platform: 'darwin', home: '/Users/u', ...fake(['/Applications/RapidR Studio.app/Contents/MacOS/rapidr']) }).path,
            '/Applications/RapidR Studio.app/Contents/MacOS/rapidr');
        assert.strictEqual(findRapidr({ env: {}, platform: 'darwin', home: '/Users/u', ...fake(['/Applications/RapidR.app/Contents/MacOS/rapidr']) }).path,
            '/Applications/RapidR.app/Contents/MacOS/rapidr');
        const fs = fake(['/Applications/RapidR Studio.app/Contents/MacOS/rapidr'], ['/Applications/RapidR Studio.app']);
        assert.strictEqual(findRapidr({ setting: '/Applications/RapidR Studio.app', env: {}, platform: 'darwin', home: '/Users/u', ...fs }).path,
            '/Applications/RapidR Studio.app/Contents/MacOS/rapidr');
    });

    it('finds the installs', () => {
        assert.strictEqual(findRapidr({ env: {}, platform: 'darwin', home: '/Users/u', ...fake(['/Users/u/Applications/RapidR.app/Contents/MacOS/rapidr']) }).path,
            '/Users/u/Applications/RapidR.app/Contents/MacOS/rapidr');
        assert.strictEqual(findRapidr({ env: {}, platform: 'linux', home: '/home/u', ...fake(['/home/u/.local/bin/rapidr']) }).path,
            '/home/u/.local/bin/rapidr');
        assert.strictEqual(
            findRapidr({ env: { LOCALAPPDATA: 'C:\\Users\\u\\AppData\\Local' }, platform: 'win32', home: 'C:\\Users\\u', ...fake(['C:\\Users\\u\\AppData\\Local\\Programs\\RapidR\\bin\\rapidr.exe']) }).path,
            'C:\\Users\\u\\AppData\\Local\\Programs\\RapidR\\bin\\rapidr.exe');
    });

    it('finds a source checkout in a trusted workspace only', () => {
        const fs = fake(['/src/RapidR/target/debug/rapidr']);
        const opts = { env: {}, platform: 'linux', home: '/home/u', workspaceFolders: ['/src/RapidR'], ...fs };
        assert.deepStrictEqual(findRapidr(opts), { path: '/src/RapidR/target/debug/rapidr', source: 'workspace' });
        assert.strictEqual(findRapidr({ ...opts, trusted: false }).path, null);
    });

    it('prefers a release build to a debug one in the checkout', () => {
        const fs = fake(['/src/RapidR/target/debug/rapidr', '/src/RapidR/target/release/rapidr']);
        assert.strictEqual(findRapidr({ env: {}, platform: 'linux', home: '/h', workspaceFolders: ['/src/RapidR'], ...fs }).path,
            '/src/RapidR/target/release/rapidr');
    });

    it('takes RAPIDR_PATH after the setting', () => {
        const fs = fake(['/dev/rapidr', '/usr/bin/rapidr']);
        assert.deepStrictEqual(findRapidr({ env: { RAPIDR_PATH: '/dev/rapidr', PATH: '/usr/bin' }, platform: 'linux', home: '/h', ...fs }),
            { path: '/dev/rapidr', source: 'RAPIDR_PATH' });
    });

    it('says what it tried when there is none', () => {
        const r = findRapidr({ env: { PATH: '/a' }, platform: 'linux', home: '/h', ...fake([]) });
        assert.strictEqual(r.path, null);
        assert.ok(r.tried.includes('/a/rapidr'));
        assert.ok(r.tried.includes('/usr/bin/rapidr'));
    });
});
