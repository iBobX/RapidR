// A command line for the terminal's shell, each argument quoted the way that
// shell reads it (unit-tested in test/unit/shell.test.js).
'use strict';

/** 'pwsh' | 'cmd' | 'fish' | 'posix', from the shell's path (VS Code's env.shell). */
function shellKind(shellPath, platform) {
    const s = String(shellPath || '').toLowerCase().replace(/\\/g, '/');
    const base = s.slice(s.lastIndexOf('/') + 1);
    if (/^(pwsh|powershell)(\.exe)?$/.test(base)) return 'pwsh';
    if (/^cmd(\.exe)?$/.test(base)) return 'cmd';
    if (/^fish$/.test(base)) return 'fish';
    if (base === '') return platform === 'win32' ? 'pwsh' : 'posix';
    return 'posix';
}

function quote(kind, arg) {
    const a = String(arg);
    switch (kind) {
        case 'pwsh':
            return `'${a.replace(/'/g, "''")}'`;
        case 'cmd':
            // (Windows paths can't hold a double quote; % and ^ are taken literally inside quotes
            // except %VAR%, which cmd expands anyway: it's what a user typing it would get)
            return `"${a.replace(/"/g, '""')}"`;
        case 'fish':
            return `'${a.replace(/\\/g, '\\\\').replace(/'/g, "\\'")}'`;
        default:
            return `'${a.replace(/'/g, `'\\''`)}'`;
    }
}

/** The whole line: `exe args…`, PowerShell calling the executable with `&`. */
function commandLine(kind, exe, args) {
    const parts = [quote(kind, exe), ...args.map((a) => quote(kind, a))].join(' ');
    return kind === 'pwsh' ? `& ${parts}` : parts;
}

module.exports = { shellKind, quote, commandLine };
