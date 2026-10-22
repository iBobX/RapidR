//! The reference tells the truth (`rapidr lang truth`, docs/preview4-plan.md
//! K7). RAI's language card and every model's knowledge come from this
//! registry, so a wrong signature here teaches every model a wrong call.
//! This module plans the checks; the CLI runs them (it has the compiler and
//! the VM).
//!
//! - **Every `syntax` line** of a statement, a directive and a builtin is made
//!   into programs and must compile. A metavariable (`name`, `expression`,
//!   `Number`, `Search$`) gets a valid stand-in; `[optional]` parts and
//!   `A|B` choices are tried one at a time (and all optional parts at once).
//!   The grammar of a syntax line (what the check can read):
//!   - an ALL-CAPS word is a keyword, any other word a metavariable;
//!   - `[ … ]` is optional; `A|B|C` without spaces is a choice of one word;
//!     ` | ` with spaces separates whole alternatives of its group;
//!   - `…` (or `...`) after a `,` or `;` means more of the same, between a
//!     header and its `END` a block of statements;
//!   - a trailing `(also …)` note is prose and left out.
//! - **Every `example`** compiles (a component's, a method's, a builtin's).
//!   One with an **`expect`** facet — the output it prints — is also *run*, and
//!   must print exactly that. That is the proof for the calls that two
//!   arguments of the same type make easy to get backwards (`Cell(row, col)`,
//!   `Iloc(start, end)`, `MID$(text, start, count)`): swapped, the program
//!   still compiles and prints something else.
//! - **The coverage rule** for the families where that trap lives: a method
//!   (or builtin) with two parameters of one kind has an `expect`
//!   ([`COVERED`]).
//!
//! The runtimes accept any number of arguments to a builtin or a component's
//! method (a missing one reads as its default, an extra one is ignored), so
//! the compiler and the runtime can't be asked "is this the right count"; the
//! only proof of a signature is a program whose output depends on every
//! argument, which is what `expect` is.

use crate::*;
use std::collections::BTreeMap;

/// What a case checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum What {
    /// A `syntax` line's variant.
    Syntax,
    /// An `example` (compiled; run when it has an `expect`).
    Example,
}

/// How far a case goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Run {
    /// It must compile.
    Compile,
    /// It must compile, run and print this.
    Execute(String),
}

/// One program (or the alternative forms of one use) to check.
#[derive(Clone, Debug)]
pub struct Case {
    /// The registry entry: `statement FOR`, `builtin MID$`, `RDATAFRAME.Iloc`.
    pub entry: String,
    pub what: What,
    /// What was documented, as the report quotes it.
    pub text: String,
    /// Programs that mean the same use written in different positions (a
    /// builtin as a statement, or inside PRINT): the case passes when one
    /// compiles.
    pub forms: Vec<String>,
    pub run: Run,
}

/// What the check will do.
#[derive(Default, Debug)]
pub struct Plan {
    pub cases: Vec<Case>,
    /// Entries not checked (or not run), and why.
    pub skipped: Vec<(String, String)>,
    /// What the planning itself found wrong (a syntax line it can't read, a
    /// trap without an `expect`): (entry, what).
    pub problems: Vec<(String, String)>,
    /// The coverage rule's numbers: argument-order traps in the covered
    /// families, with and without an `expect`; and everywhere else.
    pub traps_covered: usize,
    pub traps_elsewhere: usize,
}

/// The families whose argument-order traps must all have an `expect`
/// (components by registry name; builtins by group).
pub const COVERED_COMPONENTS: &[&str] = &["RDATAFRAME", "RNUM", "RCANVAS", "RBITMAP"];
pub const COVERED_BUILTIN_GROUPS: &[&str] = &["Strings"];
/// Where the rule is documented, for the messages.
pub const COVERED: &str = "RDATAFRAME, RNUM, RCANVAS, RBITMAP and the string builtins";

