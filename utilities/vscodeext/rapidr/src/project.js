// RapidR projects (`.rrproj`, format 2, TOML: docs/manual/studio-projects.md)
// as the extension needs them: the name, the main file, the run settings.
// No VS Code API here (unit-tested in test/unit/project.test.js), and no
// TOML package: the file is a flat set of strings, booleans, numbers and
// arrays of strings in tables, which this reads; anything else it ignores,
// as RapidR does with the keys it doesn't know.
'use strict';

const nodePath = require('path');

/** A TOML value of the subset: "basic", 'literal', true/false, a number, an array of those. */
function parseValue(text) {
    const t = text.trim();
    if (t.startsWith('[')) {
        const items = [];
        let rest = t.slice(1);
        for (;;) {
            rest = rest.replace(/^[\s,]+/, '');
            if (rest === '' || rest.startsWith(']')) break;
            const { value, length } = parseScalar(rest);
            if (length === 0) break;
            items.push(value);
            rest = rest.slice(length);
        }
        return items;
    }
    return parseScalar(t).value;
}

function parseScalar(text) {
    const quote = text[0];
    if (quote === '"') {
        let out = '';
        let i = 1;
        for (; i < text.length && text[i] !== '"'; i++) {
            if (text[i] !== '\\') {
                out += text[i];
                continue;
            }
            const next = text[++i];
            if (next === 'n') out += '\n';
            else if (next === 't') out += '\t';
            else if (next === 'r') out += '\r';
            else if (next === 'u' && /^[0-9a-fA-F]{4}$/.test(text.slice(i + 1, i + 5))) {
                out += String.fromCharCode(parseInt(text.slice(i + 1, i + 5), 16));
                i += 4;
            } else out += next;
        }
        return { value: out, length: i + 1 };
    }
    if (quote === "'") {
        const end = text.indexOf("'", 1);
        return end < 0 ? { value: text.slice(1), length: text.length } : { value: text.slice(1, end), length: end + 1 };
    }
    const m = /^[^\s,\]]+/.exec(text);
    if (!m) return { value: undefined, length: 0 };
    const word = m[0];
    if (word === 'true') return { value: true, length: word.length };
    if (word === 'false') return { value: false, length: word.length };
    if (/^[+-]?\d+(\.\d+)?$/.test(word)) return { value: Number(word), length: word.length };
    return { value: word, length: word.length };
}

/** A line without its comment (a # inside a string stays). */
function stripComment(line) {
    let quote = '';
    for (let i = 0; i < line.length; i++) {
        const c = line[i];
        if (quote) {
            if (c === '\\' && quote === '"') i++;
            else if (c === quote) quote = '';
        } else if (c === '"' || c === "'") quote = c;
        else if (c === '#') return line.slice(0, i);
    }
    return line;
}

/** Reads the TOML subset: { key: value, table: { … }, files: [ { … } ] } (`[[files]]` is an array of tables). */
function parseToml(text) {
    const root = {};
    let table = root;
    const lines = String(text).split(/\r?\n/);
    for (let i = 0; i < lines.length; i++) {
        let line = stripComment(lines[i]).trim();
        if (line === '') continue;
        const array = /^\[\[\s*([\w.-]+)\s*\]\]$/.exec(line);
        const head = /^\[\s*([\w.-]+)\s*\]$/.exec(line);
        if (array) {
            table = {};
            (root[array[1]] = root[array[1]] || []).push(table);
            continue;
        }
        if (head) {
            table = root[head[1]] = root[head[1]] && !Array.isArray(root[head[1]]) ? root[head[1]] : {};
            continue;
        }
        const kv = /^([\w.-]+|"[^"]*")\s*=\s*(.*)$/.exec(line);
        if (!kv) continue;
        let value = kv[2];
        // (an array over several lines: until its bracket closes)
        while (value.trim().startsWith('[') && !closes(value) && i + 1 < lines.length) value += ' ' + stripComment(lines[++i]).trim();
        table[kv[1].replace(/^"|"$/g, '')] = parseValue(value);
    }
    return root;
}

/** Whether an array's brackets are balanced (outside strings). */
function closes(value) {
    let depth = 0;
    let quote = '';
    for (let i = 0; i < value.length; i++) {
        const c = value[i];
        if (quote) {
            if (c === '\\' && quote === '"') i++;
            else if (c === quote) quote = '';
        } else if (c === '"' || c === "'") quote = c;
        else if (c === '[') depth++;
        else if (c === ']') depth--;
    }
    return depth <= 0;
}

/**
 * A `.rrproj`'s text as what the extension uses:
 * { name, main, args: [..], cwd, rapidqCompatible, targets: [..] }
 * (`main`, `cwd` as the project file spells them: relative to its folder).
 */
function readProject(text) {
    const toml = parseToml(text);
    const run = toml.run || {};
    const compat = toml.compat || {};
    const build = toml.build || {};
    return {
        name: typeof toml.name === 'string' ? toml.name : '',
        main: typeof toml.main === 'string' ? toml.main : '',
        args: Array.isArray(run.args) ? run.args.map(String) : [],
        cwd: typeof run.cwd === 'string' ? run.cwd : '',
        rapidqCompatible: compat.rapidq_compatible === true,
        targets: Array.isArray(build.targets) ? build.targets.map(String) : [],
    };
}

/** Where the project's main file and working folder are, on disk. */
function projectPaths(projectFile, project) {
    const dir = nodePath.dirname(projectFile);
    return {
        dir,
        main: project.main ? nodePath.resolve(dir, project.main) : '',
        cwd: project.cwd ? nodePath.resolve(dir, project.cwd) : dir,
    };
}

module.exports = { parseToml, readProject, projectPaths };
