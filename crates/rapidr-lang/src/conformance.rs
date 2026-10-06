//! The conformance programs that tie the registry to the runtimes: one per
//! component, made from its registry entry, which every runtime (native
//! builds, the interpreter, the web) runs (tests/lang_conformance.mjs).
//!
//! A program creates the component (on a form when it's visual), then for
//! each member the registry says the runtime answers:
//!
//! - reads each readable property — printing its value where the registry
//!   gives a default (`Caption=`), else just that it read it;
//! - calls each method (arguments made from its parameters: 0, "", …;
//!   `test = "skip: …"` ones aren't called, `test = "args: …"` ones with
//!   those arguments);
//! - binds a handler (with the event's parameters) to each event.
//!
//! The expected output is the registry's, except where a runtime is known to
//! differ (tests/lang/gaps.txt: `RBUTTON.Cursor =` the value it reads, a
//! RapidQ default RapidR doesn't give yet). The runner also fails on any
//! "not implemented" / "no such method" warning: every member the registry
//! lists must be answered.

use crate::*;
use std::collections::HashMap;
use std::fmt::Write as _;

/// Which runtime a program is for: members only the other one has are left
/// out.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    Desktop,
    Web,
}

impl Target {
    fn runs(self, r: Runtimes) -> bool {
        matches!((self, r), (_, Runtimes::All) | (Target::Desktop, Runtimes::Desktop) | (Target::Web, Runtimes::Web))
    }

    pub fn name(self) -> &'static str {
        match self {
            Target::Desktop => "desktop",
            Target::Web => "web",
        }
    }
}

/// One component's program and the output it must print.
pub struct Program {
    pub component: &'static str,
    pub source: String,
    pub expected: String,
}

/// Known differences: `COMPONENT.Member = value` (every runtime) or
/// `web: COMPONENT.Member = value` / `desktop: …` (one runtime), `#`
/// comments. The value is what PRINT shows (nothing for an empty one).
#[derive(Default)]
pub struct Gaps {
    map: HashMap<(Option<Target>, String), String>,
}

impl Gaps {
    pub fn parse(text: &str) -> Result<Gaps, String> {
        let mut g = Gaps::default();
        for (n, line) in text.lines().enumerate() {
            let line = line.trim_end();
            if line.trim().is_empty() || line.trim_start().starts_with('#') {
                continue;
            }
            let (target, rest) = match line.split_once(": ") {
                Some(("web", r)) => (Some(Target::Web), r),
                Some(("desktop", r)) => (Some(Target::Desktop), r),
                _ => (None, line),
            };
            let (member, value) = rest.split_once(" =").ok_or_else(|| format!("gaps line {}: `COMPONENT.Member = value`", n + 1))?;
            g.map.insert((target, member.trim().to_ascii_uppercase()), value.strip_prefix(' ').unwrap_or(value).to_string());
        }
        Ok(g)
    }

    fn get(&self, target: Target, member: &str) -> Option<&str> {
        let key = member.to_ascii_uppercase();
        self.map.get(&(Some(target), key.clone())).or_else(|| self.map.get(&(None, key))).map(String::as_str)
    }
}

/// What PRINT shows for a property's default, if it can be known.
pub fn printed_default(p: &Property) -> Option<String> {
    let d = p.default.as_ref()?;
    Some(match d {
        DefaultValue::Int(i) => i.to_string(),
        // (RapidQ reads a Boolean as 1 or 0)
        DefaultValue::Bool(b) => (if *b { "1" } else { "0" }).into(),
        DefaultValue::Float(f) => format!("{f}"),
        DefaultValue::Str(s) => s.to_string(),
        DefaultValue::Expr(e) => eval_constant(e)?.to_string(),
    })
}

/// A BASIC literal for an argument of type `ty`.
fn dummy(p: &Param) -> Option<&'static str> {
    let ty = p.ty.to_ascii_uppercase();
    match ty.as_str() {
        "" | "INTEGER" | "LONG" | "SHORT" | "BYTE" | "WORD" | "DWORD" | "SINGLE" | "DOUBLE" | "INT64" | "BOOLEAN" | "VARIANT" => {
            Some(if p.name.ends_with('$') { "\"\"" } else { "0" })
        }
        "STRING" => Some("\"\""),
        // (an object argument: none to give)
        _ => None,
    }
}

fn ident(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_ascii_lowercase()
}