// --- reading a syntax line ---------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
enum T {
    Word(String),
    Punct(char),
    Str(String),
    Open,
    Close,
    Bar,
    Dots,
}

#[derive(Clone, Debug)]
struct Tk {
    t: T,
    /// Whitespace came before it.
    sp: bool,
}

fn tokenize(s: &str) -> Vec<Tk> {
    let chars: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    let mut sp = false;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            sp = true;
            i += 1;
            continue;
        }
        let t = if c == '[' {
            i += 1;
            T::Open
        } else if c == ']' {
            i += 1;
            T::Close
        } else if c == '|' {
            i += 1;
            T::Bar
        } else if c == '…' {
            i += 1;
            T::Dots
        } else if c == '.' && chars[i..].starts_with(&['.', '.', '.']) {
            i += 3;
            T::Dots
        } else if c == '"' {
            let mut j = i + 1;
            while j < chars.len() && chars[j] != '"' {
                j += 1;
            }
            let text: String = chars[i..(j + 1).min(chars.len())].iter().collect();
            i = (j + 1).min(chars.len());
            T::Str(text)
        } else if c.is_alphanumeric() || c == '_' || c == '$' && chars.get(i + 1).is_some_and(|n| n.is_alphabetic()) {
            let mut j = i + 1;
            while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_') {
                j += 1;
            }
            // (one BASIC type suffix: `Search$`, `Fore%`)
            if j < chars.len() && matches!(chars[j], '$' | '%' | '&' | '!') {
                j += 1;
            }
            let w: String = chars[i..j].iter().collect();
            i = j;
            T::Word(w)
        } else {
            i += 1;
            T::Punct(c)
        };
        out.push(Tk { t, sp });
        sp = false;
    }
    out
}

/// A parsed syntax line: a sequence of tokens and choice groups.
#[derive(Clone, Debug)]
enum Node {
    Tok(Tk),
    /// A choice: `[optional]` (a choice of `0`: absent) or alternatives.
    Group(Group),
}

#[derive(Clone, Debug)]
struct Group {
    id: usize,
    optional: bool,
    /// Alternatives written `A|B` without spaces, which a further `|B` extends.
    tight: bool,
    alts: Vec<Vec<Node>>,
}

struct Parser {
    toks: Vec<Tk>,
    pos: usize,
    next_id: usize,
}

impl Parser {
    /// A sequence up to `]` (when `bracket`) or the end, as its alternatives
    /// (split at ` | `).
    fn seq(&mut self, bracket: bool) -> Vec<Vec<Node>> {
        let mut alts: Vec<Vec<Node>> = vec![Vec::new()];
        while self.pos < self.toks.len() {
            let tk = self.toks[self.pos].clone();
            self.pos += 1;
            match tk.t {
                T::Close if bracket => break,
                T::Open => {
                    let id = self.next_id;
                    self.next_id += 1;
                    let inner = self.seq(true);
                    alts.last_mut().unwrap().push(Node::Group(Group { id, optional: true, tight: false, alts: inner }));
                }
                T::Bar => {
                    let spaced = tk.sp || self.toks.get(self.pos).is_some_and(|n| n.sp);
                    if spaced {
                        alts.push(Vec::new());
                        continue;
                    }
                    // (a choice of the word before and the word after)
                    let cur = alts.last_mut().unwrap();
                    let Some(prev) = cur.pop() else { continue };
                    let Some(next) = self.toks.get(self.pos).cloned() else {
                        cur.push(prev);
                        continue;
                    };
                    self.pos += 1;
                    match prev {
                        Node::Group(mut g) if g.tight && !g.optional => {
                            g.alts.push(vec![Node::Tok(next)]);
                            cur.push(Node::Group(g));
                        }
                        other => {
                            let id = self.next_id;
                            self.next_id += 1;
                            cur.push(Node::Group(Group { id, optional: false, tight: true, alts: vec![vec![other], vec![Node::Tok(next)]] }));
                        }
                    }
                }
                _ => alts.last_mut().unwrap().push(Node::Tok(tk)),
            }
        }
        alts
    }
}

