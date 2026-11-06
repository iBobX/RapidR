//! QFormEx: the form a common RapidQ include (Rapidq2.inc) defines as
//! `TYPE QFormEx EXTENDS QFORM`, is RapidR's own RFormEx
//! (docs/rformex-design.md §6). The include reaches its features by writing
//! x86 machine code into memory and making it the window's procedure, which
//! RapidR never runs; RFormEx has the same members natively.
//!
//! - **The shape, not the file's name.** A TYPE named QFormEx that extends
//!   QFORM, whose every public member (field, custom event field with its
//!   parameter count, method, PROPERTY SET) is one of [`FIELDS`],
//!   [`EVENTS`], [`METHODS`] and whose EVENT blocks are OnShow / OnClose,
//!   is the include's: [`lower`] leaves it out (the include's other TYPEs,
//!   functions and declarations stay as they are), so `CREATE … AS
//!   QFormEx` makes an RFormEx. A QFormEx of another shape (an older or
//!   newer include, or a program's own class of that name) is not guessed
//!   at: [`check`] stops the compiler there and says what differs.
//! - **The name says which behaviour.** Written QFormEx, a form behaves as
//!   the include's did (the runtimes' "classic" flag, set by a first
//!   `__classic = 1` in its CREATE, or `Name.__classic = 1` after its DIM):
//!   an empty DragZone takes no drop, OnMouseWheel gets the screen position
//!   and Windows' key flags, OnMinimise comes before the minimise,
//!   Transparent reads 0 until set. Written RFormEx it has RapidR's
//!   behaviour (`rapidr_value::formex`). The parser keeps the name as
//!   written (`QFORMEX`) for this pass, which turns it into `RFORMEX`.

use crate::*;

/// RapidQ's name of the form, as the parser leaves it (upper case).
pub const QFORMEX: &str = "QFORMEX";
/// RapidR's.
pub const RFORMEX: &str = "RFORMEX";
/// The property that makes a form behave as the include's (runtimes:
/// `rapidr_value::formex::CLASSIC`).
pub const CLASSIC_PROPERTY: &str = "__classic";

/// The include's public fields RFormEx has (lower case).
pub const FIELDS: &[&str] = &["dragzone", "dragenable", "droppoint", "formstyle", "trayico", "trayiconhint", "trayiconindex", "trayiconupdate", "transparent", "transparentclient"];

/// The include's custom events, with how many values each hands its handler.
pub const EVENTS: &[(&str, usize)] = &[
    ("ondrag", 1),
    ("ontrayclick", 0),
    ("ontraydblclick", 0),
    ("onminimise", 0),
    ("onmousewheel", 4),
    ("onlostfocus", 0),
    ("ongetfocus", 0),
    ("onmessage", 4),
];

/// The include's public methods and property setters RFormEx answers.
pub const METHODS: &[&str] = &[
    "removetaskbar",
    "setformstyle",
    "setdragenable",
    "addtrayicon",
    "deltrayicon",
    "modifytrayicon",
    "settrayiconhint",
    "settrayiconupdate",
    "settrayiconindex",
    "center",
    "settransparentclient",
    "settransparent",
];

/// The EVENT blocks the include's TYPE handles itself (RFormEx does that
/// work on its own: `InheritOnShow` / `InheritOnClose` are accepted).
pub const TYPE_EVENTS: &[&str] = &["onshow", "onclose"];

/// Whether `t` is a TYPE named QFormEx (any case).
fn named_qformex(t: &TypeStatement) -> bool {
    t.name.eq_ignore_ascii_case(QFORMEX)
}

/// How a TYPE named QFormEx differs from the include's RFormEx stands for
/// (empty: it is that one). `declares` finds a custom event's template
/// (`DECLARE SUB OnDrag_T (file AS STRING)`) for its parameter count.
pub fn differences(t: &TypeStatement, declares: &[&DeclareStatement]) -> Vec<String> {
    let mut out = Vec::new();
    let base = t.extends.as_deref().map(canonical_type_name).unwrap_or_default();
    if !base.eq_ignore_ascii_case("RFORM") {
        let written = t.extends.as_deref().unwrap_or("nothing");
        out.push(format!("EXTENDS {} (QFORM there)", written.to_ascii_uppercase()));
    }
    let private = |n: &str| t.private.iter().any(|p| p.eq_ignore_ascii_case(n));
    for f in t.fields.iter().filter(|f| !private(&f.name)) {
        let name = f.name.to_ascii_lowercase();
        if f.type_name.eq_ignore_ascii_case("EVENT") {
            let Some((_, want)) = EVENTS.iter().find(|(e, _)| *e == name) else {
                out.push(f.name.clone());
                continue;
            };
            let template = t.event_templates.iter().find(|(n, _)| *n == name).map(|(_, tpl)| tpl.as_str()).unwrap_or("");
            if let Some(d) = declares.iter().find(|d| !template.is_empty() && d.name.eq_ignore_ascii_case(template)) {
                if d.params.len() != *want {
                    out.push(format!("{} with {} parameter(s) ({want} there)", f.name, d.params.len()));
                }
            }
        } else if !FIELDS.contains(&name.as_str()) && !EVENTS.iter().any(|(e, _)| *e == name) {
            out.push(f.name.clone());
        }
    }
    for m in &t.methods {
        let name = match m {
            Statement::Subroutine(s) => &s.name,
            Statement::Function(f) => &f.name,
            _ => continue,
        };
        if !private(name) && !METHODS.contains(&name.to_ascii_lowercase().as_str()) {
            out.push(name.clone());
        }
    }
    for e in &t.events {
        if !TYPE_EVENTS.contains(&e.name.to_ascii_lowercase().as_str()) {
            out.push(format!("EVENT {}", e.name));
        }
    }
    out
}

