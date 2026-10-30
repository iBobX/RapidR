//! The code's helpers (docs/code-helpers-plan.md, E0): where a file's
//! values are, what each is and how it is written, read from the program's
//! semantic model and the registry's `kind` facet — never from the text's
//! look. A code editor shows them (a colour's swatch, a picture's
//! thumbnail, a font's sample, a parameter's name before an argument, a
//! line's help) and edits them with its value editors, writing the new
//! value back the way it was written.
//!
//! - A property set (`Button1.Color = RGB(255, 130, 20)`, `Color = …` in a
//!   CREATE body, `.Color = …` in a WITH): the property's kind.
//! - An argument of a method, builtin or SUB (`Image1.Load "logo.png"`,
//!   `PLAYWAV "click.wav", SND_ASYNC`): its parameter's kind, and its
//!   parameter's name for the chip before it.
//! - An event set (`OnClick = SaveClick`, and `OnClick =` with nothing yet):
//!   the program's SUBs, and a new handler's name and place.
//! - `RGB(…)` and a colour constant (`clRed`) anywhere: colours.
//! - Each line's first call or property: its one-line help (the note).
//!
//! What it can't evaluate (a variable, an expression) shows no value: the
//! helper says nothing rather than guess.

use std::collections::HashSet;
use std::path::Path;

use rapidr_ast::{BinaryOperator, Expression, LiteralValue, Statement, UnaryOperator};
use rapidr_diagnostics::TextSpan;
use rapidr_editor::service::{HelperKind, HelperSite};
use rapidr_lang::kinds::{choices, ValueKind};
use rapidr_lang::{Component, Property};

use crate::context::{self, Ty};
use crate::model::{name_key, SymbolKind};
use crate::text::LineIndex;
use crate::Snapshot;

/// The longest note (characters).
const NOTE_MAX: usize = 96;

pub(crate) fn helpers(s: &Snapshot, file: &Path, text: &str) -> Vec<HelperSite> {
    let mut w = Walk {
        s,
        file,
        text,
        index: LineIndex::new(text),
        out: Vec::new(),
        seen: HashSet::new(),
        noted: HashSet::new(),
        creates: Vec::new(),
        top: 0,
        subs: program_subs(s),
        routines: std::collections::HashMap::new(),
        fid: s.parsed.tools.file_id(s.parsed.file_key(file).unwrap_or(file)),
    };
    w.statements(&s.parsed.program.statements, None);
    let mut out = w.out;
    out.sort_by_key(|h| (h.start, h.kind != HelperKind::ArgName, h.end));
    out
}

struct Walk<'a> {
    s: &'a Snapshot,
    file: &'a Path,
    text: &'a str,
    index: LineIndex,
    out: Vec<HelperSite>,
    /// (start, end, kind) already given: one helper per value.
    seen: HashSet<(usize, usize, HelperKind)>,
    /// Lines with their note.
    noted: HashSet<usize>,
    /// The CREATE blocks the walk is in: the component, its name.
    creates: Vec<(&'static Component, String)>,
    /// The outermost statement's line start: a new handler goes before it.
    top: usize,
    /// The program's SUBs (a handler's choices).
    subs: Vec<String>,
    /// Each routine's parameters, found once (lower-case name).
    routines: std::collections::HashMap<String, Option<Params>>,
    /// The file's id in the program's origin map.
    fid: Option<usize>,
}

/// The program's SUBs, as declared (a handler is one).
fn program_subs(s: &Snapshot) -> Vec<String> {
    let mut v: Vec<String> = s.model.symbols.iter().filter(|sym| sym.kind == SymbolKind::Sub && sym.scope == 0 && sym.decl.is_some()).map(|sym| sym.name.clone()).collect();
    v.sort_by_key(|n| n.to_ascii_lowercase());
    v.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    v
}

/// What a parameter list says: names, and each one's kind.
#[derive(Clone)]
struct Params {
    names: Vec<String>,
    kinds: Vec<Option<&'static str>>,
    doc: &'static str,
}