/// One choice point: how many choices it has, and what it needs set above it.
struct Choice {
    id: usize,
    optional: bool,
    count: usize,
    /// (parent group id, the choice of the parent this one is inside).
    parent: Option<(usize, usize)>,
}

fn collect(nodes: &[Node], parent: Option<(usize, usize)>, out: &mut Vec<Choice>) {
    for n in nodes {
        if let Node::Group(g) = n {
            out.push(Choice { id: g.id, optional: g.optional, count: g.alts.len() + usize::from(g.optional), parent });
            for (k, alt) in g.alts.iter().enumerate() {
                collect(alt, Some((g.id, k + usize::from(g.optional))), out);
            }
        }
    }
}

fn render(nodes: &[Node], sel: &BTreeMap<usize, usize>, out: &mut Vec<Tk>) {
    for n in nodes {
        match n {
            Node::Tok(t) => out.push(t.clone()),
            Node::Group(g) => {
                let c = sel.get(&g.id).copied().unwrap_or(0);
                let alt = if g.optional {
                    if c == 0 {
                        continue;
                    }
                    c - 1
                } else {
                    c
                };
                if let Some(a) = g.alts.get(alt) {
                    render(a, sel, out);
                }
            }
        }
    }
}

/// The variants of a syntax line: every optional part absent, every optional
/// part present, and each choice (an optional part, an alternative) taken
/// alone.
fn variants(root: &[Vec<Node>]) -> Vec<Vec<Tk>> {
    // (the top level's own ` | `: a group nobody wrote brackets for)
    let top = if root.len() > 1 { vec![Node::Group(Group { id: usize::MAX, optional: false, tight: false, alts: root.to_vec() })] } else { root[0].clone() };
    let mut choices = Vec::new();
    collect(&top, None, &mut choices);
    let mut sels: Vec<BTreeMap<usize, usize>> = vec![BTreeMap::new()];
    // (all optional parts present)
    sels.push(choices.iter().filter(|c| c.optional).map(|c| (c.id, 1)).collect());
    for c in &choices {
        for k in 1..c.count {
            let mut sel = BTreeMap::new();
            sel.insert(c.id, k);
            // (what it sits inside must be present, in the right alternative)
            let mut at = c.parent;
            while let Some((pid, pk)) = at {
                sel.insert(pid, pk);
                at = choices.iter().find(|x| x.id == pid).and_then(|x| x.parent);
            }
            sels.push(sel);
        }
    }
    let mut seen: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for sel in sels {
        let mut toks = Vec::new();
        render(&top, &sel, &mut toks);
        let key = toks.iter().map(|t| format!("{:?}", t.t)).collect::<String>();
        if !seen.contains(&key) {
            seen.push(key);
            out.push(toks);
        }
    }
    out
}

// --- stand-ins -----------------------------------------------------------------------

/// What the metavariables of one syntax line become.
struct Ctx {
    /// The statement or builtin the line documents (upper case).
    head: String,
    /// Fresh names handed out (`n1`, `n2` …).
    fresh: usize,
    /// The last metavariable's stem: what `, …` repeats.
    last: String,
}

/// Statements whose `name` is one they declare.
const DECLARING: &[&str] = &[
    "DIM", "REDIM", "STATIC", "CONST", "DEFINT", "DEFSTR", "DEFBYTE", "DEFWORD", "DEFDWORD", "DEFLNG", "DEFLONG", "DEFSHORT", "DEFSNG", "DEFDBL", "DEFCUR", "GLOBAL",
    "TYPE", "SUB", "FUNCTION", "SUBI", "FUNCTIONI", "DECLARE", "CREATE", "PROPERTY SET", "$DEFINE", "$UNDEF", "$IFDEF", "$IFNDEF", "$MACRO",
];

