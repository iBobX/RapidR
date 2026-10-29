// RapidR's version numbers, and what the extension needs of the rapidr it
// runs — no VS Code API here (unit-tested in test/unit/version.test.js).
'use strict';

/**
 * The oldest rapidr that has `rapidr lsp` and `rapidr dap` (2.117.0 rebuilt
 * the extension on them). Anything older (an ancient `rapidr` 0.1.0 on PATH,
 * say) can't serve the extension at all.
 */
const MINIMUM_RAPIDR = '2.117.0';

/** "2.121.3" / "v2.121.3-dev" -> [2, 121, 3]; null when it isn't a version. */
function parseVersion(text) {
    const m = /^v?(\d+)\.(\d+)(?:\.(\d+))?/.exec(String(text || '').trim());
    if (!m) return null;
    return [Number(m[1]), Number(m[2]), Number(m[3] || 0)];
}

/** -1, 0 or 1; a text that isn't a version is older than any that is. */
function compareVersions(a, b) {
    const x = parseVersion(a);
    const y = parseVersion(b);
    if (!x || !y) return x ? 1 : y ? -1 : 0;
    for (let i = 0; i < 3; i++) {
        if (x[i] !== y[i]) return x[i] < y[i] ? -1 : 1;
    }
    return 0;
}

/** "2.121.3" -> "2.121.0": what a rapidr must at least be to match an extension of that version. */
function majorMinor(version) {
    const v = parseVersion(version);
    return v ? `${v[0]}.${v[1]}.0` : MINIMUM_RAPIDR;
}

/**
 * How well `rapidrVersion` serves an extension of version `extensionVersion`:
 *   'current' - at least the extension's major.minor: everything works;
 *   'older'   - has the language server but predates the extension's
 *               release: it works, some newer features may be missing;
 *   'unusable' - too old to have `rapidr lsp` (or not a version at all).
 */
function assess(rapidrVersion, extensionVersion) {
    if (!parseVersion(rapidrVersion) || compareVersions(rapidrVersion, MINIMUM_RAPIDR) < 0) return 'unusable';
    if (compareVersions(rapidrVersion, majorMinor(extensionVersion)) >= 0) return 'current';
    return 'older';
}

module.exports = { MINIMUM_RAPIDR, parseVersion, compareVersions, majorMinor, assess };
