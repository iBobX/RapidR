// Command lines for the "RapidR" terminal (src/shell.js).
'use strict';

const assert = require('assert');
const { shellKind, commandLine } = require('../../src/shell');

describe('shellKind', () => {
    it('knows the shells by their path', () => {
        assert.strictEqual(shellKind('/bin/zsh', 'darwin'), 'posix');
        assert.strictEqual(shellKind('/usr/bin/bash', 'linux'), 'posix');
        assert.strictEqual(shellKind('/opt/homebrew/bin/fish', 'darwin'), 'fish');
        assert.strictEqual(shellKind('C:\\Program Files\\PowerShell\\7\\pwsh.exe', 'win32'), 'pwsh');
        assert.strictEqual(shellKind('C:\\WINDOWS\\System32\\WindowsPowerShell\\v1.0\\powershell.exe', 'win32'), 'pwsh');
        assert.strictEqual(shellKind('C:\\WINDOWS\\System32\\cmd.exe', 'win32'), 'cmd');
        assert.strictEqual(shellKind('C:\\Program Files\\Git\\bin\\bash.exe', 'win32'), 'posix');
        assert.strictEqual(shellKind('', 'win32'), 'pwsh');
        assert.strictEqual(shellKind(undefined, 'linux'), 'posix');
    });
});

describe('commandLine', () => {
    it('quotes for POSIX shells', () => {
        assert.strictEqual(commandLine('posix', '/Applications/RapidR.app/Contents/MacOS/rapidr', ['run', "/Users/u/My Programs/it's.bas"]),
            "'/Applications/RapidR.app/Contents/MacOS/rapidr' 'run' '/Users/u/My Programs/it'\\''s.bas'");
    });
    it('quotes for fish', () => {
        assert.strictEqual(commandLine('fish', '/r', ["it's"]), "'/r' 'it\\'s'");
    });
    it('calls with & in PowerShell', () => {
        assert.strictEqual(commandLine('pwsh', 'C:\\R\\rapidr.exe', ['run', "C:\\My Programs\\it's.bas"]),
            "& 'C:\\R\\rapidr.exe' 'run' 'C:\\My Programs\\it''s.bas'");
    });
    it('double-quotes for cmd', () => {
        assert.strictEqual(commandLine('cmd', 'C:\\R\\rapidr.exe', ['build', 'C:\\a b\\x.bas', '--release']),
            '"C:\\R\\rapidr.exe" "build" "C:\\a b\\x.bas" "--release"');
    });
});
