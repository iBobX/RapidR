//! RapidR Studio's Help pane (docs/studio-wow.md HLP-1): the language
//! registry's entry for a word — a component, a built-in function or SUB, a
//! statement, a keyword, a directive, a type, a constant, a global object,
//! or a member of a component — and the program's own names (a variable, a
//! SUB, a component of the form), as a page the pane shows: its title, what
//! it is (kind, type, RapidQ's default, whether it is RapidQ's or RapidR's,
//! where it runs), the syntax (what Insert puts in the code), what it does,
//! and for a component its members, each one a link to its own entry. One
//! registry for the editor's hovers, the inspector, the toolbox and this
//! pane, offline and the same on the web.
//!
//! The page is Markdown (RMARKDOWNVIEW shows it): `help:Word[@Type]` links
//! open another entry, `help-insert:` types the syntax into the code,
//! `help-goto:` jumps to where a name of the program is declared,
//! `help-refs:` finds its references.

use rapidr_lang::{Component, Origin, Runtimes};
use rapidr_langsvc::SymbolHelp;

/// Who a name belongs to, in words.
fn origin(o: Origin) -> &'static str {
    match o {
        Origin::RapidQ => "RapidQ-compatible",
        Origin::RapidR => "RapidR's own (not in RapidQ)",
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

/// A member of a component (a property, a method, an event): its entry,
/// under RapidR's name of the component (`RButton.Caption`: R-NAMES).
fn member(c: &Component, name: &str) -> Option<Vec<String>> {
    let comp = c.spelling();
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

/// The names of a component's properties, methods and events (what RapidR
/// doesn't have yet left out), each kind in the order the registry has.
fn member_names(c: &Component) -> [(&'static str, Vec<&'static str>); 3] {
    [
        ("Properties", c.properties.iter().filter(|p| !p.missing).map(|p| p.name).collect()),
        ("Methods", c.methods.iter().filter(|m| !m.missing).map(|m| m.name).collect()),
        ("Events", c.events.iter().filter(|e| !e.missing).map(|e| e.name).collect()),
    ]
}

/// A component's entry under RapidR's name (`RButton`, RapidQ's as a
/// note): what it is and its members by kind.
fn component(c: &Component) -> Vec<String> {
    let name = c.spelling();
    let from = match c.rapidq {
        Some(q) if c.kind == rapidr_lang::Kind::Component => format!("RapidQ name: {q}"),
        _ => origin(c.origin).to_string(),
    };
    let mut what = format!("Component · {from} · {}", runs(c.runtimes));
    if let Some(e) = c.default_event {
        what.push_str(&format!(" · its default event {e}"));
    }
    let mut out = vec![name.clone(), format!("CREATE {}1 AS {name}\n\nEND CREATE", c.display), what, c.doc.to_string()];
    for (title, names) in member_names(c) {
        if !names.is_empty() {
            out.push(format!("{title}: {}", names.join(", ")));
        }
    }
    out
}

/// What an entry is of, for the page that shows it.
#[derive(Clone, Copy)]
pub enum Found {
    /// A component (its members are listed).
    Component(&'static Component),
    /// A global object (`Screen`: its members are listed too).
    Global(&'static Component),
    /// A member of a component.
    Member(&'static Component),
    /// Any other word of the language.
    Other,
}

/// The entry for `word` (`of`: the component type it is a member of, when
/// known — the inspector's row, `Edit1.` in the code): [title, syntax,
/// what it is, doc, more…]; `None` when the registry doesn't know it.
pub fn entry(word: &str, of: &str) -> Option<Vec<String>> {
    lookup(word, of).map(|(parts, _)| parts)
}

/// [`entry`], with what it is of.
pub fn lookup(word: &str, of: &str) -> Option<(Vec<String>, Found)> {
    let word = word.trim().trim_end_matches(['(', '.', ' ']);
    if word.is_empty() {
        return None;
    }
    if !of.trim().is_empty() {
        if let Some(c) = rapidr_lang::resolve_component(of.trim()).or_else(|| rapidr_lang::global(of.trim())).or_else(|| rapidr_lang::item(of.trim())) {
            if let Some(m) = member(c, word) {
                return Some((m, Found::Member(c)));
            }
        }
    }
    if let Some(c) = rapidr_lang::resolve_component(word) {
        return Some((component(c), Found::Component(c)));
    }
    let other = |parts: Vec<String>| Some((parts, Found::Other));
    if let Some(b) = rapidr_lang::builtin(word) {
        let mut syntax = if b.syntax.is_empty() { format!("{}{}", b.name, rapidr_lang::params_text(b.params)) } else { b.syntax.to_string() };
        if let (Some(r), true) = (b.returns, b.syntax.is_empty()) {
            syntax.push_str(&format!(" AS {r}"));
        }
        let what = format!("{} · {} · {}", if b.returns.is_some() { "Function" } else { "Statement" }, origin(b.origin), runs(b.runtimes));
        return other(vec![b.name.to_string(), syntax, what, b.doc.to_string()]);
    }
    if let Some(s) = rapidr_lang::statement(word) {
        return other(vec![s.name.to_string(), s.syntax.to_string(), format!("Statement · {}", origin(s.origin)), s.doc.to_string()]);
    }
    if let Some(d) = rapidr_lang::directive(word) {
        return other(vec![d.name.to_string(), d.syntax.to_string(), format!("Directive · {}", origin(d.origin)), d.doc.to_string()]);
    }
    if let Some(k) = rapidr_lang::keyword(word) {
        return other(vec![k.name.to_string(), k.name.to_string(), format!("Keyword ({}) · {}", k.kind, origin(k.origin)), k.doc.to_string()]);
    }
    if let Some(t) = rapidr_lang::type_name(word) {
        return other(vec![t.name.to_string(), t.name.to_string(), format!("Type · {}", origin(t.origin)), t.doc.to_string()]);
    }
    if let Some((v, g)) = rapidr_lang::constant(word) {
        let name = g.constants.iter().find(|(n, _)| n.eq_ignore_ascii_case(word)).map_or(word, |(n, _)| *n);
        return other(vec![name.to_string(), name.to_string(), format!("Constant = {v} (&H{v:X}) · {} · {}", g.name, origin(g.origin)), g.doc.to_string()]);
    }
    if let Some(c) = rapidr_lang::global(word) {
        let mut e = component(c);
        e[1] = c.spelling();
        e[2] = e[2].replacen("Component", "Object", 1);
        return Some((e, Found::Global(c)));
    }
    None
}

/// The components a member name is looked up in first (a member after a
/// dot, in a chat's answer, whose variable's type isn't known): the usual
/// ones, then the registry's order.
const USUAL: [&str; 12] = ["RForm", "RButton", "RLabel", "REdit", "RListBox", "RComboBox", "RMemo", "RPanel", "RTimer", "RCheckBox", "RRadioButton", "RImage"];

/// A member named `name` (a method, a property, an event) of some
/// component, when the component isn't known: the entry, and the
/// component's name.
fn member_anywhere(name: &str) -> Option<(Vec<String>, &'static str)> {
    let usual = USUAL.iter().filter_map(|n| rapidr_lang::component(n));
    let rest = rapidr_lang::COMPONENTS.iter().filter(|c| c.kind == rapidr_lang::Kind::Component);
    usual.chain(rest).find_map(|c| member(c, name).map(|m| (m, c.name)))
}

/// A text cut at its first sentence or clause ("…; …" — what follows is for
/// the page), at most `max` characters, cut at a word.
fn first_sentence(t: &str, max: usize) -> String {
    let one: String = t.split_whitespace().collect::<Vec<_>>().join(" ");
    let end = [". ", "; ", " — ", " (", ": "].iter().filter_map(|m| one.find(m).map(|i| if *m == ". " { i + 1 } else { i })).min().unwrap_or(one.len());
    let clause = &one[..end];
    if clause.chars().count() <= max {
        return clause.trim_end_matches([',', ';', ':']).to_string();
    }
    let cut: String = clause.chars().take(max).collect();
    let at = cut.rfind(' ').unwrap_or(cut.len());
    format!("{}…", cut[..at].trim_end_matches([',', ';', ':', ' ']))
}

/// A word of code as one line, for a tooltip: its name or syntax, what it
/// is, the first sentence of what it does ("PRINT — Statement: Writes
/// text to the console."). `member`: a dot comes before the word, so it is
/// a member of a component (looked up in the usual ones first); the
/// component's name goes with it. `None` when the registry doesn't know
/// the word.
pub fn quick(word: &str, member: bool) -> Option<rapidr_value::panels::markdown::WordHelp> {
    let (parts, of) = if member {
        let (parts, of) = member_anywhere(word)?;
        (parts, of.to_string())
    } else {
        (entry(word, "")?, String::new())
    };
    let head = match parts.first() {
        Some(t) if member => t.clone(),
        Some(t) => parts.get(1).map(|syn| syn.lines().next().unwrap_or(t)).filter(|syn| !syn.is_empty() && syn.chars().count() <= 40 && !syn.contains("CREATE")).unwrap_or(t).to_string(),
        None => return None,
    };
    let what = parts.get(2).map(|w| w.split(" · ").next().unwrap_or(w)).unwrap_or("");
    let what = what.split(" = ").next().unwrap_or(what);
    let doc = parts.get(3).map(|d| first_sentence(d, 72)).unwrap_or_default();
    let tip = match (what.is_empty(), doc.is_empty()) {
        (true, true) => head,
        (false, true) => format!("{head} — {what}"),
        (true, false) => format!("{head} — {doc}"),
        (false, false) => format!("{head} — {what}: {doc}"),
    };
    Some(rapidr_value::panels::markdown::WordHelp { tip, of })
}

/// The Help pane's text: the entry's parts a line each (the syntax's line
/// breaks as `\r`), or "" when nothing is known.
pub fn text(word: &str, of: &str) -> String {
    match entry(word, of) {
        Some(parts) => parts.iter().map(|p| p.replace('\n', "\r")).collect::<Vec<_>>().join("\n"),
        None => String::new(),
    }
}

// ------------------------------------------------------------------ page --

/// What the pane is asked to show.
#[derive(Default)]
pub struct Ask {
    /// The word (empty: the start page, unless `mine` says what is there).
    pub word: String,
    /// The component type the word is a member of, when known.
    pub of: String,
    /// A code editor can take the syntax: Insert is offered.
    pub can_insert: bool,
    /// The word is the program's own: what the language service says of it.
    pub mine: Option<SymbolHelp>,
    /// The program's own names (the suggestions' too).
    pub program: Vec<(String, &'static str)>,
}

/// The page's own heading level: a title a pane this narrow can hold (the
/// first level is the view's largest).
const TOP: usize = 2;

/// Text as Markdown shows it as written.
fn plain(t: &str) -> String {
    let mut out = String::with_capacity(t.len() + 4);
    for c in t.chars() {
        if matches!(c, '\\' | '*' | '_' | '`' | '[' | ']' | '<' | '>' | '#') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// A link to the entry for `word` (a member of `of`, when given).
fn link(text: &str, word: &str, of: &str) -> String {
    let target = if of.is_empty() { word.to_string() } else { format!("{word}@{of}") };
    format!("[{}](help:{})", plain(text), target.replace(' ', "%20"))
}

fn heading(level: usize, text: &str) -> String {
    format!("{} {}\n\n", "#".repeat(level), plain(text))
}

/// The syntax as a code block.
fn code(syntax: &str) -> String {
    format!("```basic\n{}\n```\n\n", syntax.trim_end())
}

/// A component's members as headings with a link each.
fn members(c: &Component, level: usize) -> String {
    let owner = c.spelling();
    let mut out = String::new();
    for (title, names) in member_names(c) {
        if names.is_empty() {
            continue;
        }
        out.push_str(&heading(level, &format!("{title} ({})", names.len())));
        out.push_str(&names.iter().map(|n| link(n, n, &owner)).collect::<Vec<_>>().join(" · "));
        out.push_str("\n\n");
    }
    out
}

/// An entry as a page at heading `level` (1: the page's own; 2: a section
/// under the program's name); the syntax (what Insert types) is left out
/// when `syntax` is false.
fn entry_page(parts: &[String], found: Found, level: usize, syntax: bool, can_insert: bool) -> (String, String) {
    let mut md = heading(level, &parts[0]);
    let mut what = parts[2].clone();
    let mut insert = String::new();
    if let Found::Member(c) = found {
        md.push_str(&format!("A member of {}.\n\n", link(&c.spelling(), &c.spelling(), "")));
    }
    if let Found::Component(c) | Found::Global(c) = found {
        if let Some(e) = c.default_event {
            what = what.replace(&format!(" · its default event {e}"), "");
        }
    }
    md.push_str(&format!("{}\n\n", plain(&what)));
    if let Found::Component(c) | Found::Global(c) = found {
        if let Some(e) = c.default_event {
            md.push_str(&format!("Its default event is {}.\n\n", link(e, e, &c.spelling())));
        }
    }
    if syntax && !parts[1].trim().is_empty() {
        md.push_str(&code(&parts[1]));
        insert = parts[1].replace('\n', "\r");
        if can_insert {
            md.push_str("[Insert into the code](help-insert:)\n\n");
        }
    }
    if !parts[3].trim().is_empty() {
        md.push_str(&format!("{}\n\n", parts[3].trim()));
    }
    if let Found::Component(c) | Found::Global(c) = found {
        md.push_str(&members(c, level + 1));
    }
    (md, insert)
}

/// Where Help starts, when nothing is asked: how to use it, and a few
/// words to begin with.
fn start_page() -> String {
    let mut md = heading(TOP, "Help");
    md.push_str("Put the caret on a word in your code and press **F1**, or type a word in the box above. Click a name here to open its page.\n\n");
    md.push_str(&heading(TOP + 1, "Start with"));
    for (w, what) in [
        ("RForm", "a window"),
        ("RButton", "a button to click"),
        ("RLabel", "text on the form"),
        ("PRINT", "show text"),
        ("IF", "choose what to do"),
        ("FOR", "repeat something"),
    ] {
        md.push_str(&format!("- {}: {what}\n", link(w, w, "")));
    }
    md.push_str("\nThe whole manual is at [the RapidR manual](https://github.com/iBobX/RapidR/tree/main/docs/manual).\n");
    md
}

/// The page for a word nothing knows: the closest names.
fn unknown_page(word: &str, program: &[(String, &'static str)]) -> String {
    let mut md = heading(TOP, &format!("No entry for {word}"));
    md.push_str(&format!("Nothing in the language, and nothing in this program, is called **{}**.\n\n", plain(word)));
    let near = suggest(word, program, 8);
    if near.is_empty() {
        md.push_str("Check the spelling, or go back to the [start page](help:).\n");
    } else {
        md.push_str("Did you mean:\n\n");
        for (n, kind) in near {
            md.push_str(&format!("- {}: {}\n", link(&n, &n, ""), plain(&kind)));
        }
        md.push_str("\nOr go back to the [start page](help:).\n");
    }
    md
}

/// The page for one of the program's own names: what it is, how it was
/// declared, where, and below it the help of its type.
fn mine_page(m: &SymbolHelp, can_insert: bool) -> String {
    let kind = match m.kind {
        "sub" => "SUB",
        "function" => "FUNCTION",
        "dll" => "DLL routine",
        "type" => "TYPE",
        k => k,
    };
    let mut md = heading(TOP, &m.name);
    let article = if kind.starts_with(['a', 'e', 'i', 'o', 'u']) { "An" } else { "A" };
    md.push_str(&format!("{article} {} in this program", plain(kind)));
    if m.what != kind && !m.what.eq_ignore_ascii_case(kind) {
        md.push_str(&format!(" · {}", m.what.replace('`', "")));
    }
    md.push_str("\n\n");
    md.push_str(&code(&m.code));
    if let Some(f) = &m.file {
        let name = f.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        md.push_str(&format!("Declared in [{}, line {}](help-goto:)", plain(&name), m.line));
        md.push_str(&format!(" · {}", uses(m.uses)));
        md.push_str("\n\n");
    } else {
        md.push_str(&format!("{}\n\n", uses(m.uses)));
    }
    md.push_str("[Find References](help-refs:)\n\n");
    let routine = matches!(m.kind, "sub" | "function" | "dll");
    if let (Some(t), false) = (&m.ty, routine) {
        let shown = rapidr_lang::resolve_component(t).map_or_else(|| t.clone(), |c| c.spelling());
        md.push_str(&format!("Its type is {}.\n\n", link(&shown, &shown, "")));
        if let Some((parts, found)) = lookup(t, "") {
            md.push_str(&entry_page(&parts, found, TOP + 1, false, can_insert).0);
        }
    }
    md
}

fn uses(n: usize) -> String {
    match n {
        0 => "not used yet".to_string(),
        1 => "used once".to_string(),
        n => format!("used {n} times"),
    }
}

/// The page for `ask`: a first line of what the pane needs besides the page
/// — `kind`, the syntax Insert types (its line breaks as `\r`), the file the
/// name is declared in and its line and column, tab-separated; `kind` is `start`,
/// `none`, `entry` or `mine` — then the page's Markdown.
pub fn page(ask: &Ask) -> String {
    let word = ask.word.trim().trim_end_matches(['(', '.', ' ']);
    let (kind, insert, file, line, column, md) = if let Some(m) = &ask.mine {
        let file = m.file.as_ref().map(|f| f.to_string_lossy().replace('\\', "/")).unwrap_or_default();
        ("mine", String::new(), file, m.line, m.column, mine_page(m, ask.can_insert))
    } else if word.is_empty() {
        ("start", String::new(), String::new(), 0, 0, start_page())
    } else if let Some((parts, found)) = lookup(word, &ask.of).or_else(|| lookup(word, "")) {
        let (md, insert) = entry_page(&parts, found, TOP, true, ask.can_insert);
        ("entry", insert, String::new(), 0, 0, md)
    } else {
        ("none", String::new(), String::new(), 0, 0, unknown_page(word, &ask.program))
    };
    let insert = if ask.can_insert { insert } else { String::new() };
    format!("{kind}\t{insert}\t{file}\t{line}\t{column}\n{md}")
}

// ---------------------------------------------------------------- search --

/// Edit distance between two short names.
fn distance(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cur = row[j + 1];
            row[j + 1] = (prev + usize::from(ca != cb)).min(row[j] + 1).min(cur + 1);
            prev = cur;
        }
    }
    row[b.len()]
}

/// The names `query` means, best first: [name, what it is] — the program's
/// own, then the language's (components under RapidR's names, RapidQ's
/// matching too; functions, statements, keywords, types, directives,
/// global objects, and from two letters on the constants). A name that
/// starts with the query comes before one that has it inside; when nothing
/// does, the names it is close to.
pub fn suggest(query: &str, program: &[(String, &'static str)], limit: usize) -> Vec<(String, String)> {
    let q = query.trim().trim_start_matches('$').to_ascii_lowercase();
    if q.is_empty() || limit == 0 {
        return Vec::new();
    }
    // (shown name, other names it is found by, what it is)
    let mut all: Vec<(String, Vec<String>, String)> = Vec::new();
    for (n, k) in program {
        all.push((n.clone(), Vec::new(), format!("your {k}")));
    }
    for c in rapidr_lang::COMPONENTS.iter().filter(|c| c.kind == rapidr_lang::Kind::Component) {
        let mut keys: Vec<String> = c.rapidq.into_iter().map(|q| q.to_string()).collect();
        keys.extend(c.aliases.iter().map(|a| a.to_string()));
        all.push((c.spelling(), keys, "component".into()));
    }
    for g in rapidr_lang::GLOBALS {
        all.push((g.spelling(), Vec::new(), "object".into()));
    }
    for b in rapidr_lang::BUILTINS.iter().filter(|b| !b.missing) {
        all.push((b.name.to_string(), Vec::new(), if b.returns.is_some() { "function" } else { "statement" }.into()));
    }
    for s in rapidr_lang::STATEMENTS {
        all.push((s.name.to_string(), Vec::new(), "statement".into()));
    }
    for k in rapidr_lang::KEYWORDS {
        all.push((k.name.to_string(), Vec::new(), "keyword".into()));
    }
    for t in rapidr_lang::TYPE_NAMES {
        all.push((t.name.to_string(), Vec::new(), "type".into()));
    }
    for d in rapidr_lang::DIRECTIVES {
        all.push((d.name.to_string(), Vec::new(), "directive".into()));
    }
    if q.len() >= 2 {
        for g in rapidr_lang::CONSTANT_GROUPS {
            for (n, _) in g.constants {
                all.push((n.to_string(), Vec::new(), "constant".into()));
            }
        }
    }
    // (rank, order in `all`)
    let rank = |n: &str, keys: &[String]| -> Option<u8> {
        let mut best: Option<u8> = None;
        for name in std::iter::once(n).chain(keys.iter().map(String::as_str)) {
            let l = name.trim_start_matches('$').to_ascii_lowercase();
            let r = if l == q {
                0
            } else if l.starts_with(&q) {
                1
            } else if l.contains(&q) {
                2
            } else {
                continue;
            };
            best = Some(best.map_or(r, |b| b.min(r)));
        }
        best
    };
    // (the rank, then the shorter name — what was typed is closer to PRINT than to Printer —, then the order the sources have)
    let mut hits: Vec<(u8, usize, usize)> = all.iter().enumerate().filter_map(|(i, (n, keys, _))| rank(n, keys).map(|r| (r, n.len(), i))).collect();
    hits.sort();
    let mut out: Vec<(String, String)> = Vec::new();
    for (_, _, i) in hits {
        let (n, _, what) = &all[i];
        if !out.iter().any(|(o, _)| o.eq_ignore_ascii_case(n)) {
            out.push((n.clone(), what.clone()));
            if out.len() >= limit {
                break;
            }
        }
    }
    if out.is_empty() && q.len() >= 3 {
        let reach = (q.len() / 3).max(1);
        let mut near: Vec<(usize, usize)> =
            all.iter().enumerate().filter(|(_, (_, _, w))| w != "constant").filter_map(|(i, (n, _, _))| Some((distance(&n.to_ascii_lowercase(), &q), i)).filter(|(d, _)| *d <= reach)).collect();
        near.sort();
        for (_, i) in near {
            let (n, _, what) = &all[i];
            if !out.iter().any(|(o, _)| o.eq_ignore_ascii_case(n)) {
                out.push((n.clone(), what.clone()));
                if out.len() >= limit.min(5) {
                    break;
                }
            }
        }
    }
    out
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
        // (RapidR's names, whichever name was asked: R-NAMES)
        let b = entry("QBUTTON", "").unwrap();
        assert_eq!(b[0], "RButton");
        assert!(b[1].starts_with("CREATE Button1 AS RButton"), "{b:?}");
        assert!(b[2].starts_with("Component · RapidQ name: QBUTTON"), "{b:?}");
        assert!(b.iter().any(|l| l.starts_with("Properties: ") && l.contains("Caption")), "{b:?}");
        let p = entry("Caption", "QBUTTON").unwrap();
        assert_eq!(p[0], "RButton.Caption");
        assert!(p[2].starts_with("Property of RButton · string"), "{p:?}");
        let ev = entry("OnClick", "qbutton").unwrap();
        assert!(ev[2].starts_with("Event of RButton"), "{ev:?}");
        assert!(entry("RPLOT", "").unwrap()[2].contains("RapidR's own (not in RapidQ)"));
        assert_eq!(entry("RPLOT", "").unwrap()[0], "RPlot");
        assert!(entry("clRed", "").unwrap()[2].starts_with("Constant = "));
        assert!(entry("FOR", "").is_some());
        assert!(entry("nothing_here_at_all", "").is_none());
        assert_eq!(text("nothing_here_at_all", ""), "");
        assert_eq!(text("QBUTTON", "").lines().next(), Some("RButton"));
    }

    fn ask(word: &str, of: &str) -> Ask {
        Ask { word: word.into(), of: of.into(), can_insert: true, ..Ask::default() }
    }

    /// The first line (its fields) and the page.
    fn parts(a: &Ask) -> (Vec<String>, String) {
        let t = page(a);
        let (meta, md) = t.split_once('\n').unwrap();
        (meta.split('\t').map(str::to_string).collect(), md.to_string())
    }

    #[test]
    fn a_component_page_groups_its_members_as_links() {
        let (meta, md) = parts(&ask("RButton", ""));
        assert_eq!(meta[0], "entry");
        assert!(meta[1].starts_with("CREATE Button1 AS RButton"), "{meta:?}");
        assert!(md.starts_with("## RButton\n\nComponent · RapidQ name: QBUTTON · desktop and web\n\n"), "{md}");
        // (its default event is a link, not a tail the line cuts off)
        assert!(md.contains("Its default event is [OnClick](help:OnClick@RButton)."), "{md}");
        assert!(!md.contains("its default event OnClick"), "{md}");
        assert!(md.contains("```basic\nCREATE Button1 AS RButton"), "{md}");
        assert!(md.contains("[Insert into the code](help-insert:)"), "{md}");
        for h in ["### Properties (", "### Methods (", "### Events ("] {
            assert!(md.contains(h), "{h}: {md}");
        }
        assert!(md.contains("[Caption](help:Caption@RButton)"), "{md}");
        // (every link names an entry that exists)
        for l in md.split("](help:").skip(1) {
            let target = l.split(')').next().unwrap().replace("%20", " ");
            let (w, of) = target.split_once('@').unwrap_or((&target, ""));
            assert!(w.is_empty() || lookup(w, of).is_some(), "no entry for {target}");
        }
        // (no editor to take it: no Insert, and still the syntax to read)
        let none = page(&Ask { can_insert: false, ..ask("RButton", "") });
        assert!(none.starts_with("entry\t\t"), "{none}");
        assert!(!none.contains("help-insert"), "{none}");
        assert!(none.contains("```basic"), "{none}");
    }

    #[test]
    fn a_member_page_says_whose_member_it_is() {
        let (meta, md) = parts(&ask("Caption", "RButton"));
        assert_eq!(meta[0], "entry");
        assert!(md.starts_with("## RButton.Caption"), "{md}");
        assert!(md.contains("A member of [RButton](help:RButton)."), "{md}");
        assert!(md.contains("Property of RButton"), "{md}");
    }

    #[test]
    fn the_start_page_and_an_unknown_word() {
        let (meta, md) = parts(&ask("", ""));
        assert_eq!(meta[0], "start");
        assert!(md.contains("press **F1**") && md.contains("[RForm](help:RForm)") && md.contains("[PRINT](help:PRINT)") && md.contains("[IF](help:IF)"), "{md}");
        for l in md.split("](help:").skip(1) {
            let target = l.split(')').next().unwrap();
            assert!(target.is_empty() || lookup(target, "").is_some(), "no entry for {target}");
        }
        let (meta, md) = parts(&ask("RapidR", ""));
        assert_eq!(meta[0], "none");
        assert!(md.starts_with("## No entry for RapidR"), "{md}");
        assert!(!md.contains("Nothing in the language is called"), "{md}");
        // (a word near one: its names to click)
        let (_, md) = parts(&ask("RButon", ""));
        assert!(md.contains("[RButton](help:RButton)"), "{md}");
    }

    #[test]
    fn a_name_of_the_program_has_its_declaration_and_its_types_help_below() {
        let mine = SymbolHelp {
            name: "PickedFont".into(),
            kind: "variable",
            what: "global variable".into(),
            code: "DIM PickedFont AS RFont".into(),
            ty: Some("RFONT".into()),
            file: Some("/work/main.rr".into()),
            line: 5,
            column: 12,
            uses: 3,
        };
        let (meta, md) = parts(&Ask { word: "PickedFont".into(), mine: Some(mine.clone()), can_insert: true, ..Ask::default() });
        assert_eq!(meta, ["mine", "", "/work/main.rr", "5", "12"]);
        assert!(md.starts_with("## PickedFont\n\nA variable in this program · global variable\n\n```basic\nDIM PickedFont AS RFont\n```\n\n"), "{md}");
        assert!(md.contains("Declared in [main.rr, line 5](help-goto:) · used 3 times"), "{md}");
        assert!(md.contains("[Find References](help-refs:)"), "{md}");
        // (the type's help under it: its members, no syntax to insert)
        assert!(md.contains("Its type is [RFont](help:RFont)."), "{md}");
        assert!(md.contains("### RFont\n\n") && md.contains("#### Properties ("), "{md}");
        assert!(!md.contains("help-insert") && !md.contains("CREATE Font1"), "{md}");
        // (a SUB: its signature and the references, no type section)
        let sub = SymbolHelp { name: "Greet".into(), kind: "sub", what: "SUB".into(), code: "SUB Greet(Who AS STRING)".into(), ty: None, line: 22, uses: 1, ..mine };
        let (_, md) = parts(&Ask { word: "Greet".into(), mine: Some(sub), ..Ask::default() });
        assert!(md.starts_with("## Greet\n\nA SUB in this program\n\n```basic\nSUB Greet(Who AS STRING)\n```\n\n"), "{md}");
        assert!(md.contains("[Find References](help-refs:)") && !md.contains("Its type is"), "{md}");
    }

    #[test]
    fn suggestions_come_best_first() {
        let program = vec![("PickedFont".to_string(), "variable")];
        let s = suggest("pick", &program, 8);
        assert_eq!(s[0], ("PickedFont".into(), "your variable".into()), "{s:?}");
        let s = suggest("rbut", &[], 8);
        assert_eq!(s[0].0, "RButton", "{s:?}");
        // (RapidQ's name finds RapidR's)
        assert_eq!(suggest("QBUTTON", &[], 3)[0].0, "RButton");
        // (an entry's own name first, the longer ones after)
        let s = suggest("print", &[], 8);
        assert!(s[0].0.eq_ignore_ascii_case("print"), "{s:?}");
        // (a prefix: the shortest name first)
        let s = suggest("pri", &[], 8);
        assert_eq!(s[0].0, "PRINT", "{s:?}");
        assert!(suggest("", &[], 8).is_empty());
        assert!(suggest("zzqqxx", &[], 8).is_empty());
        assert!(suggest("clre", &[], 8).iter().any(|(n, k)| n == "clRed" && k == "constant"));
    }

    /// The chat's hover: a word of code in one line, a member after a dot
    /// with the component it was found in.
    #[test]
    fn a_word_of_code_is_one_line_of_help() {
        let t = |w: &str, m: bool| quick(w, m).map(|h| (h.tip, h.of));
        let (tip, of) = t("PRINT", false).expect("a statement");
        assert!(tip.starts_with("PRINT") && tip.contains(" — "), "{tip}");
        assert!(of.is_empty() && !tip.contains('\n') && tip.chars().count() < 200, "{tip}");
        let (tip, _) = t("RListBox", false).expect("a component");
        assert!(tip.starts_with("RListBox — Component"), "{tip}");
        let (tip, of) = t("AddItems", true).expect("a member");
        assert!(tip.contains("AddItems") && !of.is_empty(), "{tip} / {of}");
        assert!(t("Total", false).is_none() && t("NoSuchMember", true).is_none(), "the program's own names and unknown words have none");
    }
}
