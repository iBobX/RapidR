//! The registry's AI facets (docs/ai-architecture.md §2): what an AI needs
//! of every entry to use it correctly and safely — semantics, types, ranges
//! and units, side effects (and the risk they carry), examples, where it
//! runs — the derivations made from them, and the completeness checker that
//! lists every entry lacking one ([`check`]; `rapidr lang ai-check`).
//!
//! The vocabularies are shared with build.rs (`ai_vocab.rs`), which refuses
//! a word outside them.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::export::json_str;
use crate::*;

include!("ai_vocab.rs");

/// What calling a method or builtin (or setting a property) does beyond
/// returning a value. In [`EFFECT_NAMES`]' order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Effect {
    /// Computes a value; changes nothing.
    None,
    /// Changes what the program shows; reversible.
    Ui,
    /// Waits for a person to answer (a modal dialog, INPUT).
    Wait,
    /// Changes the program's own data.
    State,
    Clipboard,
    FileRead,
    Network,
    /// A serial port, sound, a camera, a joystick, a CD drive.
    Device,
    /// The settings store (QREGISTRY).
    Registry,
    DbRead,
    /// Creates, overwrites, renames or deletes files or folders.
    FileWrite,
    DbWrite,
    Printer,
    /// Ends the program.
    Exit,
    /// Runs another program.
    Process,
    /// Native code or raw memory.
    Native,
}

/// How much an entry can do, lowest first: what permission tiers are
/// expressed in (docs/ai-architecture.md §5.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Risk {
    /// Changes nothing.
    Read,
    /// Changes what the program shows.
    Ui,
    /// Changes the program's own data.
    State,
    /// Reaches outside the program (files read, the network, devices …).
    External,
    /// Can't be undone (files written, a database changed, printing, the end).
    Irreversible,
    /// Runs other programs or native code.
    System,
}

impl Effect {
    pub const ALL: [Effect; 16] = [
        Effect::None,
        Effect::Ui,
        Effect::Wait,
        Effect::State,
        Effect::Clipboard,
        Effect::FileRead,
        Effect::Network,
        Effect::Device,
        Effect::Registry,
        Effect::DbRead,
        Effect::FileWrite,
        Effect::DbWrite,
        Effect::Printer,
        Effect::Exit,
        Effect::Process,
        Effect::Native,
    ];

    /// As the registry writes it (`file-write`).
    pub fn as_str(self) -> &'static str {
        EFFECT_NAMES[self as usize]
    }

    pub fn parse(name: &str) -> Option<Effect> {
        EFFECT_NAMES.iter().position(|n| n.eq_ignore_ascii_case(name)).map(|i| Effect::ALL[i])
    }

    pub fn risk(self) -> Risk {
        Risk::ALL[EFFECT_RISK[self as usize] as usize]
    }

    /// It reaches outside the program: a capability a component needs.
    pub fn is_capability(self) -> bool {
        self.risk() >= Risk::External
    }
}

impl Risk {
    pub const ALL: [Risk; 6] = [Risk::Read, Risk::Ui, Risk::State, Risk::External, Risk::Irreversible, Risk::System];

    pub fn as_str(self) -> &'static str {
        RISK_NAMES[self as usize]
    }

    pub fn parse(name: &str) -> Option<Risk> {
        RISK_NAMES.iter().position(|n| n.eq_ignore_ascii_case(name)).map(|i| Risk::ALL[i])
    }

    /// In a confirmation's words.
    pub fn explain(self) -> &'static str {
        match self {
            Risk::Read => "It only reads.",
            Risk::Ui => "It changes what the program shows.",
            Risk::State => "It changes the program's data.",
            Risk::External => "It reaches outside the program (files, the network or a device).",
            Risk::Irreversible => "It can't be undone.",
            Risk::System => "It runs other programs or native code.",
        }
    }
}

/// The highest risk of some effects; `None` when there are none (not said).
pub fn risk_of(effects: &[Effect]) -> Option<Risk> {
    effects.iter().map(|e| e.risk()).max()
}

impl Method {
    /// Its risk: its effects' (raised by `risk`); `None` while its effects
    /// aren't said.
    pub fn risk(&self) -> Option<Risk> {
        risk_of(self.effects).map(|r| self.risk.map_or(r, |x| x.max(r)))
    }
}

