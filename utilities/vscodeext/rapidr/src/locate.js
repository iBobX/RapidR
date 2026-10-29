// Finding the `rapidr` executable — no VS Code API here (unit-tested in
// test/unit/locate.test.js), and no child process but `rapidr version`: PATH
// is searched the way `which` / `where` do it.
//
// Order: the `rapidr.path` setting; the RAPIDR_PATH environment variable;
// PATH; the places RapidR's installers put it; a RapidR source checkout open
// in the workspace (only in a trusted workspace).
//
// Finding one isn't enough: an ancient rapidr (0.1.0, from `cargo install`
// years ago) first on PATH is "found" and has no language server. `chooseRapidr`
// asks each candidate its version and takes the first that is as new as the
// extension; an older one that still has `rapidr lsp` is the fallback; one
// too old for the extension is skipped, and said so (never silently).
'use strict';

const fs = require('fs');
const nodePath = require('path');
const os = require('os');
const { execFile } = require('child_process');
const { MINIMUM_RAPIDR, assess, compareVersions } = require('./version');

function pathFor(platform) {
    return platform === 'win32' ? nodePath.win32 : nodePath.posix;
}

function exeName(platform) {
    return platform === 'win32' ? 'rapidr.exe' : 'rapidr';
}

/** A file we may run: it exists, it's a file, and (not on Windows) it's executable. */
function defaultIsExecutable(file, platform) {
    try {
        if (!fs.statSync(file).isFile()) return false;
        if (platform !== 'win32') fs.accessSync(file, fs.constants.X_OK);
        return true;
    } catch {
        return false;
    }
}

function defaultIsDirectory(file) {
    try {
        return fs.statSync(file).isDirectory();
    } catch {
        return false;
    }
}

function defaultRealpath(file) {
    try {
        return fs.realpathSync(file);
    } catch {
        return file;
    }
}

function expandHome(p, home, P) {
    if (p === '~') return home;
    if (p.startsWith('~/') || p.startsWith('~\\')) return P.join(home, p.slice(2));
    return p;
}

/**
 * What `rapidr.path` may name: the executable; the folder it's in (or an
 * install prefix with bin/); RapidR Studio.app (or an older RapidR.app) on macOS.
 */
function candidatesForSetting(value, ctx) {
    const P = pathFor(ctx.platform);
    let p = expandHome(value.trim(), ctx.home, P);
    if (!P.isAbsolute(p) && ctx.workspaceFolders.length > 0) p = P.join(ctx.workspaceFolders[0], p);
    if (!ctx.isDirectory(p)) return [p];
    const exe = exeName(ctx.platform);
    return [P.join(p, exe), P.join(p, 'bin', exe), P.join(p, 'Contents', 'MacOS', 'rapidr')];
}

/** PATH, as `which` (POSIX) or `where` (Windows: PATHEXT, the folder entries may be quoted). */
function searchPath(ctx) {
    const P = pathFor(ctx.platform);
    const env = ctx.env;
    const raw = env.PATH || env.Path || env.path || '';
    const sep = ctx.platform === 'win32' ? ';' : ':';
    const dirs = raw.split(sep).map((d) => d.trim().replace(/^"(.*)"$/, '$1')).filter((d) => d.length > 0);
    const names = ctx.platform === 'win32'
        ? (env.PATHEXT || '.COM;.EXE;.BAT;.CMD').split(';').filter(Boolean).map((ext) => 'rapidr' + ext.toLowerCase())
        : ['rapidr'];
    const out = [];
    for (const d of dirs) for (const n of names) out.push(P.join(d, n));
    return out;
}

