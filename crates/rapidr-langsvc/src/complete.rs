//! Completion: the program's own names in scope, components and their
//! members by the variable's type (also after a WITH's `.` and inside a
//! CREATE), builtins, statements, directives and types.

use std::collections::HashSet;
use std::path::Path;

use rapidr_lang::{Component, Kind, Origin};

use crate::compat;
use crate::context::{self, line_context, pretty_component, Place, Ty};
use crate::model::{name_key, ScopeKind, SymbolKind};
use crate::{Completion, CompletionKind, Completions, Snapshot};

pub(crate) fn completions(s: &Snapshot, file: &Path, text: &str, offset: usize) -> Completions {
    let lc = line_context(text, offset);
    let pre = s.pre_offset(file, lc.word_start).unwrap_or(0);
    let mut out = Out::default();
    match &lc.place {
        Place::Nothing => {}
        Place::Directive => {
            for d in rapidr_lang::DIRECTIVES {
                out.push(Completion {
                    label: d.name.to_string(),
                    kind: CompletionKind::Directive,
                    detail: Some(d.syntax.to_string()),
                    doc: Some(d.doc.to_string()),
                    insert: None,
                    snippet: false,
                    sort: "0".into(),
                });
            }
        }
        Place::AfterAs => types(s, &mut out),
        Place::Label => {
            let scope = s.model.scope_at(pre);
            for sym in &s.model.symbols {
                if sym.kind == SymbolKind::Label && (sym.scope == scope || sym.scope == 0) {
                    out.push(simple(&sym.name, CompletionKind::Label, Some("label".into()), "0"));
                }
            }
        }
        Place::Member(chain) => {
            if let Some(ty) = context::resolve_chain(s, pre, chain) {
                members(s, &ty, &mut out, false);
            }
        }
        Place::Code { statement_start } => {
            if *statement_start {
                if let Some(c) = context::create_at(s, pre) {
                    // In a CREATE body: the component's properties and events.
                    component_members(c, &mut out, true);
                    out.push(Completion {
                        label: "CREATE".into(),
                        kind: CompletionKind::Keyword,
                        detail: Some("CREATE name AS type … END CREATE".into()),
                        doc: Some("Creates a component inside this one.".into()),
                        insert: Some("CREATE ${1:Name} AS ${2:QButton}\n\t$0\nEND CREATE".into()),
                        snippet: true,
                        sort: "1".into(),
                    });
                    return out.finish(lc.word_start, lc.word_end);
                }
            }
            names(s, pre, &mut out);
            // (what RapidR doesn't have yet isn't offered: compat's warning says so where it's written)
            for b in rapidr_lang::BUILTINS.iter().filter(|b| !b.missing) {
                out.push(Completion {
                    label: b.name.to_string(),
                    kind: CompletionKind::Builtin,
                    detail: Some(b.syntax.to_string()),
                    doc: Some(with_notes(b.doc, &compat::notes(b.origin, Origin::RapidQ, false, b.runtimes, None))),
                    insert: None,
                    snippet: false,
                    sort: "3".into(),
                });
            }
            for g in rapidr_lang::GLOBALS {
                out.push(Completion {
                    label: g.name.to_string(),
                    kind: CompletionKind::Component,
                    detail: Some("global object".into()),
                    doc: Some(with_notes(g.doc, &compat::notes(g.origin, Origin::RapidQ, false, g.runtimes, g.from))),
                    insert: None,
                    snippet: false,
                    sort: "3".into(),
                });
            }
            for st in rapidr_lang::STATEMENTS {
                out.push(Completion {
                    label: st.name.to_string(),
                    kind: CompletionKind::Keyword,
                    detail: Some(st.syntax.to_string()),
                    doc: Some(with_notes(st.doc, &compat::notes(st.origin, Origin::RapidQ, false, rapidr_lang::Runtimes::All, None))),
                    insert: None,
                    snippet: false,
                    sort: "4".into(),
                });
            }
            for k in rapidr_lang::KEYWORDS {
                out.push(Completion {
                    label: k.name.to_string(),
                    kind: CompletionKind::Keyword,
                    detail: Some(k.kind.to_string()),
                    doc: Some(with_notes(k.doc, &compat::notes(k.origin, Origin::RapidQ, false, rapidr_lang::Runtimes::All, None))),
                    insert: None,
                    snippet: false,
                    sort: "4".into(),
                });
            }
            // (RapidR's own constants, always there; RapidQ's come with
            // the program's $INCLUDE "RAPIDQ.INC": its own names)
            for g in rapidr_lang::CONSTANT_GROUPS.iter().filter(|g| g.origin == Origin::RapidR) {
                for (name, value) in g.constants {
                    let label = pretty_constant(name);
                    out.push(Completion {
                        label,
                        kind: CompletionKind::Constant,
                        detail: Some(format!("RapidR constant = {value}")),
                        doc: Some(g.doc.to_string()),
                        insert: None,
                        snippet: false,
                        sort: "3".into(),
                    });
                }
            }
        }
    }
    out.finish(lc.word_start, lc.word_end)
}