impl Builtin {
    pub fn risk(&self) -> Option<Risk> {
        risk_of(self.effects).map(|r| self.risk.map_or(r, |x| x.max(r)))
    }

    /// Whether its syntax takes arguments (`MID$(text, start)`, `KILL f$`),
    /// not just its name (`TIMER`).
    pub fn takes_arguments(&self) -> bool {
        let syntax = self.syntax.trim();
        let rest = match syntax.get(..self.name.len()) {
            Some(head) if head.eq_ignore_ascii_case(self.name) => &syntax[self.name.len()..],
            _ => syntax,
        };
        let rest = rest.trim();
        !rest.is_empty() && rest != "()" && rest != "[()]"
    }
}

static UI: [Effect; 1] = [Effect::Ui];
static STATE: [Effect; 1] = [Effect::State];

impl Property {
    /// What setting it does: its own `effects`, else `ui` on a visual
    /// component and `state` on any other (docs/ai-architecture.md §2.3).
    pub fn set_effects(&self, owner: &Component) -> &'static [Effect] {
        if !self.effects.is_empty() {
            self.effects
        } else if owner.visual {
            &UI
        } else {
            &STATE
        }
    }

    pub fn set_risk(&self, owner: &Component) -> Risk {
        risk_of(self.set_effects(owner)).unwrap_or(Risk::State)
    }

    /// Whether a program (or an AI) can set it.
    pub fn writable(&self) -> bool {
        self.access != Access::Read
    }
}

impl Component {
    /// What it can reach outside the program: the effects above `state` of
    /// its members (methods and properties that say them), sorted.
    pub fn capabilities(&self) -> Vec<Effect> {
        let mut out: Vec<Effect> = self
            .methods
            .iter()
            .filter(|m| !m.missing)
            .flat_map(|m| m.effects.iter().copied())
            .chain(self.properties.iter().filter(|p| !p.missing).flat_map(|p| p.effects.iter().copied()))
            .filter(|e| e.is_capability())
            .collect();
        out.sort();
        out.dedup();
        out
    }

    /// The highest risk of its members; `None` while a method's effects
    /// aren't said.
    pub fn risk(&self) -> Option<Risk> {
        let mut max = Risk::Read;
        for m in self.methods.iter().filter(|m| !m.missing) {
            max = max.max(m.risk()?);
        }
        for p in self.properties.iter().filter(|p| !p.missing && p.writable()) {
            max = max.max(p.set_risk(self));
        }
        Some(max)
    }

    /// The name examples give an instance: `Button1`; a global object its
    /// own name (`Screen`).
    pub fn instance_name(&self) -> String {
        match self.kind {
            Kind::Global => self.display.to_string(),
            _ => format!("{}1", self.display),
        }
    }
}

// --- generated examples (docs/ai-architecture.md §2.3) -----------------------------

/// An example value of a BASIC type (a parameter's).
fn example_arg(p: &Param) -> String {
    match p.ty.to_ascii_uppercase().as_str() {
        "STRING" => "\"text\"".into(),
        "DOUBLE" | "SINGLE" | "FLOAT" => "1.5".into(),
        "INTEGER" | "LONG" | "WORD" | "BYTE" | "SHORT" | "DWORD" | "INT64" | "BOOLEAN" | "" => "0".into(),
        // (an object: a variable of that type)
        _ => p.name.trim_end_matches("()").to_string(),
    }
}

/// An example value for property `p` of `c`: its default, else one of its
/// type and range.
fn example_value(p: &Property) -> String {
    match p.ty {
        Type::String => match &p.default {
            Some(DefaultValue::Str(s)) if !s.is_empty() => format!("\"{s}\""),
            _ => "\"text\"".into(),
        },
        Type::Int | Type::Float => {
            let low = p.range.and_then(|r| r.split("..").next()).map(str::trim).filter(|l| l.parse::<f64>().is_ok());
            match (&p.default, low) {
                (Some(DefaultValue::Int(i)), _) if *i != 0 => i.to_string(),
                (Some(DefaultValue::Float(f)), _) if *f != 0.0 => f.to_string(),
                (_, Some(l)) if l != "0" => l.to_string(),
                _ => if p.ty == Type::Float { "1.5".into() } else { "10".into() },
            }
        }
        Type::Bool => match p.default {
            Some(DefaultValue::Bool(true)) => "0".into(),
            _ => "1".into(),
        },
        Type::Enum | Type::Set => {
            let default = match &p.default {
                Some(DefaultValue::Expr(e)) => *e,
                _ => "",
            };
            p.values.iter().find(|v| !v.eq_ignore_ascii_case(default)).or(p.values.first()).map_or("0".into(), |v| v.to_string())
        }
        Type::Color => "&H00FF00".into(),
        Type::Component => p.kinds.first().and_then(|k| resolve_component(k)).map_or("Other1".into(), |c| c.instance_name()),
        Type::Font => "Font1".into(),
        Type::Picture => "\"picture.bmp\"".into(),
        Type::Resource | Type::Item | Type::Any => "0".into(),
    }
}