/** Where RapidR's installers put it (docs/release-packaging.md). */
function installPlaces(ctx) {
    const P = pathFor(ctx.platform);
    const home = ctx.home;
    switch (ctx.platform) {
        case 'darwin':
            return [
                '/Applications/RapidR Studio.app/Contents/MacOS/rapidr',
                P.join(home, 'Applications', 'RapidR Studio.app', 'Contents', 'MacOS', 'rapidr'),
                // (the Runtime: the same rapidr, without Studio)
                '/Applications/RapidR Runtime.app/Contents/MacOS/rapidr',
                P.join(home, 'Applications', 'RapidR Runtime.app', 'Contents', 'MacOS', 'rapidr'),
                // (before 2.118 the app was called RapidR.app)
                '/Applications/RapidR.app/Contents/MacOS/rapidr',
                P.join(home, 'Applications', 'RapidR.app', 'Contents', 'MacOS', 'rapidr'),
                '/usr/local/bin/rapidr',
                '/opt/homebrew/bin/rapidr',
                P.join(home, '.local', 'bin', 'rapidr'),
            ];
        case 'win32': {
            const local = ctx.env.LOCALAPPDATA || P.join(home, 'AppData', 'Local');
            const out = [P.join(local, 'Programs', 'RapidR', 'bin', 'rapidr.exe')];
            for (const name of ['ProgramFiles', 'ProgramW6432', 'ProgramFiles(x86)']) {
                const dir = ctx.env[name];
                if (dir) out.push(P.join(dir, 'RapidR', 'bin', 'rapidr.exe'));
            }
            return out;
        }
        default:
            return ['/usr/bin/rapidr', '/usr/local/bin/rapidr', P.join(home, '.local', 'bin', 'rapidr')];
    }
}

/** A RapidR source checkout open in the workspace: its link at the root, then cargo's builds. */
function workspacePlaces(ctx) {
    const P = pathFor(ctx.platform);
    const exe = exeName(ctx.platform);
    const out = [];
    for (const folder of ctx.workspaceFolders) {
        out.push(P.join(folder, exe), P.join(folder, 'target', 'release', exe), P.join(folder, 'target', 'debug', exe));
    }
    return out;
}

function contextFor(options) {
    return {
        setting: options.setting || '',
        env: options.env || process.env,
        platform: options.platform || process.platform,
        home: options.home || os.homedir(),
        workspaceFolders: options.workspaceFolders || [],
        trusted: options.trusted !== false,
        isExecutable: options.isExecutable || defaultIsExecutable,
        isDirectory: options.isDirectory || defaultIsDirectory,
        realpath: options.realpath || defaultRealpath,
    };
}

/**
 * Every rapidr there is to find, in the order they are preferred, each
 * once (the same file through two links counts once):
 * { list: [{ path, source }], tried, setting, error }.
 * With a `rapidr.path` setting that is the only candidate (an explicit
 * choice is never silently replaced by another rapidr): `setting: true`,
 * and `error` when it isn't an executable. `tried` lists every place looked at.
 */
function candidates(options = {}) {
    const ctx = contextFor(options);
    const tried = [];
    const seen = new Set();
    const list = [];
    const collect = (places, source, firstOnly) => {
        for (const file of places) {
            tried.push(file);
            if (!ctx.isExecutable(file, ctx.platform)) continue;
            const real = ctx.realpath(file);
            if (seen.has(real)) continue;
            seen.add(real);
            list.push({ path: file, source });
            if (firstOnly) return;
        }
    };
    if (ctx.setting.trim() !== '') {
        collect(candidatesForSetting(ctx.setting, ctx), 'setting', true);
        const error = list.length ? null : `rapidr.path is set to "${ctx.setting}", which isn't an executable rapidr.`;
        return { list, tried, setting: true, error };
    }
    if (ctx.env.RAPIDR_PATH) collect(candidatesForSetting(ctx.env.RAPIDR_PATH, ctx), 'RAPIDR_PATH', true);
    // (every rapidr on PATH, in order: an old one first doesn't hide a newer one behind it)
    collect(searchPath(ctx), 'PATH', false);
    collect(installPlaces(ctx), 'install', false);
    if (ctx.trusted) collect(workspacePlaces(ctx), 'workspace', false);
    return { list, tried, setting: false, error: null };
}

/**
 * The first candidate (see [candidates]), whatever its version. `options`:
 * { setting, env, platform, home, workspaceFolders, trusted,
 * isExecutable(file, platform), isDirectory(file), realpath(file) } — all
 * optional. Returns { path, source } or { path: null, error?, tried }.
 */
