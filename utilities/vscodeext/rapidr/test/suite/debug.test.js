// Debugging through `rapidr dap` beyond the basics (extension.test.js has
// those): conditional breakpoints, hit counts, logpoints, stepping in / over
// / out, and what a launch configuration passes (arguments, environment,
// folder). Each test runs a real program under the debugger.
'use strict';

const assert = require('assert');
const path = require('path');
const vscode = require('vscode');
const { FIXTURES, activate, open, at, eventually } = require('./util');

/** Collects the debug adapter's events and its launch request while a session runs. */
function track() {
    const events = [];
    const launches = [];
    const tracker = vscode.debug.registerDebugAdapterTrackerFactory('rapidr', {
        createDebugAdapterTracker: () => ({
            onDidSendMessage: (m) => {
                if (m.type === 'event') events.push(m);
            },
            onWillReceiveMessage: (m) => {
                if (m.type === 'request' && m.command === 'launch') launches.push(m.arguments);
            },
        }),
    });
    return {
        events,
        launches,
        stops: () => events.filter((e) => e.event === 'stopped'),
        output: () => events.filter((e) => e.event === 'output').map((e) => e.body.output).join(''),
        ended: () => events.some((e) => e.event === 'terminated'),
        dispose: () => tracker.dispose(),
    };
}

async function frame(session) {
    const st = await session.customRequest('stackTrace', { threadId: 1 });
    return st.stackFrames[0];
}

async function local(session, name) {
    const top = await frame(session);
    const scopes = await session.customRequest('scopes', { frameId: top.id });
    const vars = await session.customRequest('variables', { variablesReference: scopes.scopes[0].variablesReference });
    const v = vars.variables.find((x) => x.name.toLowerCase() === name.toLowerCase());
    return v && v.value;
}