/// How a program uses property `p` of `c`: `Button1.Caption = "OK"`, or
/// `PRINT Button1.Handle` for a read-only one; indexed ones with their
/// index (`Grid1.Cell(0, 0) = "text"`).
pub fn property_example(c: &Component, p: &Property) -> String {
    let index = match p.indexed {
        1 => "(0)",
        2 => "(0, 0)",
        _ => "",
    };
    let target = format!("{}.{}{index}", c.instance_name(), p.name);
    if p.writable() {
        format!("{target} = {}", example_value(p))
    } else {
        format!("PRINT {target}")
    }
}

/// How a program calls method `m` of `c`: `Button1.SetFocus`,
/// `List1.AddItems "text"`, `n = List1.IndexOf("text")`. A hand-written
/// `example` wins.
pub fn method_example(c: &Component, m: &Method) -> String {
    if !m.example.is_empty() {
        return m.example.to_string();
    }
    let args: Vec<String> = m.params.iter().filter(|p| !p.optional).map(example_arg).collect();
    let call = format!("{}.{}", c.instance_name(), m.name);
    match (m.returns, args.is_empty()) {
        (Some(_), true) => format!("result = {call}"),
        (Some(_), false) => format!("result = {call}({})", args.join(", ")),
        (None, true) => call,
        (None, false) => format!("{call} {}", args.join(", ")),
    }
}

/// A handler for event `e` of `c` and its binding, as RapidR Studio writes
/// them: `SUB Button1Click` … `END SUB` and `OnClick = Button1Click`.
pub fn event_example(c: &Component, e: &Event) -> String {
    let sub = format!("{}{}", c.instance_name(), e.name.strip_prefix("On").unwrap_or(e.name));
    let params = e.params.iter().map(|p| p.text()).collect::<Vec<_>>().join(", ");
    let header = if params.is_empty() { format!("SUB {sub}") } else { format!("SUB {sub} ({params})") };
    format!("{header}\n    ' …\nEND SUB\n' in {}'s CREATE block: {} = {sub}", c.instance_name(), e.name)
}

// --- the completeness checker (docs/ai-architecture.md §2.6) ----------------------

/// A facet an entry may lack.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Facet {
    /// What it is and does (`doc`).
    Semantics,
    /// Types of its value, parameters and result.
    Types,
    /// A number's range.
    Range,
    /// What it does beyond returning a value.
    Effects,
    Example,
}

impl Facet {
    pub const ALL: [Facet; 5] = [Facet::Semantics, Facet::Types, Facet::Range, Facet::Effects, Facet::Example];

    pub fn as_str(self) -> &'static str {
        match self {
            Facet::Semantics => "semantics",
            Facet::Types => "types",
            Facet::Range => "range",
            Facet::Effects => "effects",
            Facet::Example => "example",
        }
    }
}

/// What a checked entry is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EntryKind {
    /// A component the compilers create, or a BASIC library RapidR supplies.
    Component,
    /// A global object (Screen) or an item object (TreeNode).
    Object,
    Property,
    Method,
    Event,
    Builtin,
    /// A statement or a directive.
    Statement,
}

impl EntryKind {
    pub const ALL: [EntryKind; 7] = [EntryKind::Component, EntryKind::Object, EntryKind::Property, EntryKind::Method, EntryKind::Event, EntryKind::Builtin, EntryKind::Statement];

    pub fn as_str(self) -> &'static str {
        match self {
            EntryKind::Component => "component",
            EntryKind::Object => "object",
            EntryKind::Property => "property",
            EntryKind::Method => "method",
            EntryKind::Event => "event",
            EntryKind::Builtin => "builtin",
            EntryKind::Statement => "statement",
        }
    }

    fn plural(self) -> &'static str {
        match self {
            EntryKind::Component => "components",
            EntryKind::Object => "objects",
            EntryKind::Property => "properties",
            EntryKind::Method => "methods",
            EntryKind::Event => "events",
            EntryKind::Builtin => "builtins",
            EntryKind::Statement => "statements",
        }
    }
}

