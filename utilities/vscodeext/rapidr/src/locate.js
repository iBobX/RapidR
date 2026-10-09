// Finding the `rapidr` executable — no VS Code API here (unit-tested in
// test/unit/locate.test.js), and no child process: PATH is searched the way
// `which` / `where` do it.
//
// Order: the `rapidr.path` setting; the RAPIDR_PATH environment variable;
// PATH; the places RapidR's installers put it; a RapidR source checkout open
// in the workspace (only in a trusted workspace).
'use strict';

const fs = require('fs');
const nodePath = require('path');
const os = require('os');
const { execFile } = require('child_process');

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
            if (ctx.env.ProgramFiles) out.push(P.join(ctx.env.ProgramFiles, 'RapidR', 'bin', 'rapidr.exe'));
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

/**
 * Finds rapidr. `options`: { setting, env, platform, home, workspaceFolders,
 * trusted, isExecutable(file, platform), isDirectory(file) } — all optional.
 * Returns { path, source } or { path: null, error?, tried }.
 */
function findRapidr(options = {}) {
    const ctx = {
        setting: options.setting || '',
        env: options.env || process.env,
        platform: options.platform || process.platform,
        home: options.home || os.homedir(),
        workspaceFolders: options.workspaceFolders || [],
        trusted: options.trusted !== false,
        isExecutable: options.isExecutable || defaultIsExecutable,
        isDirectory: options.isDirectory || defaultIsDirectory,
    };
    const tried = [];
    const first = (list, source) => {
        for (const file of list) {
            tried.push(file);
            if (ctx.isExecutable(file, ctx.platform)) return { path: file, source };
        }
        return null;
    };
    if (ctx.setting.trim() !== '') {
        const hit = first(candidatesForSetting(ctx.setting, ctx), 'setting');
        // (an explicit setting is never silently replaced by another rapidr)
        return hit || { path: null, tried, error: `rapidr.path is set to "${ctx.setting}", which isn't an executable rapidr.` };
    }
    if (ctx.env.RAPIDR_PATH) {
        const hit = first(candidatesForSetting(ctx.env.RAPIDR_PATH, ctx), 'RAPIDR_PATH');
        if (hit) return hit;
    }
    return first(searchPath(ctx), 'PATH')
        || first(installPlaces(ctx), 'install')
        || (ctx.trusted ? first(workspacePlaces(ctx), 'workspace') : null)
        || { path: null, tried };
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

module.exports = { findRapidr, rapidrVersion, searchPath, installPlaces, workspacePlaces, candidatesForSetting };