fn split_suffix(w: &str) -> (&str, Option<char>) {
    match w.chars().last() {
        Some(c @ ('$' | '%' | '&' | '!')) if w.len() > 1 => (&w[..w.len() - 1], Some(c)),
        _ => (w, None),
    }
}

/// Whether a word is a keyword (upper case letters, at least two) rather than
/// a metavariable.
fn is_keyword(w: &str) -> bool {
    let (stem, _) = split_suffix(w);
    let letters: Vec<char> = stem.chars().filter(|c| c.is_alphabetic()).collect();
    w.starts_with('$') || (letters.len() >= 2 && letters.iter().all(|c| c.is_uppercase()))
}

/// Metavariables that stand for text.
const TEXT: &[&str] = &[
    "string", "str", "text", "source", "search", "find", "match", "replace", "replacestr", "withstr", "hex", "path", "file", "filename", "dir", "dllname", "title", "msg",
    "command", "char", "s", "searchstring", "findstring", "stringexpression", "stringexpresion", "deliminator", "prompt", "comment", "module", "widestring",
];

/// What a metavariable becomes: a value of the right kind for the line.
fn stand_in(word: &str, ctx: &mut Ctx) -> String {
    let (stem, suffix) = split_suffix(word);
    let l = stem.to_ascii_lowercase();
    ctx.last = l.clone();
    let head = ctx.head.as_str();
    match l.as_str() {
        // a name the statement declares, or one it uses
        "name" if DECLARING.contains(&head) => {
            ctx.fresh += 1;
            format!("n{}", ctx.fresh)
        }
        "name" if head == "CALL" => "mysub".into(),
        "name" if head == "$OPTION" => "x".into(),
        "name" => "x".into(),
        "alias" => "al".into(),
        "variable" | "var" | "varname" | "pointer" => "v".into(),
        "a" => "a".into(),
        "b" => "b".into(),
        "array" | "arrayname" => "arr".into(),
        "routine" => "mysub".into(),
        "type" if head == "CREATE" => "QFORM".into(),
        "type" if head == "EXTENDS" => "QOBJECT".into(),
        "type" => "INTEGER".into(),
        "object" => "f".into(),
        "member" => "OnClick".into(),
        "mode" => "INPUT".into(),
        "label" => "lbl".into(),
        "condition" => "1 = 1".into(),
        "params" => "p1 AS INTEGER".into(),
        "bounds" => "3".into(),
        _ if suffix == Some('$') => "\"x\"".into(),
        _ if TEXT.contains(&l.as_str()) => "\"x\"".into(),
        _ => "1".into(),
    }
}

/// What `…` is: more of the last list, nothing, or a block's body.
fn body_for(head: &str) -> &'static str {
    match head {
        "TYPE" => "f1 AS INTEGER",
        "CREATE" => "Caption = \"x\"",
        "WITH" => ".Caption = \"x\"",
        _ => "PRINT 1",
    }
}

const CLOSERS: &[&str] = &["END", "NEXT", "WEND", "LOOP", "RUSTEND", "RETURN"];