/// A doc with notes after it (either may be empty).
pub(crate) fn with_notes(doc: &str, notes: &str) -> String {
    match (doc.is_empty(), notes.is_empty()) {
        (_, true) => doc.to_string(),
        (true, false) => notes.to_string(),
        (false, false) => format!("{doc}\n\n{notes}"),
    }
}

/// Types after `AS`: RapidQ's components under RapidQ's names, RapidR's
/// own under RapidR's (docs/q-and-r-components.md), the program's TYPEs,
/// the built-in types.
fn types(s: &Snapshot, out: &mut Out) {
    for t in rapidr_lang::TYPE_NAMES {
        out.push(Completion {
            label: t.name.to_string(),
            kind: CompletionKind::Type,
            detail: Some(origin_label(t.origin).to_string()),
            doc: Some(t.doc.to_string()),
            insert: None,
            snippet: false,
            sort: "1".into(),
        });
    }
    for sym in &s.model.symbols {
        if sym.kind == SymbolKind::Type {
            out.push(simple(&sym.name, CompletionKind::Type, Some("TYPE".into()), "0"));
        }
    }
    // (RapidQ's components RapidR doesn't have yet aren't offered)
    for c in rapidr_lang::COMPONENTS.iter().filter(|c| c.kind != Kind::Planned) {
        let label = pretty_component(c.written_name());
        let detail = match (c.rapidq, c.from) {
            (Some(_), Some(inc)) => format!("RapidQ component from {inc} (RapidR: {})", pretty_component(c.name)),
            (Some(_), None) => format!("RapidQ component (RapidR: {})", pretty_component(c.name)),
            (None, _) => "RapidR component".to_string(),
        };
        out.push(Completion {
            label,
            kind: CompletionKind::Component,
            detail: Some(detail),
            doc: component_doc(c),
            insert: None,
            snippet: false,
            sort: if c.rapidq.is_some() { "2".into() } else { "3".into() },
        });
    }
}

/// A component's doc: what it is, its two names, where it comes from and
/// where it works, what it has.
pub(crate) fn component_doc(c: &Component) -> Option<String> {
    let mut doc = String::new();
    if !c.doc.is_empty() {
        doc.push_str(c.doc);
        doc.push_str("\n\n");
    }
    match c.rapidq {
        Some(q) => doc.push_str(&format!("RapidQ's **{}**, RapidR's **{}**: one component under two names.", pretty_component(q), pretty_component(c.name))),
        None => doc.push_str(&format!("**{}** is RapidR's own (RapidQ doesn't have it).", pretty_component(c.name))),
    }
    let notes = compat::notes(Origin::RapidQ, Origin::RapidQ, c.kind == Kind::Planned, c.runtimes, c.from);
    if !notes.is_empty() {
        doc.push_str("\n\n");
        doc.push_str(&notes);
    }
    if c.kind == Kind::Library {
        doc.push_str("\n\n*A RapidQ library written in BASIC, which RapidR supplies.*");
    }
    doc.push_str(&format!("\n\n{} properties, {} methods, {} events", c.properties.len(), c.methods.len(), c.events.len()));
    if let Some(e) = c.default_event {
        doc.push_str(&format!("; its default event is {e}"));
    }
    doc.push('.');
    Some(doc)
}

fn origin_label(o: Origin) -> &'static str {
    match o {
        Origin::RapidQ => "RapidQ",
        Origin::RapidR => "RapidR",
    }
}

/// The members of a type (`events_as_assignments`: in a CREATE body, events
/// are set as `OnClick = Handler`).
pub(crate) fn members(s: &Snapshot, ty: &Ty, out: &mut Out, events_as_assignments: bool) {
    match ty {
        Ty::Component(c) => component_members(c, out, events_as_assignments),
        Ty::User(t) => {
            for scope in context::type_chain(s, t) {
                for sym in s.model.symbols.iter().filter(|sym| sym.scope == scope) {
                    let (kind, detail) = match sym.kind {
                        SymbolKind::Field => (CompletionKind::Field, sym.ty.clone().map(|t| format!("AS {t}"))),
                        SymbolKind::Sub => (CompletionKind::Method, Some(format!("SUB of {t}"))),
                        SymbolKind::Function => (CompletionKind::Method, Some(format!("FUNCTION of {t}"))),
                        _ => continue,
                    };
                    out.push(simple(&sym.name, kind, detail, "0"));
                }
            }
            if let Some(c) = context::base_component(s, t) {
                component_members(c, out, events_as_assignments);
            }
        }
    }
}

