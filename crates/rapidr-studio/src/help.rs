//! RapidR Studio's Help pane (docs/studio-wow.md HLP-1): the language
//! registry's entry for a word — a component, a built-in function or SUB, a
//! statement, a keyword, a directive, a type, a constant, a global object,
//! or a member of a component — as text the pane shows: its title, the
//! syntax (what Insert puts in the code), what it is (kind, type, RapidQ's
//! default, whether it is RapidQ's or RapidR's, where it runs), what it
//! does, and for a component its members. One registry for the editor's
//! hovers, the inspector, the toolbox and this pane, offline and the same
//! on the web.

use rapidr_lang::{Component, Origin, Runtimes};

/// Who a name belongs to, in words.
fn origin(o: Origin) -> &'static str {
    match o {
        Origin::RapidQ => "RapidQ",
        Origin::RapidR => "RapidR (not in RapidQ)",
    }
}

fn runs(r: Runtimes) -> &'static str {
    match r {
        Runtimes::All => "desktop and web",
        Runtimes::Desktop => "desktop only",
        Runtimes::Web => "web only",
    }
}

fn default_text(d: &rapidr_lang::DefaultValue) -> String {
    use rapidr_lang::DefaultValue as D;
    match d {
        D::Int(i) => i.to_string(),
        D::Float(f) => f.to_string(),
        D::Bool(b) => if *b { "True".into() } else { "False".into() },
        D::Str(s) => format!("\"{s}\""),
        D::Expr(e) => e.to_string(),
    }
}

/// A member of a component (a property, a method, an event): its entry.
fn member(c: &Component, name: &str) -> Option<Vec<String>> {
    let comp = c.written_name();
    if let Some(p) = c.property(name) {
        let mut what = format!("Property of {comp} · {}", p.ty.as_str());
        if let Some(d) = &p.default {
            what.push_str(&format!(" · default {}", default_text(d)));
        }
        what.push_str(&format!(" · {} · {}", origin(p.origin), runs(p.runtimes)));
        return Some(vec![format!("{comp}.{}", p.name), p.name.to_string(), what, p.doc.to_string()]);
    }
    if let Some(m) = c.method(name) {
        let what = format!("Method of {comp} · {} · {}", origin(m.origin), runs(m.runtimes));
        return Some(vec![format!("{comp}.{}", m.name), m.signature(), what, m.doc.to_string()]);
    }
    if let Some(e) = c.event(name) {
        let what = format!("Event of {comp} · its handler: SUB Name{} · {} · {}", rapidr_lang::params_text(e.params), origin(e.origin), runs(e.runtimes));
        return Some(vec![format!("{comp}.{}", e.name), format!("{} = Name", e.name), what, e.doc.to_string()]);
    }
    None
}

/// A component's entry: what it is and its members by kind.
fn component(c: &Component) -> Vec<String> {
    let name = c.written_name();
    let mut what = format!("Component · {} · {}", origin(c.origin), runs(c.runtimes));
    if let Some(e) = c.default_event {
        what.push_str(&format!(" · its default event {e}"));
    }
    let list = |names: Vec<&str>| names.join(", ");
    let mut out = vec![name.to_string(), format!("CREATE {}1 AS {name}\n\nEND CREATE", c.display), what, c.doc.to_string()];
    let props: Vec<&str> = c.properties.iter().filter(|p| !p.missing).map(|p| p.name).collect();
    let methods: Vec<&str> = c.methods.iter().filter(|m| !m.missing).map(|m| m.name).collect();
    let events: Vec<&str> = c.events.iter().filter(|e| !e.missing).map(|e| e.name).collect();
    if !props.is_empty() {
        out.push(format!("Properties: {}", list(props)));
    }
    if !methods.is_empty() {
        out.push(format!("Methods: {}", list(methods)));
    }
    if !events.is_empty() {
        out.push(format!("Events: {}", list(events)));
    }
    out
}