/// Writes a variant as source text.
fn finish(toks: &[Tk], ctx: &mut Ctx) -> String {
    let block = toks.iter().any(|t| matches!(&t.t, T::Word(w) if CLOSERS.contains(&w.as_str())));
    let mut out = String::new();
    let mut prev: Option<&T> = None;
    for (i, tk) in toks.iter().enumerate() {
        let sp = if tk.sp && !out.is_empty() && !out.ends_with('\n') { " " } else { "" };
        match &tk.t {
            T::Word(w) => {
                let text = if is_keyword(w) { w.clone() } else { stand_in(w, ctx) };
                out.push_str(sp);
                out.push_str(&text);
            }
            T::Punct(c) => {
                out.push_str(sp);
                out.push(*c);
            }
            T::Str(s) => {
                out.push_str(sp);
                out.push_str(s);
            }
            T::Dots => {
                let after_sep = matches!(prev, Some(T::Punct(',' | ';')));
                if block {
                    let next_is_case = matches!(toks.get(i + 1).map(|t| &t.t), Some(T::Word(w)) if w == "CASE");
                    let after_meta = matches!(prev, Some(T::Word(w)) if !is_keyword(w));
                    if matches!(prev, Some(T::Punct('('))) {
                        // (an empty parameter list)
                    } else if next_is_case && after_meta {
                        out.push('\n');
                    } else {
                        out.push('\n');
                        out.push_str(body_for(&ctx.head));
                        out.push('\n');
                    }
                } else if after_sep {
                    // (more of the last list)
                    let more = match ctx.last.as_str() {
                        "name" => {
                            ctx.fresh += 1;
                            format!("n{}", ctx.fresh)
                        }
                        "variable" | "var" => "v".into(),
                        _ => "2".into(),
                    };
                    out.push(' ');
                    out.push_str(&more);
                }
            }
            T::Open | T::Close | T::Bar => {}
        }
        prev = Some(&tk.t);
    }
    out
}

// --- the lines of source around a statement ---------------------------------------------

/// What a statement's line needs around it to be a program: names it uses,
/// a label to go to.
const PRELUDE: &str = "DIM v AS INTEGER\nDIM a AS INTEGER\nDIM b AS INTEGER\nDIM s AS STRING\nDIM arr(10) AS INTEGER\nCREATE f AS QFORM\nEND CREATE\nSUB mysub\nEND SUB\n";
const POSTLUDE: &str = "\nlbl:\nPRINT 0\n";

/// The statement's text inside the program it needs (EXIT FOR in a FOR …).
fn wrap_statement(head: &str, text: &str) -> String {
    let inner = match head {
        "EXIT" => {
            let what = text.split_whitespace().nth(1).unwrap_or("");
            match what {
                "FOR" => format!("FOR v = 1 TO 2\n{text}\nNEXT v"),
                "DO" => format!("DO\n{text}\nLOOP"),
                "WHILE" => format!("WHILE 1\n{text}\nWEND"),
                "SUB" => format!("SUB mysub2\n{text}\nEND SUB"),
                "FUNCTION" => format!("FUNCTION myfn2 AS INTEGER\n{text}\nEND FUNCTION"),
                _ => text.to_string(),
            }
        }
        _ => text.to_string(),
    };
    format!("{PRELUDE}{inner}{POSTLUDE}")
}

// --- statements that can't be derived from their syntax line ----------------------------------

/// A statement whose syntax line is prose, or which only exists inside another
/// (EVENT in a TYPE): fixed programs instead, with the reason. Each still has
/// to compile.
const FIXED: &[(&str, &str, &[&str])] = &[
    ("PUBLIC", "only inside a TYPE; its syntax line is prose", &["TYPE t1\n  PUBLIC:\n  f1 AS INTEGER\n  PRIVATE:\n  f2 AS INTEGER\nEND TYPE\n"]),
    ("PROPERTY SET", "only inside a TYPE", &["TYPE t1 EXTENDS QBUTTON\n  PROPERTY SET Tag2(value AS INTEGER)\n    PRINT value\n  END PROPERTY\nEND TYPE\n"]),
    ("EVENT", "only inside a TYPE that extends a component", &["TYPE t1 EXTENDS QBUTTON\n  EVENT OnClick\n    PRINT 1\n  END EVENT\nEND TYPE\n"]),
    ("CONSTRUCTOR", "only inside a TYPE", &["TYPE t1\n  f1 AS INTEGER\n  CONSTRUCTOR\n    f1 = 1\n  END CONSTRUCTOR\nEND TYPE\n"]),
    ("RUSTSTART", "its body is Rust, for native builds", &["RUSTSTART\nlet x = 1;\nRUSTEND\n"]),
];

// --- planning --------------------------------------------------------------------------------------