/// One missing facet of one entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gap {
    /// `RButton`, `RButton.Left`, `RButton.SetFocus()`, `RButton.OnClick`,
    /// `MID$`, `PRINT`.
    pub entry: String,
    pub kind: EntryKind,
    pub facet: Facet,
    /// What to add, in a line.
    pub why: String,
}

/// How many entries of a kind were checked and how many have every facet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tally {
    pub entries: usize,
    pub complete: usize,
}

/// What [`check`] found.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub gaps: Vec<Gap>,
    pub tally: BTreeMap<EntryKind, Tally>,
}

impl Report {
    fn entry(&mut self, kind: EntryKind, name: &str, gaps: Vec<(Facet, String)>) {
        let t = self.tally.entry(kind).or_default();
        t.entries += 1;
        if gaps.is_empty() {
            t.complete += 1;
        }
        self.gaps.extend(gaps.into_iter().map(|(facet, why)| Gap { entry: name.to_string(), kind, facet, why }));
    }

    /// Every checked entry has every facet.
    pub fn is_complete(&self) -> bool {
        self.gaps.is_empty()
    }

    pub fn entries(&self) -> usize {
        self.tally.values().map(|t| t.entries).sum()
    }

    pub fn complete(&self) -> usize {
        self.tally.values().map(|t| t.complete).sum()
    }

    /// Gaps per facet.
    pub fn by_facet(&self) -> BTreeMap<Facet, usize> {
        let mut m = BTreeMap::new();
        for g in &self.gaps {
            *m.entry(g.facet).or_insert(0) += 1;
        }
        m
    }

    /// The gaps of one entry.
    pub fn gaps_of(&self, entry: &str) -> Vec<&Gap> {
        self.gaps.iter().filter(|g| g.entry.eq_ignore_ascii_case(entry)).collect()
    }

    /// The counts, as `rapidr lang ai-check --summary` prints them.
    pub fn summary(&self) -> String {
        let mut out = String::from("AI readiness of the language registry (docs/ai-architecture.md §2.6)\n\n");
        let _ = writeln!(out, "{:<12} {:>8} {:>9} {:>7}", "", "entries", "complete", "ready");
        let pct = |c: usize, e: usize| if e == 0 { 100.0 } else { c as f64 * 100.0 / e as f64 };
        for kind in EntryKind::ALL {
            let t = self.tally.get(&kind).copied().unwrap_or_default();
            if t.entries == 0 {
                continue;
            }
            let _ = writeln!(out, "{:<12} {:>8} {:>9} {:>6.1}%", kind.plural(), t.entries, t.complete, pct(t.complete, t.entries));
        }
        let _ = writeln!(out, "{:<12} {:>8} {:>9} {:>6.1}%", "total", self.entries(), self.complete(), pct(self.complete(), self.entries()));
        let facets = self.by_facet();
        let list = Facet::ALL.iter().map(|f| format!("{} {}", f.as_str(), facets.get(f).copied().unwrap_or(0))).collect::<Vec<_>>().join(", ");
        let _ = writeln!(out, "\n{} gaps by facet: {list}.", self.gaps.len());
        out
    }

    /// Every gap, a line each (entry, facet, what to add), then the summary.
    pub fn text(&self) -> String {
        let width = self.gaps.iter().map(|g| g.entry.chars().count()).max().unwrap_or(0).min(48);
        let mut out = String::new();
        for g in &self.gaps {
            let _ = writeln!(out, "{:<width$}  {:<9}  {}", g.entry, g.facet.as_str(), g.why);
        }
        if !self.gaps.is_empty() {
            out.push('\n');
        }
        out.push_str(&self.summary());
        out
    }

