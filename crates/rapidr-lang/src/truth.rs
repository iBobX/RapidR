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
//!   (or builtin) with two neighbouring parameters of one kind has an `expect`
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
    /// A RapidR program (`.rr`: RAPIDQ.INC's constants built in, `True` -1)
    /// rather than a RapidQ one (`.bas`): the language card's rules.
    pub rapidr: bool,
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
    count: usize,
    /// (parent group id, the choice of the parent this one is inside).
    parent: Option<(usize, usize)>,
}

fn collect(nodes: &[Node], parent: Option<(usize, usize)>, out: &mut Vec<Choice>) {
    for n in nodes {
        if let Node::Group(g) = n {
            out.push(Choice { id: g.id, count: g.alts.len() + usize::from(g.optional), parent });
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

/// The variants of a syntax line: every optional part absent, and each choice
/// (an optional part, an alternative) taken alone. (Not every optional part at
/// once: parts that exclude each other, a single value for an array, would fail
/// for what the program means, not for what the line says.)
fn variants(root: &[Vec<Node>]) -> Vec<Vec<Tk>> {
    // (the top level's own ` | `: a group nobody wrote brackets for)
    let top = if root.len() > 1 { vec![Node::Group(Group { id: usize::MAX, optional: false, tight: false, alts: root.to_vec() })] } else { root[0].clone() };
    let mut choices = Vec::new();
    collect(&top, None, &mut choices);
    let mut sels: Vec<BTreeMap<usize, usize>> = vec![BTreeMap::new()];
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
        "type" if head == "EXTENDS" || head == "TYPE" => "QOBJECT".into(),
        "component" => "QBUTTON".into(),
        "args" => String::new(),
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

/// One written piece of a variant: its text, and whether it is a word or a
/// string (two of them side by side need a space between).
struct Piece {
    text: String,
    wordy: bool,
}

/// The last element of the list a `, …` goes on with: what stands between the
/// list's previous `,` (or its start: after the head, or an opening
/// parenthesis) and the separator that was just written.
fn last_element(pieces: &[Piece]) -> Vec<String> {
    let end = pieces.len().saturating_sub(1);
    let mut depth = 0i32;
    let mut start = 1.min(end);
    for i in (0..end).rev() {
        match pieces[i].text.as_str() {
            ")" => depth += 1,
            "(" if depth == 0 => {
                start = i + 1;
                break;
            }
            "(" => depth -= 1,
            "," if depth == 0 => {
                start = i + 1;
                break;
            }
            _ => {}
        }
    }
    pieces[start..end].iter().map(|p| p.text.clone()).collect()
}

fn is_fresh(w: &str) -> bool {
    w.strip_prefix('n').is_some_and(|d| !d.is_empty() && d.chars().all(|c| c.is_ascii_digit()))
}

/// Writes a variant as source text.
fn finish(toks: &[Tk], ctx: &mut Ctx) -> String {
    let block = toks.iter().any(|t| matches!(&t.t, T::Word(w) if CLOSERS.contains(&w.as_str())));
    let mut pieces: Vec<Piece> = Vec::new();
    let mut prev: Option<&T> = None;
    for (i, tk) in toks.iter().enumerate() {
        match &tk.t {
            T::Word(w) => {
                let text = if is_keyword(w) { w.clone() } else { stand_in(w, ctx) };
                if !text.is_empty() {
                    pieces.push(Piece { text, wordy: true });
                }
            }
            T::Punct(c) => pieces.push(Piece { text: c.to_string(), wordy: *c == '#' }),
            T::Str(s) => pieces.push(Piece { text: s.clone(), wordy: true }),
            T::Dots => {
                let after_sep = matches!(prev, Some(T::Punct(',' | ';')));
                if block {
                    let next_is_case = matches!(toks.get(i + 1).map(|t| &t.t), Some(T::Word(w)) if w == "CASE");
                    // (the body before a SELECT CASE's first CASE is none)
                    let after_meta = matches!(prev, Some(T::Word(w)) if w.eq_ignore_ascii_case("expression"));
                    if matches!(prev, Some(T::Punct('('))) {
                        // (an empty parameter list)
                    } else if next_is_case && after_meta {
                        pieces.push(Piece { text: "\n".into(), wordy: false });
                    } else {
                        pieces.push(Piece { text: format!("\n{}\n", body_for(&ctx.head)), wordy: false });
                    }
                } else if after_sep {
                    // (more of the list: another element like the last)
                    for w in last_element(&pieces) {
                        let w = if is_fresh(&w) {
                            ctx.fresh += 1;
                            format!("n{}", ctx.fresh)
                        } else {
                            w
                        };
                        pieces.push(Piece { text: w, wordy: true });
                    }
                }
            }
            T::Open | T::Close | T::Bar => {}
        }
        prev = Some(&tk.t);
    }
    // (words and strings side by side are apart; `=` has a space each side)
    let mut out = String::new();
    let mut last: Option<&Piece> = None;
    for p in &pieces {
        let newline = p.text.starts_with('\n') || out.ends_with('\n');
        let apart = last.is_some_and(|l| (l.wordy && p.wordy) || p.text == "=" || l.text == "=" || (l.text == "," && p.wordy));
        if !newline && apart {
            out.push(' ');
        }
        out.push_str(&p.text);
        last = Some(p);
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
        // (a clause of TYPE: a TYPE to put it in)
        "EXTENDS" => format!("{text}\nf1 AS INTEGER\nEND TYPE"),
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
    ("RUSTSTART", "its body is Rust: native builds only, and the interpreter this check compiles with refuses it by design", &[]),
];

// --- planning --------------------------------------------------------------------------------------

fn statement_cases(plan: &mut Plan) {
    for s in STATEMENTS {
        let entry = format!("statement {}", s.name);
        if let Some((_, why, programs)) = FIXED.iter().find(|(n, _, _)| *n == s.name) {
            plan.skipped.push((entry.clone(), if programs.is_empty() { format!("not compiled: {why}") } else { format!("syntax line not derived: {why}; a fixed program is compiled instead") }));
            for p in *programs {
                plan.cases.push(Case { rapidr: false, entry: entry.clone(), what: What::Syntax, text: p.trim().to_string(), forms: vec![format!("{PRELUDE}{p}")], run: Run::Compile });
            }
            continue;
        }
        for text in syntax_programs(s.name, s.syntax, &entry, plan) {
            let forms = vec![wrap_statement(s.name, &text)];
            plan.cases.push(Case { rapidr: false, entry: entry.clone(), what: What::Syntax, text, forms, run: Run::Compile });
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
    // (two forms of one call: `STRING$(Num, Char) or STRING$(Num, Char$)`)
    let line = strip_note(syntax).replace(") or ", ") | ");
    let mut p = Parser { toks: tokenize(&line), pos: 0, next_id: 0 };
    let mut root = p.seq(false);
    if p.pos < p.toks.len() || root.iter().all(Vec::is_empty) {
        plan.problems.push((entry.to_string(), format!("the syntax line `{syntax}` can't be read (an unbalanced `[`?)")));
        return Vec::new();
    }
    // (` | ` between whole alternatives: the ones after the first may leave
    // out the head they share, as in `EXIT FOR | DO | WHILE`)
    if root.len() > 1 {
        let head_toks = tokenize(head);
        for alt in root.iter_mut().skip(1) {
            let starts_with_head = matches!((alt.first(), head_toks.first()), (Some(Node::Tok(Tk { t: T::Word(a), .. })), Some(Tk { t: T::Word(h), .. })) if a.eq_ignore_ascii_case(h));
            if !starts_with_head {
                for (k, h) in head_toks.iter().enumerate() {
                    alt.insert(k, Node::Tok(h.clone()));
                }
            }
        }
    }
    let mut out = Vec::new();
    for v in variants(&root) {
        let mut ctx = Ctx { head: head.to_string(), fresh: 0 };
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
            if text.contains("ICON \"") {
                plan.skipped.push((entry.clone(), "`ICON \"file\"` names a file the compiler opens: checked in the examples suite".into()));
                continue;
            }
            plan.cases.push(Case { rapidr: false, entry: entry.clone(), what: What::Syntax, forms: vec![format!("{text}{closing}\nPRINT 1\n")], text, run: Run::Compile });
        }
    }
}

/// A parameter as an example must name it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Slot {
    name: String,
    optional: bool,
    /// Any number more of these (`v1, v2 ...`).
    variadic: bool,
}

impl Slot {
    fn of(p: &Param) -> Slot {
        Slot { name: p.name.to_string(), optional: p.optional, variadic: p.variadic }
    }
}

/// A builtin's arguments as its syntax line names them, one list per form (`A or B` gives two):
/// each metavariable, in the order written, with whether it is optional (in `[ ]`).
fn arg_names(syntax: &str) -> Vec<Vec<Slot>> {
    syntax
        .split(" or ")
        .map(|form| {
            let toks = tokenize(strip_note(form));
            let mut depth = 0usize;
            let mut out: Vec<Slot> = Vec::new();
            for (i, t) in toks.iter().enumerate() {
                match &t.t {
                    T::Open => depth += 1,
                    T::Close => depth = depth.saturating_sub(1),
                    T::Word(w) if i > 0 && !is_keyword(w) => out.push(Slot { name: w.clone(), optional: depth > 0, variadic: false }),
                    T::Dots => {
                        if let Some(last) = out.last_mut() {
                            last.variadic = true;
                        }
                    }
                    _ => {}
                }
            }
            out
        })
        .collect()
}

/// How a builtin's arguments read, for the coverage rule: whether each
/// stands for text (or a number).
fn arg_kinds(syntax: &str) -> Vec<bool> {
    let mut ctx = Ctx { head: String::new(), fresh: 0 };
    arg_names(syntax).concat().iter().map(|s| stand_in(&s.name, &mut ctx).starts_with('"')).collect()
}

/// The arguments of each call of `name` in `source`, as texts: after a `.`
/// for a method, not for a builtin; in parentheses, or (as a statement) the
/// rest of the line up to a `:`.
fn calls(source: &str, name: &str, method: bool) -> Vec<Vec<String>> {
    let chars: Vec<char> = source.chars().collect();
    let lname: Vec<char> = name.to_ascii_lowercase().chars().collect();
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let mut out = Vec::new();
    let mut i = 0;
    let mut quoted = false;
    while i < chars.len() {
        if chars[i] == '"' {
            quoted = !quoted;
        }
        let at_name = !quoted
            && i + lname.len() <= chars.len()
            && chars[i..i + lname.len()].iter().map(|c| c.to_ascii_lowercase()).eq(lname.iter().copied())
            && !chars.get(i + lname.len()).is_some_and(|c| word(*c))
            && match i.checked_sub(1).map(|j| chars[j]) {
                Some('.') => method,
                Some(c) if word(c) || c == '$' => false,
                _ => !method,
            };
        if !at_name {
            i += 1;
            continue;
        }
        let mut j = i + lname.len();
        while chars.get(j) == Some(&' ') {
            j += 1;
        }
        let (start, end) = if chars.get(j) == Some(&'(') {
            let (mut depth, mut k, mut q) = (0, j, false);
            while k < chars.len() {
                match chars[k] {
                    '"' => q = !q,
                    '(' if !q => depth += 1,
                    ')' if !q => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                k += 1;
            }
            (j + 1, k.min(chars.len()))
        } else {
            let (mut k, mut q) = (j, false);
            while k < chars.len() && chars[k] != '\n' && !(chars[k] == ':' && !q) {
                if chars[k] == '"' {
                    q = !q;
                }
                k += 1;
            }
            (j, k)
        };
        let text: String = chars[start..end.max(start)].iter().collect();
        out.push(split_args(&text));
        i = end.max(i + 1);
    }
    out
}

/// A call's argument texts: split at the commas outside quotes and parentheses.
fn split_args(text: &str) -> Vec<String> {
    let (mut out, mut cur, mut depth, mut q) = (Vec::new(), String::new(), 0i32, false);
    for c in text.chars() {
        match c {
            '"' => q = !q,
            '(' | '[' if !q => depth += 1,
            ')' | ']' if !q => depth -= 1,
            _ => {}
        }
        if c == ',' && !q && depth == 0 {
            out.push(cur.trim().to_string());
            cur.clear();
        } else {
            cur.push(c);
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

fn is_identifier(s: &str) -> bool {
    let (stem, _) = split_suffix(s);
    stem.starts_with(|c: char| c.is_alphabetic() || c == '_') && stem.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// Whether the arguments are the parameters' names, in order, leaving out only optional
/// ones; after a `...` parameter any more arguments are fine.
fn names_match(call: &[String], params: &[Slot]) -> bool {
    match (call.split_first(), params.split_first()) {
        (None, _) => params.iter().all(|s| s.optional || s.variadic),
        (Some(_), None) => false,
        (Some(_), Some((s, _))) if s.variadic => true,
        (Some((a, rest)), Some((s, more))) => (a.eq_ignore_ascii_case(&s.name) && names_match(rest, more)) || (s.optional && names_match(call, more)),
    }
}

/// Whether the example calls `name` with arguments that are variables named like the
/// documented parameters, in their order — so that what `expect` proves (that those
/// values, in that order, give that output) is about the documented signature.
fn calls_with_the_names(example: &str, name: &str, method: bool, forms: &[Vec<Slot>]) -> bool {
    calls(example, name, method).iter().any(|call| !call.is_empty() && call.iter().all(|a| is_identifier(a)) && forms.iter().any(|f| names_match(call, f)))
}

fn builtin_cases(plan: &mut Plan) {
    for b in BUILTINS.iter().filter(|b| !b.missing && !b.name.starts_with("__") && !b.syntax.is_empty()) {
        let entry = format!("builtin {}", b.name);
        // (written with parentheses it is a function: `PRINT MID$(…)`; else
        // a statement; either may be right for a name that is both)
        let name = b.name;
        // (the PARAM… functions read a SUBI's or FUNCTIONI's arguments)
        let around = |body: String| {
            if name.starts_with("PARAM") {
                format!("{PRELUDE}FUNCTIONI fi() AS INTEGER\n{body}\nEND FUNCTIONI{POSTLUDE}")
            } else {
                format!("{PRELUDE}{body}{POSTLUDE}")
            }
        };
        for text in syntax_programs(name, b.syntax, &entry, plan) {
            let mut forms = vec![around(text.clone())];
            let value = text.trim_start().starts_with(&format!("{name}(")) || b.bare || b.returns.is_some() || name.ends_with('$') || text.trim() == name;
            if value {
                forms.push(around(format!("PRINT {text}")));
                forms.push(around(format!("v = {text}")));
            }
            plan.cases.push(Case { rapidr: false, entry: entry.clone(), what: What::Syntax, text, forms, run: Run::Compile });
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
    plan.cases.push(Case { rapidr: false, entry, what: What::Example, text: example.trim().to_string(), forms, run });
}

/// The language card's rules ([`crate::card`]): each `right` program compiled
/// as a RapidR program, and run when it has an `expect`.
fn rule_cases(plan: &mut Plan) {
    for r in RULES.iter().filter(|r| !r.right.trim().is_empty()) {
        let entry = format!("rule {}", r.name);
        let run = if r.expect.is_empty() {
            plan.skipped.push((entry.clone(), "compiled, not run: no `expect` says what it prints".into()));
            Run::Compile
        } else {
            Run::Execute(r.expect.to_string())
        };
        plan.cases.push(Case { rapidr: true, entry, what: What::Example, text: r.right.trim().to_string(), forms: vec![r.right.trim_end().to_string() + "\n"], run });
    }
}

fn example_cases(plan: &mut Plan) {
    for c in COMPONENTS.iter().chain(GLOBALS) {
        if !c.example.trim().is_empty() {
            let entry = c.name.to_string();
            let source = c.example.trim_end().to_string() + "\n";
            plan.skipped.push((entry.clone(), "a component's example opens windows: compiled, not run".into()));
            plan.cases.push(Case { rapidr: false, entry, what: What::Example, text: c.example.trim().to_string(), forms: vec![source], run: Run::Compile });
        }
        for m in c.methods.iter().filter(|m| !m.example.trim().is_empty()) {
            example_case(plan, format!("{}.{}", c.name, m.name), m.example, m.expect, m.effects, true);
        }
    }
    for b in BUILTINS.iter().filter(|b| !b.example.trim().is_empty()) {
        example_case(plan, format!("builtin {}", b.name), b.example, b.expect, b.effects, true);
    }
}

/// What a trap's example must do, for the messages.
fn named_problem(names: &[Slot], example: &str, name: &str, method: bool) -> String {
    let list = names.iter().map(|s| if s.optional { format!("[{}]", s.name) } else { s.name.clone() }).collect::<Vec<_>>().join(", ");
    // (what the example passes, when it passes variables)
    let passes = calls(example, name, method).into_iter().find(|c| !c.is_empty() && c.iter().all(|a| is_identifier(a))).map(|c| format!("it passes ({})", c.join(", ")));
    format!(
        "the documented parameters are ({list}), but its `example` {}: write the call with variables named like the parameters, in their order, set just above it, so what `expect` proves is about this signature",
        passes.unwrap_or_else(|| "never calls it with variables".into())
    )
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
            // (two neighbours of one kind: the same type, an untyped one counting as its own kind)
            if m.params.windows(2).any(|w| !w[0].variadic && !w[1].variadic && w[0].ty == w[1].ty) {
                trap(plan, format!("{}.{}", c.name, m.name), covered, !m.expect.is_empty());
            }
        }
    }
    for b in BUILTINS.iter().filter(|b| !b.missing && !b.name.starts_with("__") && !b.syntax.is_empty()) {
        if arg_kinds(b.syntax).windows(2).any(|w| w[0] == w[1]) {
            trap(plan, format!("builtin {}", b.name), COVERED_BUILTIN_GROUPS.contains(&b.group), !b.expect.is_empty());
        }
    }
}

/// An example with an `expect` calls its method (or builtin) with variables named like the
/// documented parameters: that ties the signature the reference shows to what the example
/// proves, so a signature written back to front fails here.
fn names_rule(plan: &mut Plan) {
    for c in COMPONENTS.iter().chain(GLOBALS) {
        for m in c.methods.iter().filter(|m| !m.expect.is_empty() && !m.params.is_empty()) {
            let slots: Vec<Slot> = m.params.iter().map(Slot::of).collect();
            if !calls_with_the_names(m.example, m.name, true, std::slice::from_ref(&slots)) {
                plan.problems.push((format!("{}.{}", c.name, m.name), named_problem(&slots, m.example, m.name, true)));
            }
        }
    }
    for b in BUILTINS.iter().filter(|b| !b.expect.is_empty()) {
        let forms = arg_names(b.syntax);
        if forms.iter().any(|f| !f.is_empty()) && !calls_with_the_names(b.example, b.name, false, &forms) {
            plan.problems.push((format!("builtin {}", b.name), named_problem(forms.first().map_or(&[][..], Vec::as_slice), b.example, b.name, false)));
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
    rule_cases(&mut plan);
    trap_rule(&mut plan);
    names_rule(&mut plan);
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
        // (each optional part alone, never all at once: an array has no single first value)
        let v = lines("DIM", "DIM name[(bounds)] [AS type] [= value] [, …]");
        assert_eq!(v, ["DIM n1", "DIM n1(3)", "DIM n1 AS INTEGER", "DIM n1 = 1", "DIM n1, n2"]);
        let v = lines("EXIT", "EXIT FOR|DO|WHILE|SUB|FUNCTION");
        assert_eq!(v, ["EXIT FOR", "EXIT DO", "EXIT WHILE", "EXIT SUB", "EXIT FUNCTION"]);
        let v = lines("INSTR", "INSTR([Start,] Search$,Find$)");
        assert_eq!(v, ["INSTR(\"x\", \"x\")", "INSTR(1, \"x\", \"x\")"]);
    }

    #[test]
    fn const_lists_repeat() {
        let v = lines("CONST", "CONST name = expression [, …]");
        assert!(v.contains(&"CONST n1 = 1".to_string()), "{v:?}");
        assert!(v.contains(&"CONST n1 = 1, n2 = 1".to_string()), "{v:?}");
    }

    #[test]
    fn blocks_get_bodies() {
        let v = lines("FOR", "FOR variable = start TO end [STEP step] … NEXT [variable]");
        assert!(v.iter().any(|l| l.starts_with("FOR v = 1 TO 1") && l.contains("\nPRINT 1\nNEXT")), "{v:?}");
        let v = lines("SELECT CASE", "SELECT CASE expression … CASE values … CASE ELSE … END SELECT");
        assert_eq!(v.len(), 1);
        assert_eq!(v[0], "SELECT CASE 1\nCASE 1\nPRINT 1\nCASE ELSE\nPRINT 1\nEND SELECT");
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

    fn names(list: &[(&str, bool)]) -> Vec<Slot> {
        list.iter().map(|(n, o)| Slot { name: n.to_string(), optional: *o, variadic: false }).collect()
    }

    #[test]
    fn an_example_must_call_with_the_documented_names() {
        let example = "row = 1\ncol = 0\nPRINT d.Cell(row, col); \"|\"; d.Cell(2, \"city\")";
        let documented = names(&[("row", false), ("col", false)]);
        let swapped = names(&[("col", false), ("row", false)]);
        assert!(calls_with_the_names(example, "Cell", true, &[documented]));
        // (the documentation said `col, row` once: the example, which prints what `expect` says, doesn't)
        assert!(!calls_with_the_names(example, "Cell", true, &[swapped]));
        // (literals prove nothing about the names)
        assert!(!calls_with_the_names("PRINT d.Cell(1, 0)", "Cell", true, &[names(&[("row", false), ("col", false)])]));
        // (a statement call; an optional parameter left out)
        assert!(calls_with_the_names("start = 1\nend = 3\nd.Iloc start, end : PRINT d.Nrows", "Iloc", true, &[names(&[("start", false), ("end", true)])]));
        assert!(calls_with_the_names("d.Select columns", "Select", true, &[names(&[("columns", false)])]));
        assert!(!calls_with_the_names("d.Select columns", "Select", true, &[names(&[("col1", false), ("col2", true)])]), "the documentation said col1, col2 ... once");
        // (a `...` parameter: any more variables)
        let mut row = names(&[("v1", false), ("v2", false)]);
        row[1].variadic = true;
        assert!(calls_with_the_names("d.AddRow v1, v2, v3", "AddRow", true, &[row]));
        // (a builtin: the optional `Start` may be left out anywhere)
        let instr = arg_names("INSTR([Start,] Search$,Find$)");
        assert!(calls_with_the_names("PRINT INSTR(Search$, Find$)", "INSTR", false, &instr));
        assert!(calls_with_the_names("PRINT INSTR(Start, Search$, Find$)", "INSTR", false, &instr));
        assert!(!calls_with_the_names("PRINT INSTR(Find$, Search$)", "INSTR", false, &instr));
        // (a name inside a text isn't a call)
        assert!(calls("PRINT \"d.Cell(a, b)\"", "Cell", true).is_empty());
    }

    #[test]
    fn calls_are_read_with_their_arguments() {
        assert_eq!(calls("x = a.Clip(low, high) + 1", "Clip", true), [vec!["low", "high"]]);
        assert_eq!(calls("a.Clip low, high: PRINT 1", "Clip", true), [vec!["low", "high"]]);
        assert_eq!(calls("PRINT MID$(f(1, 2), n)", "MID$", false), [vec!["f(1, 2)", "n"]]);
        assert!(calls("PRINT XMID$(a, b)", "MID$", false).is_empty());
        assert_eq!(arg_names("STRING$(Num, Char) or STRING$(Num, Char$)").len(), 2);
    }

    #[test]
    fn every_syntax_line_is_read() {
        let plan = plan();
        let unread: Vec<_> = plan.problems.iter().filter(|(_, w)| w.contains("can't be read")).collect();
        assert!(unread.is_empty(), "{unread:?}");
    }
}