fn statement_cases(plan: &mut Plan) {
    for s in STATEMENTS {
        let entry = format!("statement {}", s.name);
        if let Some((_, why, programs)) = FIXED.iter().find(|(n, _, _)| *n == s.name) {
            plan.skipped.push((entry.clone(), format!("syntax line not derived: {why}; a fixed program is compiled instead")));
            for p in *programs {
                plan.cases.push(Case { entry: entry.clone(), what: What::Syntax, text: p.trim().to_string(), forms: vec![format!("{PRELUDE}{p}")], run: Run::Compile });
            }
            continue;
        }
        for text in syntax_programs(s.name, s.syntax, &entry, plan) {
            let forms = vec![wrap_statement(s.name, &text)];
            plan.cases.push(Case { entry: entry.clone(), what: What::Syntax, text, forms, run: Run::Compile });
        }
    }
}

/// A syntax line's note: `(also …)` and `(Rust code …)`, prose that isn't part of it.
fn strip_note(syntax: &str) -> &str {
    for key in [" (also ", " (Rust "] {
        if let Some(i) = syntax.find(key) {
            return syntax[..i].trim_end();
        }
    }
    syntax
}

/// Each variant of a syntax line, as source (the statements' body or the
/// builtins' call).
fn syntax_programs(head: &str, syntax: &str, entry: &str, plan: &mut Plan) -> Vec<String> {
    let line = strip_note(syntax);
    let mut p = Parser { toks: tokenize(line), pos: 0, next_id: 0 };
    let root = p.seq(false);
    if p.pos < p.toks.len() || root.iter().all(Vec::is_empty) {
        plan.problems.push((entry.to_string(), format!("the syntax line `{syntax}` can't be read (an unbalanced `[`?)")));
        return Vec::new();
    }
    let mut out = Vec::new();
    for v in variants(&root) {
        let mut ctx = Ctx { head: head.to_string(), fresh: 0, last: String::new() };
        let text = finish(&v, &mut ctx);
        if !out.contains(&text) {
            out.push(text);
        }
    }
    out
}

fn directive_cases(plan: &mut Plan) {
    for d in DIRECTIVES {
        let entry = format!("directive {}", d.name);
        // (a directive's line is a line of its own, and the block ones close)
        let closing = match d.name {
            "$IFDEF" | "$IFNDEF" => "\n$ENDIF",
            "$ELSE" | "$ENDIF" => {
                plan.skipped.push((entry, "closes an $IFDEF: checked with it".into()));
                continue;
            }
            "$INCLUDE" | "$RESOURCE" => {
                plan.skipped.push((entry, "names a file: the program that includes it is run in the examples suite".into()));
                continue;
            }
            _ => "",
        };
        for text in syntax_programs(d.name, d.syntax, &entry, plan) {
            plan.cases.push(Case { entry: entry.clone(), what: What::Syntax, forms: vec![format!("{text}{closing}\nPRINT 1\n")], text, run: Run::Compile });
        }
    }
}

/// How a builtin's arguments read, for the coverage rule: a metavariable that
/// stands for text, or a number.
fn arg_kinds(syntax: &str) -> Vec<bool> {
    // (the arguments of `NAME(a, b [, c])` or `NAME a, b`: its words that
    // aren't keywords, in the order written, optional ones included)
    let toks = tokenize(strip_note(syntax));
    let mut ctx = Ctx { head: String::new(), fresh: 0, last: String::new() };
    let mut out = Vec::new();
    for (i, t) in toks.iter().enumerate() {
        if let T::Word(w) = &t.t {
            if i == 0 || is_keyword(w) {
                continue;
            }
            out.push(stand_in(w, &mut ctx).starts_with('"'));
        }
    }
    out
}