fn declares(program: &Program) -> Vec<&DeclareStatement> {
    program.statements.iter().filter_map(|s| if let Statement::Declare(d) = s { Some(d) } else { None }).collect()
}

/// Whether the program's TYPE QFormEx (if it has one) is the include's.
fn known(program: &Program) -> bool {
    let d = declares(program);
    program.statements.iter().all(|s| !matches!(s, Statement::Type(t) if named_qformex(t) && !differences(t, &d).is_empty()))
}

/// The compiler's error for a TYPE QFormEx of another shape, in plain words.
pub fn check(program: &Program) -> Vec<(TextSpan, String)> {
    let d = declares(program);
    let mut out = Vec::new();
    for s in &program.statements {
        let Statement::Type(t) = s else { continue };
        if !named_qformex(t) {
            continue;
        }
        let diff = differences(t, &d);
        if diff.is_empty() {
            continue;
        }
        // (rapidr_diagnostics' RR0047, word for word)
        out.push((
            t.span,
            format!(
                "TYPE {} isn't the QFormEx RapidR knows (Rapidq2.inc's): it differs in {}. RapidR has its own QFormEx (RFormEx: the same members, without changing the window's code in memory), so it doesn't compile this TYPE: leave the TYPE (or its $INCLUDE) out to use RapidR's, or give your class another name (MyFormEx) to keep it",
                t.name,
                diff.join(", ")
            ),
        ));
    }
    out
}

fn classic_mark(span: TextSpan, target: Expression) -> Statement {
    Statement::Assignment(AssignmentStatement { span, target, value: Expression::Literal(Literal { span, value: LiteralValue::Integer(1) }) })
}

fn is_q(t: &str) -> bool {
    t.trim().eq_ignore_ascii_case(QFORMEX)
}

/// The pass (see the module docs): the include's TYPE left out, QFormEx
/// made RFormEx, each one written QFormEx marked classic. A program whose
/// TYPE QFormEx has another shape is left as it is ([`check`] refuses it).
pub fn lower(program: &Program) -> Program {
    if !known(program) {
        return program.clone();
    }
    let mut program = program.clone();
    program.statements.retain(|s| !matches!(s, Statement::Type(t) if named_qformex(t)));
    // (types written QFormEx anywhere: parameters, results, fields, EXTENDS)
    let fix = |t: &mut String| {
        if is_q(t) {
            *t = RFORMEX.into();
        }
    };
    walk_statements_mut(&mut program.statements, &mut |s| match s {
        Statement::Subroutine(r) => r.params.iter_mut().for_each(|p| fix(&mut p.type_name)),
        Statement::Function(f) => {
            f.params.iter_mut().for_each(|p| fix(&mut p.type_name));
            if let Some(t) = f.return_type.as_mut() {
                fix(t);
            }
        }
        Statement::Declare(d) => d.params.iter_mut().for_each(|p| fix(&mut p.type_name)),
        Statement::Type(t) => {
            t.fields.iter_mut().for_each(|f| fix(&mut f.type_name));
            if t.extends.as_deref().is_some_and(is_q) {
                t.extends = Some(RFORMEX.into());
                let span = t.span;
                let this = Expression::Identifier(Identifier { span, name: "This".into() });
                let target = Expression::MemberAccess(MemberAccessExpression { span, object: Box::new(this), member: CLASSIC_PROPERTY.into() });
                t.constructor.insert(0, classic_mark(span, target));
            }
        }
        Statement::Create(c) if is_q(&c.type_name) => {
            c.type_name = RFORMEX.into();
            let target = Expression::Identifier(Identifier { span: c.span, name: CLASSIC_PROPERTY.into() });
            c.body.insert(0, classic_mark(c.span, target));
        }
        _ => {}
    });
    mark_dims(&mut program.statements);
    program
}

/// A DIM … AS QFormEx: RFormEx, each form it makes (not an array's)
/// marked right after it — in every block, SUBs' and CREATEs' included.
fn mark_dims(block: &mut Vec<Statement>) {
    for s in block.iter_mut() {
        for body in child_bodies_mut(s) {
            mark_dims(body);
        }
    }
    if !block.iter().any(|s| matches!(s, Statement::Dim(d) if is_q(&d.type_name))) {
        return;
    }
    let mut out = Vec::with_capacity(block.len() + 1);
    for mut s in block.drain(..) {
        let mut marks = Vec::new();
        if let Statement::Dim(d) = &mut s {
            if is_q(&d.type_name) {
                d.type_name = RFORMEX.into();
                for v in d.declarators.iter().filter(|v| v.dimensions.is_empty()) {
                    let object = Expression::Identifier(Identifier { span: v.span, name: v.name.clone() });
                    let target = Expression::MemberAccess(MemberAccessExpression { span: v.span, object: Box::new(object), member: CLASSIC_PROPERTY.into() });
                    marks.push(classic_mark(v.span, target));
                }
            }
        }
        out.push(s);
        out.extend(marks);
    }
    *block = out;
}