impl Walk<'_> {
    /// A span of the AST in this file (bytes), when it's here.
    fn here(&self, span: TextSpan) -> Option<(usize, usize)> {
        // (the origin map straight, not `Snapshot::locate`: that one counts
        // a location's line and column too — the whole file's bytes up to
        // it, for each of the thousands of places a program has; the walk
        // took 90 ms on a 4,500-line project that way)
        let fid = self.fid?;
        let o = self.s.parsed.tools.preprocessed.origins.origin_span(span.start, span.end)?;
        (o.exact && o.file == fid).then_some((o.start, o.end.max(o.start).min(self.text.len())))
    }

    fn push(&mut self, h: HelperSite) {
        if self.seen.insert((h.start, h.end, h.kind)) {
            self.out.push(h);
        }
    }

    fn statements(&mut self, body: &[Statement], parent_create: Option<usize>) {
        for st in body {
            if self.creates.is_empty() && parent_create.is_none() {
                // (a top-level statement: where a new handler goes, before it)
                if let Some((a, _)) = self.here(rapidr_parser::statement_span(st)) {
                    self.top = self.index.line_start(self.index.line_col(a).0).unwrap_or(a);
                }
            }
            self.statement(st, parent_create);
        }
    }

    fn statement(&mut self, st: &Statement, parent_create: Option<usize>) {
        match st {
            Statement::Assignment(a) => {
                self.assignment(&a.target, &a.value, parent_create);
                self.expressions(&[&a.value]);
            }
            Statement::Call(c) => {
                let args: Vec<&Expression> = c.args.iter().collect();
                self.call(&c.callee, &args, c.span);
                self.expressions(&args);
            }
            Statement::Create(c) => {
                if let Some(comp) = rapidr_lang::resolve_component(&c.type_name) {
                    self.creates.push((comp, c.name.clone()));
                    let depth = self.creates.len();
                    self.statements(&c.body, Some(depth));
                    self.pending_handlers(c, comp);
                    self.creates.pop();
                } else {
                    self.statements(&c.body, None);
                }
            }
            Statement::Subroutine(x) => self.statements(&x.body, None),
            Statement::Function(x) => self.statements(&x.body, None),
            Statement::With(x) => self.statements(&x.body, None),
            Statement::For(x) => {
                self.expressions(&[&x.start, &x.end]);
                self.statements(&x.body, None);
            }
            Statement::While(x) => {
                self.expressions(&[&x.condition]);
                self.statements(&x.body, None);
            }
            Statement::DoLoop(x) => {
                let c: Vec<&Expression> = x.condition.iter().collect();
                self.expressions(&c);
                self.statements(&x.body, None);
            }
            Statement::If(i) => {
                self.expressions(&[&i.condition]);
                self.statements(&i.then_body, None);
                for b in &i.elseif_branches {
                    self.expressions(&[&b.condition]);
                    self.statements(&b.body, None);
                }
                self.statements(&i.else_body, None);
            }
            Statement::SelectCase(x) => {
                self.expressions(&[&x.expression]);
                for c in &x.cases {
                    self.statements(&c.body, None);
                }
                self.statements(&x.case_else, None);
            }
            Statement::Type(t) => {
                self.statements(&t.methods, None);
                self.statements(&t.constructor, None);
            }
            Statement::Print(p) => {
                let items: Vec<&Expression> = p.items.iter().collect();
                self.expressions(&items);
            }
            Statement::Const(c) => self.expressions(&[&c.value]),
            Statement::Return(r) => {
                let v: Vec<&Expression> = r.value.iter().collect();
                self.expressions(&v);
            }
            _ => {}
        }
    }

    /// `target = value`: a property's value, an event's SUB.
    fn assignment(&mut self, target: &Expression, value: &Expression, parent_create: Option<usize>) {
        let (comp, member, owner) = match target {
            // (a property set directly in a CREATE body)
            Expression::Identifier(id) if parent_create.is_some_and(|d| d == self.creates.len()) => {
                let Some((c, name)) = self.creates.last() else { return };
                (*c, id.name.as_str(), name.clone())
            }
            Expression::MemberAccess(m) => {
                let ty = match context::expr_type(self.s, &m.object, m.span.start) {
                    Some(ty) => Some(ty),
                    // (in a CREATE body a path starts at a property of the
                    // component made — `Font.Color = …` —, as a name alone does)
                    None if parent_create.is_some_and(|d| d == self.creates.len()) => {
                        let made = self.creates.last().map(|(c, _)| *c);
                        made.and_then(|c| context::chain_in(self.s, Ty::Component(c), &m.object))
                    }
                    None => None,
                };
                let comp = match ty {
                    Some(Ty::Component(c)) => c,
                    Some(Ty::User(t)) => match context::base_component(self.s, &t) {
                        Some(c) => c,
                        None => return,
                    },
                    None => return,
                };
                let owner = match m.object.as_ref() {
                    Expression::Identifier(id) if id.name != "_with_" => id.name.clone(),
                    _ => String::new(),
                };
                (comp, m.member.as_str(), owner)
            }
            _ => return,
        };
        let line_of = |w: &Self, span: TextSpan| w.here(span).map(|(a, _)| a);
        if let Some(p) = comp.property(member) {
            if let Some(at) = line_of(self, rapidr_parser::expression_span(target)) {
                self.note(at, crate::hover::tdoc(&format!("{}.{}", comp.name, p.name), p.doc));
            }
            if let Some(k) = p.kind.and_then(ValueKind::parse) {
                self.value(k, value, Some(p));
            }
        } else if let Some(e) = comp.event(member) {
            if let Some(at) = line_of(self, rapidr_parser::expression_span(target)) {
                self.note(at, crate::hover::tdoc(&format!("{}.{}", comp.name, e.name), e.doc));
            }
            self.handler(value, comp, e, &owner);
        }
    }

    /// A value of kind `k` (a property's, `p`; an argument's).
    fn value(&mut self, k: ValueKind, e: &Expression, p: Option<&Property>) {
        let Some((start, end)) = self.here(rapidr_parser::expression_span(e)) else { return };
        match k {
            ValueKind::Color => {
                if let Some((v, form)) = self.color(e) {
                    let mut h = HelperSite::new(HelperKind::Color, start, end);
                    h.value = v.to_string();
                    h.form = form.into();
                    self.push(h);
                }
            }
            ValueKind::Font | ValueKind::File(_) => {
                let Expression::Literal(l) = e else { return };
                let LiteralValue::String(t) = &l.value else { return };
                let mut h = HelperSite::new(if k == ValueKind::Font { HelperKind::Font } else { HelperKind::File }, start, end);
                h.value = t.clone();
                h.form = "string".into();
                if let ValueKind::File(f) = k {
                    h.label = match f {
                        rapidr_lang::kinds::FileKind::Image => "image",
                        rapidr_lang::kinds::FileKind::Sound => "sound",
                        rapidr_lang::kinds::FileKind::Any => "any",
                    }
                    .into();
                }
                self.push(h);
            }
            ValueKind::Choice("bool") => {
                let (v, form) = match e {
                    Expression::Identifier(id) if id.name.eq_ignore_ascii_case("true") => ("True", "word"),
                    Expression::Identifier(id) if id.name.eq_ignore_ascii_case("false") => ("False", "word"),
                    Expression::Literal(l) => match l.value {
                        LiteralValue::Integer(0) => ("False", "number"),
                        LiteralValue::Integer(_) => ("True", "number"),
                        _ => return,
                    },
                    Expression::Unary(u) if u.operator == UnaryOperator::Negate && matches!(u.operand.as_ref(), Expression::Literal(_)) => ("True", "number"),
                    _ => return,
                };
                let mut h = HelperSite::new(HelperKind::Bool, start, end);
                h.value = v.into();
                h.form = form.into();
                h.choices = vec!["True".into(), "False".into()];
                self.push(h);
            }
            ValueKind::Choice(_) | ValueKind::Cursor => {
                let set = if let ValueKind::Choice(s) = k { s } else { "values" };
                let (list, flags) = choices(set, p);
                if list.is_empty() {
                    return;
                }
                let Some((value, form)) = choice_value(e, &list) else { return };
                let mut h = HelperSite::new(HelperKind::Choice, start, end);
                h.value = value;
                h.form = form.into();
                h.flag = flags;
                h.label = set.into();
                h.choices = list.iter().map(|c| c.to_string()).collect();
                self.push(h);
            }
            ValueKind::Component(_) => {
                let Expression::Identifier(id) = e else { return };
                let types = k.component_types();
                let mut h = HelperSite::new(HelperKind::Component, start, end);
                h.value = id.name.clone();
                h.form = "name".into();
                h.choices = self.components_of(&types);
                self.push(h);
            }
            _ => {}
        }
    }

    /// A colour's value and how it's written, when it can be told.
    fn color(&self, e: &Expression) -> Option<(i64, &'static str)> {
        match e {
            Expression::FunctionCall(f) => {
                let Expression::Identifier(id) = f.callee.as_ref() else { return None };
                if !id.name.eq_ignore_ascii_case("RGB") || f.args.len() != 3 {
                    return None;
                }
                let mut c = [0i64; 3];
                for (k, a) in f.args.iter().enumerate() {
                    c[k] = whole(a)?.clamp(0, 255);
                }
                Some((c[0] | (c[1] << 8) | (c[2] << 16), "rgb"))
            }
            Expression::Literal(l) => {
                let LiteralValue::Integer(v) = l.value else { return None };
                let (a, b) = self.here(l.span)?;
                let hex = self.text.get(a..b).is_some_and(|t| t.trim_start().get(..2).is_some_and(|p| p.eq_ignore_ascii_case("&H")));
                Some((v, if hex { "hex" } else { "number" }))
            }
            Expression::Unary(u) if u.operator == UnaryOperator::Negate => {
                let v = whole(e)?;
                let _ = u;
                Some((v, "number"))
            }
            Expression::Identifier(id) => {
                let (v, g) = rapidr_lang::constant(&id.name)?;
                (g.kind == Some("color")).then_some((v, "const"))
            }
            _ => None,
        }
    }

    /// The program's components of one of `types` (any: every one).
    fn components_of(&self, types: &[&str]) -> Vec<String> {
        let wanted: Vec<&'static str> = types.iter().filter_map(|t| rapidr_lang::resolve_component(t)).map(|c| c.name).collect();
        let mut v: Vec<String> = self
            .s
            .model
            .symbols
            .iter()
            .filter(|sym| sym.scope == 0 && matches!(sym.kind, SymbolKind::Component | SymbolKind::Global))
            .filter(|sym| {
                let Some(c) = sym.ty.as_deref().and_then(rapidr_lang::resolve_component) else { return false };
                wanted.is_empty() || wanted.contains(&c.name)
            })
            .map(|sym| sym.name.clone())
            .collect();
        v.sort_by_key(|n| n.to_ascii_lowercase());
        v.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
        v
    }

    /// An event's SUB: `OnClick = SaveClick`.
    fn handler(&mut self, value: &Expression, comp: &'static Component, e: &'static rapidr_lang::Event, owner: &str) {
        let Some((start, end)) = self.here(rapidr_parser::expression_span(value)) else { return };
        let Expression::Identifier(id) = value else { return };
        let mut h = HelperSite::new(HelperKind::Handler, start, end);
        h.value = id.name.clone();
        h.form = "name".into();
        self.fill_handler(&mut h, comp, e, owner);
        self.push(h);
    }

    fn fill_handler(&self, h: &mut HelperSite, comp: &'static Component, e: &'static rapidr_lang::Event, owner: &str) {
        h.choices = self.subs.clone();
        h.label = e.params.iter().map(|p| p.text()).collect::<Vec<_>>().join(", ");
        let stem = e.name.strip_prefix("On").or_else(|| e.name.strip_prefix("on")).unwrap_or(e.name);
        let owner = if owner.is_empty() { comp.display } else { owner };
        h.hint = format!("{owner}{stem}");
        h.at = self.top;
    }

    /// `OnClick =` with nothing after it yet, directly in a CREATE body: the
    /// event's SUB is wanted (the parser has no statement for it).
    fn pending_handlers(&mut self, c: &rapidr_ast::CreateStatement, comp: &'static Component) {
        let Some((a, b)) = self.here(c.span) else { return };
        let (first, last) = (self.index.line_col(a).0 + 1, self.index.line_col(b).0);
        for line in first..last {
            let Some(ls) = self.index.line_start(line) else { continue };
            let lt = self.index.line_text(self.text, line);
            let body = lt.split('\'').next().unwrap_or("").trim_end();
            let Some(name) = body.trim().strip_suffix('=').map(str::trim_end) else { continue };
            if name.is_empty() || !name.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_') {
                continue;
            }
            let Some(e) = comp.event(name) else { continue };
            let at = ls + body.len();
            let mut h = HelperSite::new(HelperKind::Handler, at, at);
            let owner = self.creates.last().map(|(_, n)| n.clone()).unwrap_or_default();
            self.fill_handler(&mut h, comp, e, &owner);
            h.form = "name".into();
            self.note(ls + (lt.len() - lt.trim_start().len()), crate::hover::tdoc(&format!("{}.{}", comp.name, e.name), e.doc));
            self.push(h);
        }
    }

    /// A call: `callee args` (a statement) or `callee(args)`.
    fn call(&mut self, callee: &Expression, args: &[&Expression], span: TextSpan) {
        let params = match callee {
            Expression::Identifier(id) => self.routine_params(&id.name),
            Expression::MemberAccess(m) => self.method_params(&m.object, &m.member, m.span.start),
            _ => None,
        };
        let Some(params) = params else { return };
        if let Some((a, _)) = self.here(span) {
            self.note(a, params.doc);
        }
        self.arguments(&params, args, false);
    }

    /// The parameters of a builtin or of one of the program's routines
    /// (found once a name).
    fn routine_params(&mut self, name: &str) -> Option<Params> {
        let key = name.to_ascii_lowercase();
        if let Some(p) = self.routines.get(&key) {
            return p.clone();
        }
        let p = self.find_routine_params(name);
        self.routines.insert(key, p.clone());
        p
    }

    fn find_routine_params(&self, name: &str) -> Option<Params> {
        let scope = self.s.model.scope_at(0);
        if let Some(sym) = self.s.model.lookup(name, scope).map(|id| &self.s.model.symbols[id]) {
            if matches!(sym.kind, SymbolKind::Sub | SymbolKind::Function | SymbolKind::External) {
                let st = context::routine_statement(&self.s.parsed.program, &sym.name)?;
                let names = match st {
                    Statement::Subroutine(s) => s.params.iter().map(|p| p.name.clone()).collect(),
                    Statement::Function(f) => f.params.iter().map(|p| p.name.clone()).collect(),
                    Statement::Declare(d) => d.params.iter().map(|p| p.name.clone()).collect(),
                    _ => Vec::new(),
                };
                let kinds = vec![None; names.len()];
                return Some(Params { names, kinds, doc: "" });
            }
            return None;
        }
        let b = rapidr_lang::builtin(name)?;
        let names = b.param_names();
        let kinds = names.iter().map(|n| b.arg_kind(n)).collect();
        // (a built-in's one-letter names stand for a type — `S$` a text,
        // `n` a number — and say nothing a chip could add)
        let names = names.into_iter().map(|n| if n.trim_end_matches(['$', '%', '&', '#', '!']).chars().count() > 1 { n } else { String::new() }).collect();
        Some(Params { names, kinds, doc: crate::hover::tdoc(b.name, b.doc) })
    }

    /// The parameters of a component's method (`object.member`).
    fn method_params(&self, object: &Expression, member: &str, pre: usize) -> Option<Params> {
        let comp = match context::expr_type(self.s, object, pre)? {
            Ty::Component(c) => c,
            Ty::User(t) => context::base_component(self.s, &t)?,
        };
        let m = comp.method(member)?;
        let names: Vec<String> = m.params.iter().map(|p| p.name.trim_end_matches("()").trim_end_matches(['$', '%', '&', '#', '!']).to_string()).collect();
        let kinds = names.iter().map(|n| m.arg_kind(n)).collect();
        Some(Params { names, kinds, doc: crate::hover::tdoc(&format!("{}.{}", comp.name, m.name), m.doc) })
    }

    /// The arguments' chips and values (`quiet`: their names only while
    /// Ctrl is held — an `RGB(…)`'s, whose swatch says it all).
    fn arguments(&mut self, params: &Params, args: &[&Expression], quiet: bool) {
        let several = args.len() > 1 && !quiet;
        for (i, arg) in args.iter().enumerate() {
            let Some(name) = params.names.get(i) else { break };
            if let Some(k) = params.kinds.get(i).copied().flatten().and_then(ValueKind::parse) {
                self.value(k, arg, None);
            }
            // (a variable named as the parameter needs no chip: `x` for x;
            // nor does a name that says nothing — `S`, `arg1`)
            if !tells(name) || matches!(arg, Expression::Identifier(id) if name_key(&id.name) == name_key(name)) {
                continue;
            }
            let Some((start, end)) = self.here(rapidr_parser::expression_span(arg)) else { continue };
            let mut h = HelperSite::new(HelperKind::ArgName, start, end);
            h.label = name.clone();
            h.flag = several && literal(arg);
            self.push(h);
        }
    }

    /// The expressions' calls (`RGB(…)`, `Obj.Method(…)`, a FUNCTION's) and
    /// colour constants, at any depth.
    fn expressions(&mut self, list: &[&Expression]) {
        let mut found: Vec<Expression> = Vec::new();
        for e in list {
            rapidr_ast::walk_expression(e, &mut |x| {
                if matches!(x, Expression::FunctionCall(_) | Expression::MethodCall(_) | Expression::Identifier(_)) {
                    found.push(x.clone());
                }
            });
        }
        for x in &found {
            match x {
                Expression::FunctionCall(f) => {
                    if let Expression::Identifier(id) = f.callee.as_ref() {
                        if rapidr_lang::builtin(&id.name).and_then(|b| b.kind) == Some("color") {
                            self.value(ValueKind::Color, x, None);
                        }
                    }
                    let args: Vec<&Expression> = f.args.iter().collect();
                    // (an array's element looks the same: only a routine's call)
                    if let Expression::Identifier(id) = f.callee.as_ref() {
                        if let Some(p) = self.routine_params(&id.name) {
                            let quiet = rapidr_lang::builtin(&id.name).is_some_and(|b| b.kind.is_some());
                            self.arguments(&p, &args, quiet);
                        }
                    }
                }
                Expression::MethodCall(m) => {
                    let args: Vec<&Expression> = m.args.iter().collect();
                    if let Some(p) = self.method_params(&m.object, &m.method, m.span.start) {
                        self.arguments(&p, &args, false);
                    }
                }
                Expression::Identifier(id) => {
                    if rapidr_lang::kinds::constant_kind(&id.name) == Some("color") && self.s.model.lookup(&id.name, self.s.model.scope_at(id.span.start)).is_none() {
                        self.value(ValueKind::Color, x, None);
                    }
                }
                _ => {}
            }
        }
    }

    /// The note of the line holding byte `at`: `doc`'s first sentence (once
    /// a line: its first call or property).
    fn note(&mut self, at: usize, doc: &str) {
        let line = self.index.line_col(at).0;
        if doc.trim().is_empty() || !self.noted.insert(line) {
            return;
        }
        let Some(ls) = self.index.line_start(line) else { return };
        let le = ls + self.index.line_text(self.text, line).len();
        let mut h = HelperSite::new(HelperKind::Note, ls, le);
        h.label = first_sentence(doc);
        self.push(h);
    }
}