    /// The report as JSON: counts per kind and facet, and every gap.
    pub fn json(&self) -> String {
        let kinds = EntryKind::ALL
            .iter()
            .filter_map(|k| self.tally.get(k).map(|t| format!("{}:{{\"entries\":{},\"complete\":{}}}", json_str(k.plural()), t.entries, t.complete)))
            .collect::<Vec<_>>()
            .join(",");
        let facets = self.by_facet();
        let facets = Facet::ALL.iter().map(|f| format!("{}:{}", json_str(f.as_str()), facets.get(f).copied().unwrap_or(0))).collect::<Vec<_>>().join(",");
        let gaps = self
            .gaps
            .iter()
            .map(|g| format!("{{\"entry\":{},\"kind\":{},\"facet\":{},\"why\":{}}}", json_str(&g.entry), json_str(g.kind.as_str()), json_str(g.facet.as_str()), json_str(&g.why)))
            .collect::<Vec<_>>()
            .join(",\n");
        format!(
            "{{\"ai_check\":1,\"complete\":{},\"entries\":{},\"ready\":{},\"kinds\":{{{kinds}}},\"facets\":{{{facets}}},\"gaps\":[\n{gaps}\n]}}\n",
            self.is_complete(),
            self.entries(),
            self.complete()
        )
    }
}

fn untyped(params: &[Param]) -> Vec<&'static str> {
    params.iter().filter(|p| p.ty.is_empty()).map(|p| p.name).collect()
}

/// Checks one component (or global / item object) and its members.
pub fn check_component(c: &Component, report: &mut Report) {
    if c.kind == Kind::Planned {
        return;
    }
    let name = c.spelling();
    let object = matches!(c.kind, Kind::Global | Kind::Item);
    let mut gaps = Vec::new();
    if c.doc.trim().is_empty() {
        gaps.push((Facet::Semantics, "what it is and does: `doc`".to_string()));
    }
    if !object && c.example.trim().is_empty() {
        gaps.push((Facet::Example, "a few lines that use it: `example` (a CREATE block, its default event wired)".to_string()));
    }
    report.entry(if object { EntryKind::Object } else { EntryKind::Component }, &name, gaps);

    for p in c.properties.iter().filter(|p| !p.missing) {
        let mut gaps = Vec::new();
        if p.doc.trim().is_empty() {
            gaps.push((Facet::Semantics, "what it means: `doc`".to_string()));
        }
        match p.ty {
            Type::Any => gaps.push((Facet::Types, "its type is `any`: say the type".to_string())),
            Type::Component if p.kinds.is_empty() => gaps.push((Facet::Types, "a component property names the types it takes: `kinds`".to_string())),
            _ => {}
        }
        if matches!(p.ty, Type::Int | Type::Float) && p.writable() && p.range.is_none() {
            gaps.push((Facet::Range, format!("a writable {}: its range (`range = \"0..\"`, `\"Min..Max\"` or `\"any\"`), here or in the properties glossary", p.ty.as_str())));
        }
        report.entry(EntryKind::Property, &format!("{name}.{}", p.name), gaps);
    }
    for m in c.methods.iter().filter(|m| !m.missing) {
        let mut gaps = Vec::new();
        if m.doc.trim().is_empty() {
            gaps.push((Facet::Semantics, "what it does: `doc`".to_string()));
        }
        let untyped = untyped(m.params);
        if !untyped.is_empty() {
            gaps.push((Facet::Types, format!("parameters without a type: {}", untyped.join(", "))));
        }
        // (`returns` can't be checked mechanically — a method that gives
        // nothing has none, and `value` is a parsing rule, not a result:
        // the pass that says a method's effects says its result too)
        if m.effects.is_empty() {
            gaps.push((Facet::Effects, "what calling it does: `effects = [\"ui\"]` (none, ui, wait, state, file-read, file-write, network …)".to_string()));
        }
        report.entry(EntryKind::Method, &format!("{name}.{}()", m.name), gaps);
    }
    for e in c.events.iter().filter(|e| !e.missing) {
        let mut gaps = Vec::new();
        if e.doc.trim().is_empty() {
            gaps.push((Facet::Semantics, "when it fires: `doc`".to_string()));
        }
        let untyped = untyped(e.params);
        if !untyped.is_empty() {
            gaps.push((Facet::Types, format!("parameters without a type: {}", untyped.join(", "))));
        }
        report.entry(EntryKind::Event, &format!("{name}.{}", e.name), gaps);
    }
}

