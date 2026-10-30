// When typing should open the completion list by itself — no VS Code API here
// (unit-tested in test/unit/triggers.test.js). The server's trigger
// characters are `.` (members) and `$` (directives); a list of types after
// `AS ` has no character of its own (a space ends many things), so the
// extension asks for it when the word AS is finished with a space.
'use strict';

/** Whether `before` ends inside a string or a comment. */
function inStringOrComment(before) {
    if (/^\s*REM\b/i.test(before)) return true;
    let inString = false;
    for (const c of before) {
        if (c === '"') inString = !inString;
        else if (c === "'" && !inString) return true;
    }
    return inString;
}

/**
 * `before` is the line up to the caret, a space just typed: whether it ends
 * with the word AS (`DIM test AS `, `SUB Go(n AS `, `CREATE Form AS `), where
 * what comes next is a type or a component.
 */
function suggestsTypes(before) {
    return /(^|[^\w$%&#!.])as[ \t]$/i.test(before) && !inStringOrComment(before);
}

module.exports = { suggestsTypes, inStringOrComment };