/// The program for component `c` on `target` (None when that runtime
/// doesn't have the component).
pub fn program(c: &Component, target: Target, gaps: &Gaps) -> Option<Program> {
    if c.kind != Kind::Component || !target.runs(c.runtimes) {
        return None;
    }
    let p = ident(c.name);
    // (a component RapidQ has one global instance of — Printer — is that)
    let global = GLOBALS.iter().find(|g| g.instance_of.is_some_and(|i| i.eq_ignore_ascii_case(c.name)));
    let obj = global.map_or_else(|| format!("{p}_o"), |g| g.name.to_string());
    let mut src = String::new();
    let mut body = String::new();
    let mut expected = String::new();
    let _ = writeln!(src, "' The language registry's conformance program for {} (`rapidr lang conformance`): generated, don't edit.", c.name);
    // Handlers, one per event (its parameters as the registry has them).
    let mut handlers = String::new();
    for (i, e) in c.events.iter().enumerate().filter(|(_, e)| !e.missing && target.runs(e.runtimes)) {
        let params = e
            .params
            .iter()
            .enumerate()
            .map(|(j, p)| {
                let ty = if p.ty.is_empty() || p.ty.starts_with('Q') { String::new() } else { format!(" AS {}", p.ty) };
                format!("{}a{j}{ty}", if p.byref { "BYREF " } else { "" })
            })
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(handlers, "SUB {p}_e{i}({params})\nEND SUB");
    }
    src.push_str(&handlers);
    // The component: on a form when it's visual (a form on its own).
    let written = c.written_name();
    if global.is_some() {
        // (there already)
    } else if c.name == "RFORM" || c.name == "RFORMMDI" {
        let _ = writeln!(src, "CREATE {obj} AS {written}\nEND CREATE");
    } else if c.visual {
        let _ = writeln!(src, "CREATE {p}_f AS QFORM\n  CREATE {obj} AS {written}\n  END CREATE\nEND CREATE");
    } else {
        let _ = writeln!(src, "DIM {obj} AS {written}");
    }
    let _ = writeln!(src, "DIM {p}_v AS VARIANT");
    let mut line = |code: String, out: String| {
        body.push_str(&code);
        body.push('\n');
        expected.push_str(&out);
        expected.push('\n');
    };
    line(format!("PRINT \"== {}\"", c.name), format!("== {}", c.name));
    for pr in c.properties.iter().filter(|x| !x.missing && x.access != Access::Write && x.indexed == 0 && target.runs(x.runtimes)) {
        let member = format!("{}.{}", c.name, pr.name);
        match gaps.get(target, &member).map(str::to_string).or_else(|| printed_default(pr)) {
            Some(v) if !matches!(pr.ty, Type::Font | Type::Component | Type::Resource) => {
                line(format!("PRINT \"{}=\"; {obj}.{}", pr.name, pr.name), format!("{}={v}", pr.name));
            }
            _ => line(format!("{p}_v = {obj}.{}: PRINT \"{} read\"", pr.name, pr.name), format!("{} read", pr.name)),
        }
    }
    for m in c.methods.iter().filter(|x| !x.missing && target.runs(x.runtimes)) {
        let args = match m.test {
            Test::Skip(_) => continue,
            Test::Args(a) => Some(a.to_string()),
            Test::Call => m.params.iter().filter(|x| !x.optional).map(dummy).collect::<Option<Vec<_>>>().map(|v| v.join(", ")),
        };
        let Some(args) = args else { continue };
        let call = if args.is_empty() { format!("{obj}.{}", m.name) } else { format!("{obj}.{} {args}", m.name) };
        line(format!("{call}: PRINT \"{} called\"", m.name), format!("{} called", m.name));
    }
    for (i, e) in c.events.iter().enumerate().filter(|(_, e)| !e.missing && target.runs(e.runtimes)) {
        line(format!("{obj}.{} = {p}_e{i}: PRINT \"{} bound\"", e.name, e.name), format!("{} bound", e.name));
    }
    src.push_str(&body);
    Some(Program { component: c.name, source: src, expected })
}

/// Every component's program for `target`.
pub fn programs(target: Target, gaps: &Gaps) -> Vec<Program> {
    COMPONENTS.iter().filter_map(|c| program(c, target, gaps)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_button() {
        let p = program(component("RBUTTON").unwrap(), Target::Desktop, &Gaps::default()).unwrap();
        assert!(p.source.contains("CREATE rbutton_o AS QBUTTON"), "{}", p.source);
        assert!(p.expected.starts_with("== RBUTTON\n"));
        assert!(p.expected.contains("Left=0\n"));
        assert!(p.expected.contains("OnClick bound\n"));
    }

    #[test]
    fn gaps_override_defaults() {
        let g = Gaps::parse("# x\nRBUTTON.Cursor =\nweb: RBUTTON.Left = 7\n").unwrap();
        let d = program(component("RBUTTON").unwrap(), Target::Desktop, &g).unwrap();
        assert!(d.expected.contains("Cursor=\n") && d.expected.contains("Left=0\n"));
        let w = program(component("RBUTTON").unwrap(), Target::Web, &g).unwrap();
        assert!(w.expected.contains("Left=7\n"));
    }

    #[test]
    fn web_components_only_on_the_web() {
        assert!(program(component("RWEBVIEW").unwrap(), Target::Desktop, &Gaps::default()).is_none());
        assert!(program(component("RWEBVIEW").unwrap(), Target::Web, &Gaps::default()).is_some());
    }
}