/// Checks one builtin.
pub fn check_builtin(b: &Builtin, report: &mut Report) {
    if b.missing {
        return;
    }
    let mut gaps = Vec::new();
    if b.doc.trim().is_empty() {
        gaps.push((Facet::Semantics, "what it does: `doc`".to_string()));
    }
    if b.takes_arguments() {
        let untyped = untyped(b.params);
        if b.params.is_empty() {
            gaps.push((Facet::Types, "its parameters with their types: `params` (only the syntax line says them)".to_string()));
        } else if !untyped.is_empty() {
            gaps.push((Facet::Types, format!("parameters without a type: {}", untyped.join(", "))));
        }
    }
    if b.effects.is_empty() {
        gaps.push((Facet::Effects, "what calling it does: `effects = [\"none\"]` (none, state, file-read, file-write, process …)".to_string()));
    }
    if b.example.trim().is_empty() {
        gaps.push((Facet::Example, "a call and what it gives: `example`".to_string()));
    }
    report.entry(EntryKind::Builtin, b.name, gaps);
}

fn check_statement(name: &str, syntax: &str, doc: &str, report: &mut Report) {
    let mut gaps = Vec::new();
    if doc.trim().is_empty() {
        gaps.push((Facet::Semantics, "what it does: `doc`".to_string()));
    }
    if syntax.trim().is_empty() {
        gaps.push((Facet::Types, "how it's written: `syntax`".to_string()));
    }
    report.entry(EntryKind::Statement, name, gaps);
}

/// Checks the whole registry: every component (not the planned ones), global
/// and item object, member (not the missing ones), builtin (not the missing
/// or internal ones), statement and directive.
pub fn check() -> Report {
    let mut report = Report::default();
    for c in COMPONENTS.iter().chain(GLOBALS).chain(ITEMS) {
        check_component(c, &mut report);
    }
    for b in BUILTINS {
        check_builtin(b, &mut report);
    }
    for s in STATEMENTS {
        check_statement(s.name, s.syntax, s.doc, &mut report);
    }
    for d in DIRECTIVES {
        check_statement(d.name, d.syntax, d.doc, &mut report);
    }
    report
}