fn component_members(c: &Component, out: &mut Out, in_create: bool) {
    let owner = pretty_component(c.written_name());
    let doc = |text: &str, origin, runtimes, from| Some(with_notes(text, &compat::notes(origin, c.origin, false, runtimes, from)));
    // (what RapidR doesn't answer yet isn't offered; in a CREATE body,
    // what can be set)
    for p in c.properties.iter().filter(|p| !p.missing && !(in_create && p.access == rapidr_lang::Access::Read)) {
        let ty = match p.access {
            rapidr_lang::Access::Read => format!("{}, read only", p.ty.as_str()),
            _ => p.ty.as_str().to_string(),
        };
        out.push(Completion {
            label: p.name.to_string(),
            kind: CompletionKind::Property,
            detail: Some(format!("property of {owner} ({ty})")),
            doc: doc(p.doc, p.origin, p.runtimes, p.from),
            insert: in_create.then(|| format!("{} = ", p.name)),
            snippet: false,
            sort: "0".into(),
        });
    }
    if !in_create {
        for m in c.methods.iter().filter(|m| !m.missing) {
            out.push(Completion {
                label: m.name.to_string(),
                kind: CompletionKind::Method,
                detail: Some(format!("method of {owner}: {}", m.signature())),
                doc: doc(m.doc, m.origin, m.runtimes, m.from),
                insert: None,
                snippet: false,
                sort: "1".into(),
            });
        }
    }
    for e in c.events.iter().filter(|e| !e.missing) {
        out.push(Completion {
            label: e.name.to_string(),
            kind: CompletionKind::Event,
            detail: Some(format!("event of {owner}: {}", e.signature())),
            doc: doc(e.doc, e.origin, e.runtimes, e.from),
            insert: Some(format!("{} = ", e.name)),
            snippet: false,
            sort: "2".into(),
        });
    }
}

/// The program's names visible at a preprocessed offset.
fn names(s: &Snapshot, pre: usize, out: &mut Out) {
    let scope = s.model.scope_at(pre);
    let in_routine = !matches!(s.model.scopes[scope].kind, ScopeKind::Program);
    for id in s.model.visible(scope) {
        let sym = &s.model.symbols[id];
        let (kind, sort) = match sym.kind {
            SymbolKind::Local | SymbolKind::Static => (CompletionKind::Variable, "0"),
            SymbolKind::Param => (CompletionKind::Parameter, "0"),
            SymbolKind::Global => (CompletionKind::Variable, if in_routine { "1" } else { "0" }),
            SymbolKind::Constant => (CompletionKind::Constant, "1"),
            SymbolKind::Component => (CompletionKind::Component, "1"),
            SymbolKind::Sub | SymbolKind::External => (CompletionKind::Sub, "1"),
            SymbolKind::Function => (CompletionKind::Function, "1"),
            SymbolKind::Type => (CompletionKind::Type, "2"),
            SymbolKind::Field => (CompletionKind::Field, "0"),
            SymbolKind::Label => continue,
        };
        let detail = match sym.kind {
            SymbolKind::Sub | SymbolKind::Function | SymbolKind::External => {
                context::routine_statement(&s.parsed.program, &sym.name).map(crate::signature::routine_label).map(|(l, _)| l)
            }
            SymbolKind::Component => sym.ty.as_deref().map(|t| pretty_component(rapidr_lang::resolve_component(t).map_or(t, |c| c.written_name()))),
            _ => sym.ty.as_ref().map(|t| format!("AS {t}")),
        };
        out.push(simple(&sym.name, kind, detail, sort));
    }
}

fn simple(label: &str, kind: CompletionKind, detail: Option<String>, sort: &str) -> Completion {
    Completion { label: label.to_string(), kind, detail, doc: None, insert: None, snippet: false, sort: sort.to_string() }
}

/// `akleft` → `akLeft`.
fn pretty_constant(name: &str) -> String {
    match name.strip_prefix("ak") {
        Some(rest) if !rest.is_empty() => format!("ak{}{}", rest[..1].to_ascii_uppercase(), &rest[1..]),
        _ => name.to_string(),
    }
}

/// Completions without duplicates (the first of a name wins).
#[derive(Default)]
pub(crate) struct Out {
    items: Vec<Completion>,
    seen: HashSet<String>,
}

impl Out {
    fn push(&mut self, c: Completion) {
        if self.seen.insert(name_key(&c.label)) {
            self.items.push(c);
        }
    }

    fn finish(self, start: usize, end: usize) -> Completions {
        let mut items = self.items;
        for (i, c) in items.iter_mut().enumerate() {
            c.sort = format!("{}{:05}", c.sort, i);
        }
        Completions { items, start, end }
    }
}