/// The entry for `word` (`of`: the component type it is a member of, when
/// known — the inspector's row, `Edit1.` in the code): [title, syntax,
/// what it is, doc, more…]; `None` when the registry doesn't know it.
pub fn entry(word: &str, of: &str) -> Option<Vec<String>> {
    let word = word.trim().trim_end_matches(['(', '.', ' ']);
    if word.is_empty() {
        return None;
    }
    if !of.trim().is_empty() {
        if let Some(c) = rapidr_lang::resolve_component(of.trim()).or_else(|| rapidr_lang::global(of.trim())) {
            if let Some(m) = member(c, word) {
                return Some(m);
            }
        }
    }
    if let Some(c) = rapidr_lang::resolve_component(word) {
        return Some(component(c));
    }
    if let Some(b) = rapidr_lang::builtin(word) {
        let mut syntax = if b.syntax.is_empty() { format!("{}{}", b.name, rapidr_lang::params_text(b.params)) } else { b.syntax.to_string() };
        if let (Some(r), true) = (b.returns, b.syntax.is_empty()) {
            syntax.push_str(&format!(" AS {r}"));
        }
        let what = format!("{} · {} · {}", if b.returns.is_some() { "Function" } else { "Statement" }, origin(b.origin), runs(b.runtimes));
        return Some(vec![b.name.to_string(), syntax, what, b.doc.to_string()]);
    }
    if let Some(s) = rapidr_lang::statement(word) {
        return Some(vec![s.name.to_string(), s.syntax.to_string(), format!("Statement · {}", origin(s.origin)), s.doc.to_string()]);
    }
    if let Some(d) = rapidr_lang::directive(word) {
        return Some(vec![d.name.to_string(), d.syntax.to_string(), format!("Directive · {}", origin(d.origin)), d.doc.to_string()]);
    }
    if let Some(k) = rapidr_lang::keyword(word) {
        return Some(vec![k.name.to_string(), k.name.to_string(), format!("Keyword ({}) · {}", k.kind, origin(k.origin)), k.doc.to_string()]);
    }
    if let Some(t) = rapidr_lang::type_name(word) {
        return Some(vec![t.name.to_string(), t.name.to_string(), format!("Type · {}", origin(t.origin)), t.doc.to_string()]);
    }
    if let Some((v, g)) = rapidr_lang::constant(word) {
        let name = g.constants.iter().find(|(n, _)| n.eq_ignore_ascii_case(word)).map_or(word, |(n, _)| *n);
        return Some(vec![name.to_string(), name.to_string(), format!("Constant = {v} (&H{v:X}) · {} · {}", g.name, origin(g.origin)), g.doc.to_string()]);
    }
    if let Some(c) = rapidr_lang::global(word) {
        let mut e = component(c);
        e[1] = c.written_name().to_string();
        e[2] = e[2].replacen("Component", "Object", 1);
        return Some(e);
    }
    None
}

/// The Help pane's text: the entry's parts a line each (the syntax's line
/// breaks as `\r`), or "" when nothing is known.
pub fn text(word: &str, of: &str) -> String {
    match entry(word, of) {
        Some(parts) => parts.iter().map(|p| p.replace('\n', "\r")).collect::<Vec<_>>().join("\n"),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_registry_answers_for_every_kind_of_word() {
        let e = entry("showmessage", "").unwrap();
        assert_eq!(e[0], "SHOWMESSAGE");
        assert!(e[1].to_ascii_uppercase().starts_with("SHOWMESSAGE"), "{e:?}");
        assert!(e[2].contains("RapidQ"), "{e:?}");
        assert!(!e[3].is_empty());
        let b = entry("QBUTTON", "").unwrap();
        assert_eq!(b[0], "QBUTTON");
        assert!(b[1].starts_with("CREATE Button1 AS QBUTTON"), "{b:?}");
        assert!(b.iter().any(|l| l.starts_with("Properties: ") && l.contains("Caption")), "{b:?}");
        let p = entry("Caption", "QBUTTON").unwrap();
        assert_eq!(p[0], "QBUTTON.Caption");
        assert!(p[2].starts_with("Property of QBUTTON · string"), "{p:?}");
        let ev = entry("OnClick", "qbutton").unwrap();
        assert!(ev[2].starts_with("Event of QBUTTON"), "{ev:?}");
        assert!(entry("RPLOT", "").unwrap()[2].contains("RapidR (not in RapidQ)"));
        assert!(entry("clRed", "").unwrap()[2].starts_with("Constant = "));
        assert!(entry("FOR", "").is_some());
        assert!(entry("nothing_here_at_all", "").is_none());
        assert_eq!(text("nothing_here_at_all", ""), "");
        assert_eq!(text("QBUTTON", "").lines().next(), Some("QBUTTON"));
    }
}