describe('debugging, more', () => {
    before(async () => {
        await activate();
    });

    afterEach(() => {
        vscode.debug.removeBreakpoints(vscode.debug.breakpoints);
    });

    async function startOn(file, breakpoint, start = {}) {
        const doc = await open(file);
        vscode.debug.removeBreakpoints(vscode.debug.breakpoints);
        if (breakpoint) vscode.debug.addBreakpoints([breakpoint(doc)]);
        const t = track();
        const ok = await vscode.debug.startDebugging(undefined, { type: 'rapidr', request: 'launch', name: file, program: doc.uri.fsPath, ...start });
        assert.ok(ok, 'the debug session did not start');
        return { doc, t };
    }

    const sourceBreakpoint = (text, extra = {}) => (doc) => {
        const line = at(doc, text).line;
        return new vscode.SourceBreakpoint(new vscode.Location(doc.uri, new vscode.Position(line, 0)), true, extra.condition, extra.hitCondition, extra.logMessage);
    };

    it('stops on a conditional breakpoint only when its condition is true', async () => {
        const { t } = await startOn('debug.bas', sourceBreakpoint('total = total + i', { condition: 'i = 2' }));
        try {
            await eventually(() => t.stops().length >= 1, 'the stop where i = 2');
            const session = vscode.debug.activeDebugSession;
            assert.strictEqual(await local(session, 'i'), '2');
            vscode.debug.removeBreakpoints(vscode.debug.breakpoints);
            await session.customRequest('continue', { threadId: 1 });
            await eventually(() => t.ended(), 'the end of the program');
            assert.strictEqual(t.stops().length, 1, 'one stop only: i = 1 and i = 3 are not 2');
            assert.match(t.output(), /total\s*6/);
        } finally {
            t.dispose();
        }
    });

    it('stops on the nth hit of a breakpoint (a hit count)', async () => {
        const { t } = await startOn('debug.bas', sourceBreakpoint('total = total + i', { hitCondition: '3' }));
        try {
            await eventually(() => t.stops().length >= 1, 'the third hit');
            assert.strictEqual(await local(vscode.debug.activeDebugSession, 'i'), '3');
            vscode.debug.removeBreakpoints(vscode.debug.breakpoints);
            await vscode.debug.activeDebugSession.customRequest('continue', { threadId: 1 });
            await eventually(() => t.ended(), 'the end of the program');
        } finally {
            t.dispose();
        }
    });

    it('prints a logpoint\'s message, with the values of {expressions}, and goes on', async () => {
        const { t } = await startOn('debug.bas', sourceBreakpoint('total = total + i', { logMessage: 'adding {i} to {total}' }));
        try {
            await eventually(() => t.ended(), 'the end of the program');
            assert.strictEqual(t.stops().length, 0, 'a logpoint never stops');
            const out = t.output();
            for (const line of ['adding 1 to 0', 'adding 2 to 1', 'adding 3 to 3']) assert.ok(out.includes(line), `no "${line}" in: ${out}`);
            assert.match(out, /total\s*6/);
        } finally {
            t.dispose();
        }
    });

    it('steps in to a SUB, over a line, and out again', async () => {
        const { t } = await startOn('debug.bas', sourceBreakpoint('AddUp 3'));
        try {
            await eventually(() => t.stops().length >= 1, 'the stop at the call');
            const session = vscode.debug.activeDebugSession;
            const call = await frame(session);
            assert.strictEqual(call.line, at(await open('debug.bas'), 'AddUp 3').line + 1);
            const step = async (command) => {
                const n = t.stops().length;
                await session.customRequest(command, { threadId: 1 });
                await eventually(() => t.stops().length > n, `the stop after ${command}`);
                return frame(session);
            };
            const inside = await step('stepIn');
            assert.match(inside.name, /AddUp/i, `inside: ${inside.name}`);
            const program = await open('debug.bas');
            const body = at(program, 'SUB AddUp').line + 1; // (from 1: the line of SUB AddUp itself)
            assert.ok(inside.line >= body && inside.line < call.line, `stopped on line ${inside.line}, inside the SUB (${body}…${call.line - 1})`);
            const over = await step('next');
            assert.match(over.name, /AddUp/i, 'stepping over a line stays in the SUB');
            assert.notStrictEqual(over.line, inside.line);
            const out = await step('stepOut');
            assert.doesNotMatch(out.name, /AddUp/i, `out: ${out.name}`);
            vscode.debug.removeBreakpoints(vscode.debug.breakpoints);
            await session.customRequest('continue', { threadId: 1 });
            await eventually(() => t.ended(), 'the end of the program');
        } finally {
            t.dispose();
        }
    });

    it('stops on the first statement with stopOnEntry', async () => {
        const { t } = await startOn('debug.bas', null, { stopOnEntry: true });
        try {
            await eventually(() => t.stops().length >= 1, 'the stop on entry');
            assert.strictEqual(t.stops()[0].body.reason, 'entry');
            // (a known gap, not asserted: the frame's line at the entry is past the end of the file for
            // debug.bas — 15 of 13 lines — and for hello.bas; the debugger session's, not this extension's)
            await vscode.debug.activeDebugSession.customRequest('continue', { threadId: 1 });
            await eventually(() => t.ended(), 'the end of the program');
        } finally {
            t.dispose();
        }
    });

    it('passes the launch configuration\'s arguments, environment and folder to the program', async () => {
        const { t } = await startOn('launch.bas', null, { args: ['first', 'second one'], env: { RVSC_ENV: 'from launch.json' }, cwd: path.dirname(FIXTURES) });
        try {
            await eventually(() => t.ended(), 'the end of the program');
            const out = t.output();
            for (const line of ['count=2', 'arg1=first', 'arg2=second one', 'env=from launch.json', `cwd=${path.dirname(FIXTURES)}`]) {
                assert.ok(out.includes(line), `no "${line}" in: ${out}`);
            }
        } finally {
            t.dispose();
        }
    });

    it('runs without debugging: breakpoints are ignored', async () => {
        const { t } = await startOn('debug.bas', sourceBreakpoint('total = total + i'), { noDebug: true });
        try {
            await eventually(() => t.ended(), 'the end of the program');
            assert.strictEqual(t.stops().length, 0);
            assert.match(t.output(), /total\s*6/);
        } finally {
            t.dispose();
        }
    });
});