fn builtin_cases(plan: &mut Plan) {
    for b in BUILTINS.iter().filter(|b| !b.missing && !b.name.starts_with("__") && !b.syntax.is_empty()) {
        let entry = format!("builtin {}", b.name);
        // (written with parentheses it is a function: `PRINT MID$(…)`; else
        // a statement; either may be right for a name that is both)
        let name = b.name;
        for text in syntax_programs(name, b.syntax, &entry, plan) {
            let mut forms = vec![format!("{PRELUDE}{text}{POSTLUDE}")];
            let value = text.trim_start().starts_with(&format!("{name}(")) || b.bare || b.returns.is_some() || name.ends_with('$') || text.trim() == name;
            if value {
                forms.push(format!("{PRELUDE}PRINT {text}{POSTLUDE}"));
                forms.push(format!("{PRELUDE}v = {text}{POSTLUDE}"));
            }
            plan.cases.push(Case { entry: entry.clone(), what: What::Syntax, text, forms, run: Run::Compile });
        }
    }
}

/// Executed when it has an `expect`, else only compiled.
fn example_case(plan: &mut Plan, entry: String, example: &str, expect: &str, effects: &[Effect], whole: bool) {
    let source = example.trim_end().to_string() + "\n";
    let forms = vec![if whole { source } else { format!("{PRELUDE}{source}") }];
    let run = if expect.is_empty() {
        let why = match effects.iter().find(|e| e.is_capability() || matches!(e, Effect::Ui | Effect::Wait)) {
            Some(e) => format!("compiled, not run: it reaches `{}`", e.as_str()),
            None => "compiled, not run: no `expect` says what it prints".into(),
        };
        plan.skipped.push((entry.clone(), why));
        Run::Compile
    } else {
        Run::Execute(expect.to_string())
    };
    plan.cases.push(Case { entry, what: What::Example, text: example.trim().to_string(), forms, run });
}

fn example_cases(plan: &mut Plan) {
    for c in COMPONENTS.iter().chain(GLOBALS) {
        if !c.example.trim().is_empty() {
            let entry = c.name.to_string();
            let source = c.example.trim_end().to_string() + "\n";
            plan.skipped.push((entry.clone(), "a component's example opens windows: compiled, not run".into()));
            plan.cases.push(Case { entry, what: What::Example, text: c.example.trim().to_string(), forms: vec![source], run: Run::Compile });
        }
        for m in c.methods.iter().filter(|m| !m.example.trim().is_empty()) {
            example_case(plan, format!("{}.{}", c.name, m.name), m.example, m.expect, m.effects, true);
        }
    }
    for b in BUILTINS.iter().filter(|b| !b.example.trim().is_empty()) {
        example_case(plan, format!("builtin {}", b.name), b.example, b.expect, b.effects, true);
    }
}

/// One trap found: counted, and a problem when it is in a covered family and has no `expect`.
fn trap(plan: &mut Plan, entry: String, covered_family: bool, has_expect: bool) {
    if covered_family {
        plan.traps_covered += 1;
        if !has_expect {
            plan.problems.push((entry, format!("two parameters of one kind, so a swapped call still compiles: it needs an `example` and an `expect` ({COVERED} are covered)")));
        }
    } else {
        plan.traps_elsewhere += 1;
    }
}

/// The coverage rule: the argument-order traps of the covered families.
fn trap_rule(plan: &mut Plan) {
    for c in COMPONENTS {
        if c.kind != Kind::Component {
            continue;
        }
        let covered = COVERED_COMPONENTS.contains(&c.name);
        for m in c.methods.iter().filter(|m| !m.missing && !m.params.is_empty() && !matches!(m.test, Test::Skip(_))) {
            // (parameters of one kind: the same type, an untyped one counting as its own kind)
            let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
            for p in m.params.iter().filter(|p| !p.variadic) {
                *kinds.entry(p.ty.to_string()).or_default() += 1;
            }
            if kinds.values().any(|n| *n >= 2) {
                trap(plan, format!("{}.{}", c.name, m.name), covered, !m.expect.is_empty());
            }
        }
    }
    for b in BUILTINS.iter().filter(|b| !b.missing && !b.name.starts_with("__") && !b.syntax.is_empty()) {
        let kinds = arg_kinds(b.syntax);
        let texts = kinds.iter().filter(|t| **t).count();
        if texts >= 2 || kinds.len() - texts >= 2 {
            trap(plan, format!("builtin {}", b.name), COVERED_BUILTIN_GROUPS.contains(&b.group), !b.expect.is_empty());
        }
    }
}

