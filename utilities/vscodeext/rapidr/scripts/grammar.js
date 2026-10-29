// Keeps the TextMate grammar (syntaxes/rapidr.tmLanguage.json) in step with
// RapidR's language registry (docs/ide-plan.md I0: one source of truth). The
// grammar's word lists are hand-ordered and stay as they are; every
// registry name the grammar doesn't know yet — a builtin, a statement, a
// keyword, a component, a global object — is added to the list its kind
// belongs to, so a new RapidR release highlights its own new words.
//
//   node scripts/grammar.js             add what is missing (RAPIDR_PATH, or ../../../target/debug/rapidr)
//   node scripts/grammar.js --check     list what is missing; exit 1 if any (the unit test does this)
//   node scripts/grammar.js --registry registry.json   a `rapidr lang export --json` already made
'use strict';

const { execFileSync } = require('child_process');
const fs = require('fs');
const path = require('path');

const ext = path.join(__dirname, '..');
const GRAMMAR = path.join(ext, 'syntaxes', 'rapidr.tmLanguage.json');

/** The registry's builtin groups -> the scope of the grammar's pattern. */
const BUILTIN_SCOPES = {
    Strings: 'support.function.string.rapidr',
    Numbers: 'support.function.math.rapidr',
    'Files and folders': 'support.function.io.rapidr',
    Console: 'support.function.io.rapidr',
    System: 'support.function.system.rapidr',
    'Dialogs and sound': 'support.function.gui.rapidr',
    Colours: 'support.function.gui.rapidr',
    'Arrays and memory': 'support.function.array.rapidr',
};
const OTHER_BUILTIN_SCOPE = 'support.function.builtin.rapidr';

/** The registry's statement groups -> the scope of the grammar's pattern. */
const STATEMENT_SCOPES = {
    Declarations: 'keyword.declaration.rapidr',
    'Types and objects': 'keyword.declaration.rapidr',
    Routines: 'keyword.control.block.rapidr',
    Flow: 'keyword.control.flow.rapidr',
};
const OTHER_STATEMENT_SCOPE = 'keyword.other.rapidr';

function loadRegistry(argv) {
    const i = argv.indexOf('--registry');
    if (i >= 0) return JSON.parse(fs.readFileSync(path.resolve(argv[i + 1]), 'utf8'));
    const exe = path.resolve(process.env.RAPIDR_PATH || path.join(ext, '..', '..', '..', 'target', 'debug', process.platform === 'win32' ? 'rapidr.exe' : 'rapidr'));
    if (!fs.existsSync(exe)) throw new Error(`no rapidr at ${exe}: build it (cargo build -p rapidr-cli), or set RAPIDR_PATH, or pass --registry`);
    return JSON.parse(execFileSync(exe, ['lang', 'export', '--json'], { maxBuffer: 64 * 1024 * 1024 }).toString());
}

/** A pattern that lists words (`\b(A|B|C)\b`, or the components' `[QR](?:…)`), not a symbol. */
function isWordList(match) {
    return match.includes('\\b(') || match.includes('[QR](?:');
}

/** The grammar's word-list patterns by scope name: { repository entry, pattern } (a scope another pattern shares, like the operators', is the word list). */
function patternsByScope(grammar) {
    const out = new Map();
    for (const [key, rule] of Object.entries(grammar.repository)) {
        for (const p of rule.patterns || []) if (p.name && p.match && isWordList(p.match)) out.set(p.name, { key, pattern: p });
    }
    return out;
}

/** The index of the parenthesis closing the one opened at `open`. */
function closing(text, open) {
    let depth = 0;
    for (let i = open; i < text.length; i++) {
        if (text[i] === '\\') i++;
        else if (text[i] === '(') depth++;
        else if (text[i] === ')' && --depth === 0) return i;
    }
    return -1;
}