/// A whole number an expression is: a literal, negated or not.
fn whole(e: &Expression) -> Option<i64> {
    match e {
        Expression::Literal(l) => match l.value {
            LiteralValue::Integer(v) => Some(v),
            LiteralValue::Float(f) if f.fract() == 0.0 => Some(f as i64),
            _ => None,
        },
        Expression::Unary(u) if u.operator == UnaryOperator::Negate => whole(&u.operand).map(|v| -v),
        Expression::Unary(u) if u.operator == UnaryOperator::Positive => whole(&u.operand),
        _ => None,
    }
}

/// Whether a parameter's name tells the reader something: not a
/// placeholder (`arg1`, `arg2`; a built-in's one-letter `S$` or `n` is
/// blanked where its names are read).
fn tells(name: &str) -> bool {
    let n = name.trim_end_matches(['$', '%', '&', '#', '!']);
    let rest = n.strip_prefix("arg").or_else(|| n.strip_prefix("Arg")).or_else(|| n.strip_prefix("ARG"));
    !n.is_empty() && !rest.is_some_and(|r| r.chars().all(|c| c.is_ascii_digit()))
}

/// A literal argument: a number, a text, True / False.
fn literal(e: &Expression) -> bool {
    match e {
        Expression::Literal(_) => true,
        Expression::Unary(u) if matches!(u.operator, UnaryOperator::Negate | UnaryOperator::Positive) => literal(&u.operand),
        Expression::Identifier(id) => id.name.eq_ignore_ascii_case("true") || id.name.eq_ignore_ascii_case("false"),
        _ => false,
    }
}