/// Everything the check does.
pub fn plan() -> Plan {
    let mut plan = Plan::default();
    statement_cases(&mut plan);
    directive_cases(&mut plan);
    builtin_cases(&mut plan);
    example_cases(&mut plan);
    trap_rule(&mut plan);
    plan
}

/// The plan's text for one syntax line (for tests and `--list`).
pub fn expand(head: &str, syntax: &str) -> Vec<String> {
    let mut plan = Plan::default();
    syntax_programs(head, syntax, head, &mut plan)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(head: &str, syntax: &str) -> Vec<String> {
        expand(head, syntax)
    }

    #[test]
    fn optional_parts_and_choices() {
        let v = lines("DIM", "DIM name[(bounds)] [AS type] [= value] [, …]");
        assert!(v.contains(&"DIM n1".to_string()), "{v:?}");
        assert!(v.contains(&"DIM n1(3) AS INTEGER = 1, n2".to_string()) || v.iter().any(|l| l.contains("(3)") && l.contains(", n2")), "{v:?}");
        let v = lines("EXIT", "EXIT FOR|DO|WHILE|SUB|FUNCTION");
        assert_eq!(v, ["EXIT FOR", "EXIT DO", "EXIT WHILE", "EXIT SUB", "EXIT FUNCTION"]);
        let v = lines("INSTR", "INSTR([Start,] Search$,Find$)");
        assert!(v.contains(&"INSTR(\"x\",\"x\")".to_string()) || v.iter().any(|l| l.starts_with("INSTR(")), "{v:?}");
        assert!(v.iter().any(|l| l.contains("1, \"x\"") || l.contains("1,")), "{v:?}");
    }

    #[test]
    fn const_lists_repeat() {
        let v = lines("CONST", "CONST name = expression [, …]");
        assert!(v.contains(&"CONST n1 = 1".to_string()), "{v:?}");
        assert!(v.contains(&"CONST n1 = 1, n2".to_string()) || v.iter().any(|l| l.contains(", n2")), "{v:?}");
    }

    #[test]
    fn blocks_get_bodies() {
        let v = lines("FOR", "FOR variable = start TO end [STEP step] … NEXT [variable]");
        assert!(v.iter().any(|l| l.starts_with("FOR v = 1 TO 1") && l.contains("\nPRINT 1\nNEXT")), "{v:?}");
        let v = lines("SELECT CASE", "SELECT CASE expression … CASE values … CASE ELSE … END SELECT");
        assert_eq!(v.len(), 1);
        assert!(v[0].starts_with("SELECT CASE 1\n\nCASE 1\nPRINT 1\nCASE ELSE\nPRINT 1\nEND SELECT"), "{:?}", v[0]);
    }

    #[test]
    fn notes_are_left_out() {
        assert_eq!(strip_note("DEFLNG name [= value] [, …] (also DEFLONG)"), "DEFLNG name [= value] [, …]");
        assert_eq!(strip_note("INSTR([Start,] Search$,Find$)"), "INSTR([Start,] Search$,Find$)");
    }

    #[test]
    fn wide_alternatives_split_whole_sequences() {
        let v = lines("ON ERROR", "ON ERROR RESUME NEXT | GOTO label");
        assert_eq!(v, ["ON ERROR RESUME NEXT", "ON ERROR GOTO lbl"]);
    }

    #[test]
    fn every_syntax_line_is_read() {
        let plan = plan();
        let unread: Vec<_> = plan.problems.iter().filter(|(_, w)| w.contains("can't be read")).collect();
        assert!(unread.is_empty(), "{unread:?}");
    }
}