/// Checks the entries `names` name: components (RapidR's or RapidQ's names),
/// global or item objects, builtins, statements or directives. Fails with
/// the names that are none of them.
pub fn check_names(names: &[&str]) -> Result<Report, String> {
    let mut report = Report::default();
    let mut unknown = Vec::new();
    for n in names {
        if let Some(c) = resolve_component(n).or_else(|| global(n)).or_else(|| item(n)) {
            check_component(c, &mut report);
        } else if let Some(b) = builtin(n) {
            check_builtin(b, &mut report);
        } else if let Some(s) = statement(n) {
            check_statement(s.name, s.syntax, s.doc, &mut report);
        } else if let Some(d) = directive(n) {
            check_statement(d.name, d.syntax, d.doc, &mut report);
        } else {
            unknown.push(*n);
        }
    }
    if unknown.is_empty() {
        Ok(report)
    } else {
        Err(format!("not in the registry: {}", unknown.join(", ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_vocabularies_line_up() {
        assert_eq!(EFFECT_NAMES.len(), Effect::ALL.len());
        assert_eq!(EFFECT_RISK.len(), Effect::ALL.len());
        assert_eq!(RISK_NAMES.len(), Risk::ALL.len());
        for (i, e) in Effect::ALL.iter().enumerate() {
            assert_eq!(*e as usize, i);
            assert_eq!(Effect::parse(e.as_str()), Some(*e));
        }
        for r in Risk::ALL {
            assert_eq!(Risk::parse(r.as_str()), Some(r));
        }
        assert_eq!(Effect::FileWrite.as_str(), "file-write");
        assert_eq!(Effect::None.risk(), Risk::Read);
        assert_eq!(Effect::Wait.risk(), Risk::Ui);
        assert_eq!(Effect::FileRead.risk(), Risk::External);
        assert_eq!(Effect::Printer.risk(), Risk::Irreversible);
        assert_eq!(Effect::Native.risk(), Risk::System);
        assert!(Effect::Network.is_capability() && !Effect::State.is_capability());
        assert_eq!(risk_of(&[Effect::Ui, Effect::FileWrite, Effect::State]), Some(Risk::Irreversible));
        assert_eq!(risk_of(&[]), None);
    }

    #[test]
    fn derivations() {
        let b = component("QBUTTON").unwrap();
        let caption = b.property("Caption").unwrap();
        assert_eq!(caption.set_effects(b), &[Effect::Ui]);
        let timer = component("RTIMER").unwrap();
        assert_eq!(timer.property("Interval").unwrap().set_effects(timer), &[Effect::State]);
        assert_eq!(b.method("SetFocus").unwrap().risk(), Some(Risk::Ui));
        assert!(b.capabilities().is_empty());
        assert_eq!(b.risk(), Some(Risk::Ui));
        assert_eq!(builtin("KILL").unwrap().risk(), Some(Risk::Irreversible));
        assert_eq!(builtin("SHELL").unwrap().risk(), Some(Risk::System));
        assert_eq!(builtin("MID$").unwrap().risk(), Some(Risk::Read));
        assert!(builtin("MID$").unwrap().takes_arguments() && !builtin("TIMER").unwrap().takes_arguments());
    }

    #[test]
    fn examples_are_made_from_the_registry() {
        let b = component("RBUTTON").unwrap();
        assert_eq!(property_example(b, b.property("Caption").unwrap()), "Button1.Caption = \"text\"");
        assert_eq!(property_example(b, b.property("Handle").unwrap()), "PRINT Button1.Handle");
        assert_eq!(property_example(b, b.property("Visible").unwrap()), "Button1.Visible = 0");
        assert_eq!(property_example(b, b.property("Parent").unwrap()), "Button1.Parent = Form1");
        assert_eq!(method_example(b, b.method("SetFocus").unwrap()), "Button1.SetFocus");
        let e = component("REDIT").unwrap();
        assert_eq!(event_example(e, e.event("OnKeyDown").unwrap()).lines().next(), Some("SUB Edit1KeyDown (Key AS WORD, Shift AS INTEGER)"));
        assert!(event_example(b, b.event("OnClick").unwrap()).ends_with("OnClick = Button1Click"));
        assert_eq!(global("Screen").unwrap().instance_name(), "Screen");
    }

    #[test]
    fn the_reference_entries_are_complete() {
        // (phase 0's examples of the format: they stay complete)
        let r = check_names(&["RButton", "MID$", "KILL", "SHELL"]).unwrap();
        assert!(r.is_complete(), "{}", r.text());
        assert_eq!(r.tally[&EntryKind::Component], Tally { entries: 1, complete: 1 });
        assert_eq!(r.tally[&EntryKind::Builtin], Tally { entries: 3, complete: 3 });
        // a Q name is the same entry
        assert_eq!(check_names(&["QBUTTON"]).unwrap(), check_names(&["RButton"]).unwrap());
        assert!(check_names(&["QNOTHING"]).is_err());
    }

    #[test]
    fn the_whole_registry_is_checked() {
        let r = check();
        // every created component, object and builtin is counted once
        let components = COMPONENTS.iter().filter(|c| c.kind != Kind::Planned).count();
        assert_eq!(r.tally[&EntryKind::Component].entries, components);
        assert_eq!(r.tally[&EntryKind::Object].entries, GLOBALS.len() + ITEMS.len());
        assert_eq!(r.tally[&EntryKind::Builtin].entries, BUILTINS.iter().filter(|b| !b.missing).count());
        assert_eq!(r.tally[&EntryKind::Statement].entries, STATEMENTS.len() + DIRECTIVES.len());
        let members: usize = COMPONENTS
            .iter()
            .chain(GLOBALS)
            .chain(ITEMS)
            .filter(|c| c.kind != Kind::Planned)
            .map(|c| c.properties.iter().filter(|p| !p.missing).count() + c.methods.iter().filter(|m| !m.missing).count() + c.events.iter().filter(|e| !e.missing).count())
            .sum();
        assert_eq!(r.tally[&EntryKind::Property].entries + r.tally[&EntryKind::Method].entries + r.tally[&EntryKind::Event].entries, members);
        // the counts add up, every gap names a checked entry
        assert_eq!(r.by_facet().values().sum::<usize>(), r.gaps.len());
        assert!(r.complete() <= r.entries());
        // docs are complete already (tests/coverage.rs: everything_has_docs)
        assert_eq!(r.by_facet().get(&Facet::Semantics), None, "{}", r.text());
        // the outputs say the same
        assert!(r.json().starts_with("{\"ai_check\":1,\"complete\":false"));
        assert!(r.summary().contains("total"));
        assert_eq!(r.text().lines().filter(|l| l.contains("  effects  ")).count(), r.by_facet().get(&Facet::Effects).copied().unwrap_or(0));
    }
}