/// A choice's value and its form: a constant (`alClient`), constants added
/// or OR'd (`mbYes + mbNo`), a number.
fn choice_value(e: &Expression, list: &[&str]) -> Option<(String, &'static str)> {
    let spelled = |n: &str| list.iter().find(|c| c.eq_ignore_ascii_case(n)).map(|c| c.to_string());
    match e {
        Expression::Identifier(id) => Some((spelled(&id.name)?, "const")),
        Expression::Literal(l) => match l.value {
            LiteralValue::Integer(v) => Some((v.to_string(), "number")),
            _ => None,
        },
        Expression::Binary(b) if matches!(b.operator, BinaryOperator::Add | BinaryOperator::Or) => {
            let mut names = Vec::new();
            fn flags(e: &Expression, op: BinaryOperator, out: &mut Vec<String>) -> bool {
                match e {
                    Expression::Identifier(id) => {
                        out.push(id.name.clone());
                        true
                    }
                    Expression::Binary(b) if b.operator == op => flags(&b.left, op, out) && flags(&b.right, op, out),
                    _ => false,
                }
            }
            if !flags(e, b.operator, &mut names) {
                return None;
            }
            let spelled: Option<Vec<String>> = names.iter().map(|n| spelled(n)).collect();
            Some((spelled?.join("+"), if b.operator == BinaryOperator::Add { "flags+" } else { "flags-or" }))
        }
        _ => None,
    }
}