function findRapidr(options = {}) {
    const found = candidates(options);
    if (found.list.length) return found.list[0];
    return found.error ? { path: null, tried: found.tried, error: found.error } : { path: null, tried: found.tried };
}

/**
 * Chooses the rapidr to run: the first candidate that is current (its
 * version at least the extension's major.minor); else the newest that still
 * has `rapidr lsp`; else none. `versionOf(file)` -> Promise<version> (it
 * rejects when the file doesn't run; default: [rapidrVersion]).
 *
 * Returns {
 *   path, version, source,   // the choice (path null: none usable)
 *   state,                   // 'current' | 'older' | 'none'
 *   skipped: [{ path, source, version, error }],  // candidates passed over (too old, or didn't run)
 *   alternative,             // the setting's rapidr is unusable: what the search without it finds { path, version, source, state }
 *   error, tried,
 * }
 */
async function chooseRapidr(options = {}, extensionVersion, versionOf = rapidrVersion) {
    const found = candidates(options);
    const result = { path: null, version: null, source: null, state: 'none', skipped: [], alternative: null, error: found.error, tried: found.tried };
    const asked = new Map();
    const ask = (c) => {
        if (!asked.has(c.path)) {
            asked.set(c.path, versionOf(c.path).then((version) => ({ version }), (err) => ({ error: err && err.message ? err.message : String(err) })));
        }
        return asked.get(c.path);
    };
    let best = null;
    for (const c of found.list) {
        const v = await ask(c);
        if (v.error) {
            result.skipped.push({ ...c, version: null, error: `it didn't run: ${v.error}` });
            continue;
        }
        const state = assess(v.version, extensionVersion);
        if (state === 'unusable') {
            result.skipped.push({ ...c, version: v.version, error: `RapidR ${v.version} is too old: the language server (rapidr lsp) needs RapidR ${MINIMUM_RAPIDR} or newer` });
        } else if (state === 'current') {
            // (an older one kept so far is passed over for this one)
            if (best) result.skipped.push({ ...best, error: `RapidR ${best.version} is older than the extension; RapidR ${v.version} is not` });
            best = { ...c, version: v.version, state };
            break;
        } else if (!best || compareVersions(v.version, best.version) > 0) {
            // (usable, but older than the extension: kept in case nothing is current)
            if (best) result.skipped.push({ ...best, error: `RapidR ${v.version} is newer` });
            best = { ...c, version: v.version, state };
        } else {
            result.skipped.push({ ...c, version: v.version, error: `RapidR ${best.version} is newer` });
        }
    }
    if (best) {
        Object.assign(result, { path: best.path, version: best.version, source: best.source, state: best.state });
        // (an older one used because nothing is current: it isn't a skipped one)
        result.skipped = result.skipped.filter((s) => s.path !== best.path);
        return result;
    }
    if (found.setting) {
        // (an unusable rapidr.path: what the search without it finds, to offer)
        const auto = await chooseRapidr({ ...options, setting: '' }, extensionVersion, versionOf);
        if (auto.path) result.alternative = { path: auto.path, version: auto.version, source: auto.source, state: auto.state };
    }
    if (!result.error && result.skipped.length) {
        const s = result.skipped[0];
        result.error = `${s.path}: ${s.error}`;
    }
    return result;
}

/**
 * `rapidr version` → "2.117.0" (it prints "RapidR <version>"); rejects when it doesn't run.
 * (A freshly installed or built rapidr can take several seconds to start the
 * first time while macOS checks it: the timeout is generous.)
 */
function rapidrVersion(file, timeoutMs = 60000) {
    return new Promise((resolve, reject) => {
        execFile(file, ['version'], { timeout: timeoutMs, windowsHide: true }, (err, stdout, stderr) => {
            const m = /RapidR\s+v?([0-9][^\s]*)/i.exec(String(stdout));
            if (m) return resolve(m[1]);
            reject(err || new Error(`"${file} version" printed no version${stderr ? ': ' + String(stderr).trim() : ''}`));
        });
    });
}

module.exports = { findRapidr, candidates, chooseRapidr, rapidrVersion, searchPath, installPlaces, workspacePlaces, candidatesForSetting };