/** The alternation a pattern's `match` holds: the group after `\b` (or after `[QR]` for components). */
function alternation(match) {
    const at = match.indexOf('[QR](?:') >= 0 ? match.indexOf('[QR](?:') + 4 : match.indexOf('\\b(') + 2;
    const end = closing(match, at);
    return { at, end, words: match.slice(at, end).replace(/^\((\?:)?/, '').split('|') };
}

/** The word a regex alternative spells, upper case, without a `$` suffix. */
function plain(alt) {
    return alt.replace(/\\s\+/g, ' ').replace(/\\\$\??|\$$/g, '').replace(/[^\w ]/g, '').toUpperCase();
}

/** What the grammar knows: every plain word of every pattern's alternation (the component pattern's with a Q and an R). */
function known(patterns) {
    const set = new Set();
    for (const { pattern } of patterns.values()) {
        const isComponent = pattern.match.includes('[QR](?:');
        let words;
        try {
            words = alternation(pattern.match).words;
        } catch {
            continue;
        }
        for (const w of words) {
            const p = plain(w);
            set.add(p);
            if (isComponent) {
                set.add('Q' + p);
                set.add('R' + p);
            }
        }
    }
    return set;
}

/** What the registry has that the grammar hasn't: { scope: [alternatives] }. */
function missing(registry, patterns) {
    const have = known(patterns);
    const add = new Map();
    const want = (scope, regex, plainName = plain(regex)) => {
        if (have.has(plainName)) return;
        have.add(plainName);
        if (!add.has(scope)) add.set(scope, []);
        // (a component has a Q name and an R name, one word in the grammar)
        if (!add.get(scope).includes(regex)) add.get(scope).push(regex);
    };
    const escape = (name) => name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&').replace(/\\\$$/, '\\$?');
    // Names that are also members of components (Color, Pos, Tab, Out …): as plain
    // words they read as the property they usually are, so they aren't coloured as
    // functions here (the language server's semantic tokens say which they are).
    const members = new Set();
    for (const c of registry.components) {
        for (const list of [c.properties, c.methods, c.events]) for (const m of list || []) members.add(m.name.toUpperCase());
    }
    // (REM and the Rust block have patterns of their own)
    const handled = new Set(['REM', 'RUSTSTART', 'RUSTEND']);
    for (const b of registry.builtins) {
        const bare = b.name.replace(/\$$/, '').toUpperCase();
        if (members.has(bare) || handled.has(bare)) continue;
        want(BUILTIN_SCOPES[b.group] || OTHER_BUILTIN_SCOPE, escape(b.name));
    }
    for (const s of registry.statements) {
        const words = s.name.replace(/\s*#$/, '').split(/\s+/);
        if (handled.has(s.name.toUpperCase())) continue;
        const scope = STATEMENT_SCOPES[s.group] || OTHER_STATEMENT_SCOPE;
        if (words.length === 1) want(scope, escape(words[0]));
        else if (!words.every((w) => have.has(w.toUpperCase()))) want(scope, words.map(escape).join('\\s+'), words.join(' ').toUpperCase());
    }
    for (const k of registry.keywords) {
        if (handled.has(k.name.toUpperCase())) continue;
        const scope = { operator: 'keyword.operator.arithmetic.rapidr', control: 'keyword.control.loop.rapidr' }[k.kind] || 'keyword.declaration.rapidr';
        if (k.name === 'IS') want('keyword.operator.logical.rapidr', 'IS');
        else if (k.name === 'THIS' || k.name === 'SUPER') want('variable.language.rapidr', k.name);
        else want(scope, escape(k.name));
    }
    for (const t of registry.types) want('storage.type.rapidr', escape(t.name));
    for (const c of registry.components) {
        if (c.kind === 'planned') continue;
        for (const name of [c.name, c.rapidq].filter(Boolean)) {
            if (have.has(name.toUpperCase())) continue;
            // (the component pattern takes the name without its Q or R)
            want('entity.name.type.component.rapidr', name.slice(1).toUpperCase(), name.toUpperCase());
        }
    }
    for (const g of registry.globals) want('support.class.global.rapidr', escape(g.name));
    for (const i of registry.items || []) want('entity.name.type.item.rapidr', escape(i.name));
    return add;
}

/** The repository entry a pattern that doesn't exist yet is added to (beside its siblings). */
const NEW_PATTERN_RULE = {
    [OTHER_BUILTIN_SCOPE]: 'builtin-functions',
    'variable.language.rapidr': 'keywords-other',
    'support.class.global.rapidr': 'component-types',
    'entity.name.type.item.rapidr': 'component-types',
};

function applyAdditions(grammar, add) {
    const patterns = patternsByScope(grammar);
    for (const [scope, list] of add) {
        const sorted = [...list].sort();
        const existing = patterns.get(scope);
        if (!existing) {
            const rule = NEW_PATTERN_RULE[scope];
            if (!rule) throw new Error(`no pattern for ${scope}`);
            grammar.repository[rule].patterns.push({ name: scope, match: `(?i)\\b(${sorted.join('|')})\\b` });
            continue;
        }
        const { end } = alternation(existing.pattern.match);
        existing.pattern.match = existing.pattern.match.slice(0, end) + '|' + sorted.join('|') + existing.pattern.match.slice(end);
    }
}

function main() {
    const argv = process.argv.slice(2);
    const registry = loadRegistry(argv);
    const grammar = JSON.parse(fs.readFileSync(GRAMMAR, 'utf8'));
    const add = missing(registry, patternsByScope(grammar));
    const count = [...add.values()].reduce((n, l) => n + l.length, 0);
    if (argv.includes('--check')) {
        for (const [scope, list] of add) console.error(`${scope}: ${list.join(' ')}`);
        if (count > 0) {
            console.error(`the grammar lacks ${count} name(s) the registry has (node scripts/grammar.js)`);
            process.exit(1);
        }
        console.log('the grammar has every name of the registry');
        return;
    }
    if (count === 0) {
        console.log('the grammar has every name of the registry');
        return;
    }
    applyAdditions(grammar, add);
    fs.writeFileSync(GRAMMAR, JSON.stringify(grammar, null, 2) + '\n');
    console.log(`added ${count} name(s) to ${path.relative(process.cwd(), GRAMMAR)}`);
}

if (require.main === module) main();

module.exports = { missing, patternsByScope, known, applyAdditions };