/// A registry doc's first sentence, plain (no Markdown), at most
/// [`NOTE_MAX`] characters.
fn first_sentence(doc: &str) -> String {
    let plain: String = doc.replace('`', "").replace("**", "");
    let mut end = plain.len();
    for (i, c) in plain.char_indices() {
        if (c == '.' || c == ':' || c == ';') && plain[i + 1..].starts_with(' ') {
            end = i + usize::from(c == '.');
            break;
        }
    }
    let s = plain[..end].trim().to_string();
    if s.chars().count() > NOTE_MAX {
        let cut: String = s.chars().take(NOTE_MAX - 1).collect();
        format!("{}…", cut.trim_end())
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Analysis, Options};
    use std::path::PathBuf;

    fn sites(code: &str) -> (String, Vec<HelperSite>) {
        let mut a = Analysis::new(Options::default());
        let f = PathBuf::from("/tmp/helpers-test/main.rr");
        a.update(&f, code);
        (code.to_string(), a.helpers(&f))
    }

    fn of(all: &[HelperSite], kind: HelperKind) -> Vec<&HelperSite> {
        all.iter().filter(|h| h.kind == kind).collect()
    }

    #[test]
    fn a_property_colour_in_its_three_forms() {
        let (t, all) = sites("CREATE Form1 AS RForm\n  CREATE Button1 AS RButton\n    Color = RGB(255, 130, 20)\n  END CREATE\nEND CREATE\nButton1.Color = &H0000FF\nButton1.FontColor = clRed\n");
        let c = of(&all, HelperKind::Color);
        assert_eq!(c.len(), 3, "{all:#?}");
        assert_eq!((&t[c[0].start..c[0].end], c[0].value.as_str(), c[0].form.as_str()), ("RGB(255, 130, 20)", "1344255", "rgb"));
        assert_eq!((&t[c[1].start..c[1].end], c[1].value.as_str(), c[1].form.as_str()), ("&H0000FF", "255", "hex"));
        assert_eq!((&t[c[2].start..c[2].end], c[2].value.as_str(), c[2].form.as_str()), ("clRed", "255", "const"));
    }

    /// A colour of a sub-object — `Font.Color`, a path of dots — has its swatch
    /// and its editor like `Color` has: set in a CREATE body (`Font.Color = …`),
    /// from outside (`Hello.Font.Color = …`), in a WITH, on a font of its own.
    #[test]
    fn a_colour_of_a_sub_object_has_its_swatch() {
        let code = "DIM PickedFont AS RFont\nCREATE Form1 AS RForm\n  Color = &HE0FFFF\n  Font.Color = &H800000\n  CREATE Hello AS RButton\n    Font.Color = RGB(1, 2, 3)\n  END CREATE\nEND CREATE\nHello.Font.Color = &H0000FF\nForm1.Font.Color = clRed\nPickedFont.Color = &H00FF00\nWITH Hello\n  .Font.Color = clBlue\nEND WITH\n";
        let (t, all) = sites(code);
        let c = of(&all, HelperKind::Color);
        let shown: Vec<(&str, &str, &str)> = c.iter().map(|h| (&t[h.start..h.end], h.value.as_str(), h.form.as_str())).collect();
        assert_eq!(
            shown,
            [
                ("&HE0FFFF", "14745599", "hex"),
                ("&H800000", "8388608", "hex"),
                ("RGB(1, 2, 3)", "197121", "rgb"),
                ("&H0000FF", "255", "hex"),
                ("clRed", "255", "const"),
                ("&H00FF00", "65280", "hex"),
                ("clBlue", "16711680", "const"),
            ],
            "{all:#?}"
        );
    }

    #[test]
    fn files_fonts_choices_and_links() {
        let code = "SUB SaveClick\nEND SUB\nCREATE Form1 AS RForm\n  CREATE Image1 AS RImage\n    Align = alClient\n  END CREATE\n  CREATE Save AS RButton\n    OnClick = SaveClick\n    Enabled = False\n  END CREATE\n  CREATE Fnt AS RFont\n  END CREATE\nEND CREATE\nImage1.Load \"logo.png\"\nFnt.Name = \"Arial\"\nPLAYWAV \"click.wav\", SND_ASYNC\n";
        let (t, all) = sites(code);
        let f = of(&all, HelperKind::File);
        assert_eq!(f.len(), 2, "{all:#?}");
        assert_eq!((f[0].value.as_str(), f[0].label.as_str(), &t[f[0].start..f[0].end]), ("logo.png", "image", "\"logo.png\""));
        assert_eq!((f[1].value.as_str(), f[1].label.as_str()), ("click.wav", "sound"));
        let font = of(&all, HelperKind::Font);
        assert_eq!(font[0].value, "Arial");
        let ch = of(&all, HelperKind::Choice);
        assert_eq!((ch[0].value.as_str(), ch[0].form.as_str()), ("alClient", "const"));
        assert!(ch[0].choices.iter().any(|c| c == "alTop"));
        assert_eq!((ch[1].value.as_str(), ch[1].label.as_str(), ch[1].flag), ("SND_ASYNC", "playwav-options", true));
        let b = of(&all, HelperKind::Bool);
        assert_eq!(b[0].value, "False");
        let h = of(&all, HelperKind::Handler);
        assert_eq!((h[0].value.as_str(), h[0].hint.as_str()), ("SaveClick", "SaveClick"));
        assert_eq!(h[0].choices, ["SaveClick"]);
        // (a new handler goes before the outermost statement: the form's CREATE)
        assert_eq!(&t[h[0].at..h[0].at + 6], "CREATE");
        let notes = of(&all, HelperKind::Note);
        assert!(notes.iter().any(|n| n.label.starts_with("Loads a picture file")), "{notes:#?}");
    }

    #[test]
    fn an_event_with_nothing_yet_wants_a_handler() {
        let code = "SUB Other\nEND SUB\nCREATE Form1 AS RForm\n  CREATE Ok AS RButton\n    OnClick = \n  END CREATE\nEND CREATE\n";
        let (t, all) = sites(code);
        let h = of(&all, HelperKind::Handler);
        assert_eq!(h.len(), 1, "{all:#?}");
        assert_eq!((h[0].value.as_str(), h[0].hint.as_str()), ("", "OkClick"));
        assert_eq!(&t[..h[0].start].lines().last().unwrap().trim(), &"OnClick =");
        assert_eq!(h[0].choices, ["Other"]);
    }

    #[test]
    fn argument_names_before_literals() {
        let code = "SUB DrawDot(x AS INTEGER, y AS INTEGER, r AS INTEGER)\nEND SUB\nx = 5\nDrawDot x, 40, 12\nCREATE Form1 AS RForm\n  CREATE cv AS RCanvas\n  END CREATE\nEND CREATE\ncv.FillRect 0, 0, 30, 30, RGB(1, 2, 3)\nPRINT MID$(\"abc\", 2, 1)\n";
        let (t, all) = sites(code);
        let chips: Vec<(&str, &str, bool)> = of(&all, HelperKind::ArgName).iter().map(|h| (h.label.as_str(), &t[h.start..h.end], h.flag)).collect();
        assert!(!chips.iter().any(|c| c.0 == "x" && c.1 == "x"), "x for x needs no chip: {chips:?}");
        assert!(chips.contains(&("y", "40", true)) && chips.contains(&("r", "12", true)), "{chips:?}");
        assert!(chips.contains(&("x1", "0", true)) && chips.contains(&("c", "RGB(1, 2, 3)", false)), "{chips:?}");
        // (an RGB(…)'s: only while Ctrl is held — its swatch says it)
        assert!(chips.contains(&("Red", "1", false)), "{chips:?}");
        assert!(chips.contains(&("Position", "2", true)), "{chips:?}");
        // (the canvas colour argument is a colour too)
        assert!(of(&all, HelperKind::Color).iter().any(|h| h.value == (1 | 2 << 8 | 3 << 16).to_string()));

        // (names that say nothing get no chip: FORMAT$'s `S$`, `arg1`)
        let (_, all) = sites("PRINT FORMAT$(\"%d-%d\", 1, 2)\n");
        assert!(of(&all, HelperKind::ArgName).is_empty(), "{all:#?}");
    }

    #[test]
    fn what_it_cannot_evaluate_shows_nothing() {
        let (_, all) = sites("c = 5\nCREATE Form1 AS RForm\n  Color = c\n  CREATE i AS RImage\n  END CREATE\nEND CREATE\ni.Load \"a\" + \".png\"\nForm1.Color = RGB(c, 0, 0)\n");
        assert!(of(&all, HelperKind::Color).is_empty() && of(&all, HelperKind::File).is_empty(), "{all:#?}");
    }

    /// The helpers' cost when typing pauses, on a real project (the folder
    /// RAPIDR_BENCH_DIR, its main.rr: Preview 4's 4,500 lines), after the
    /// diagnostics made the analysis (as the editor asks):
    ///
    ///   RAPIDR_BENCH_DIR=… cargo test --release -p rapidr-langsvc helpers_cost -- --ignored --nocapture
    #[test]
    #[ignore]
    fn helpers_cost() {
        let Ok(dir) = std::env::var("RAPIDR_BENCH_DIR") else { return };
        let dir = PathBuf::from(dir);
        let mut a = Analysis::new(Options::default());
        let mut files: Vec<PathBuf> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "rr")).collect();
        files.sort();
        for f in &files {
            a.update(f.clone(), std::fs::read_to_string(f).unwrap());
        }
        for f in &files {
            let _ = a.diagnostics(f);
            let t = std::time::Instant::now();
            let sites = a.helpers(f);
            let n = sites.len();
            let lines = std::fs::read_to_string(f).unwrap().lines().count();
            // (RAPIDR_BENCH_SITES: each file's sites written there, for the
            // editor's typing benchmark — the real ones, not made up)
            if let Ok(out) = std::env::var("RAPIDR_BENCH_SITES") {
                let rows: Vec<String> = sites.iter().map(|h| format!("{:?}\t{}\t{}\t{}\t{}\t{}\t{}", h.kind, h.start, h.end, h.value.replace(['\t', '\n'], " "), h.form, h.label.replace(['\t', '\n'], " "), u8::from(h.flag))).collect();
                std::fs::create_dir_all(&out).unwrap();
                std::fs::write(PathBuf::from(&out).join(format!("{}.sites", f.file_name().unwrap().to_string_lossy())), rows.join("\n")).unwrap();
            }
            println!("[helpers] {} ({lines} lines): {n} sites in {:.2} ms", f.file_name().unwrap().to_string_lossy(), t.elapsed().as_secs_f64() * 1000.0);
        }
    }

    #[test]
    fn notes_are_first_sentences() {
        assert_eq!(first_sentence("Loads a picture file (BMP, PNG). An SVG follows."), "Loads a picture file (BMP, PNG).");
        assert_eq!(first_sentence("`x`: the thing"), "x");
        assert!(first_sentence(&"word ".repeat(40)).ends_with('…'));
    }
}
