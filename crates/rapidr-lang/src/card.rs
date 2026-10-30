//! The language card (docs/preview4-plan.md K1): RapidR in about 2,000
//! tokens, the rules AI models actually get wrong — sent with every RAI
//! chat, so even a small model writes RapidR that compiles.
//!
//! Nothing in it is written by hand where the registry has it: the rules
//! are the registry's (`[[rule]]` in data/language.toml, each from the
//! mistake ledger, docs/rai-mistake-ledger.md; their programs compiled and
//! run by `rapidr lang truth`), the FOR line is the FOR statement's syntax,
//! the types are the registry's type names, the event SUBs' parameters are
//! each event's most common signature across the components, the names are
//! the components'. A test fails when the card names something the
//! registry doesn't have, or grows past [`BUDGET_TOKENS`].

use crate::*;
use std::collections::HashMap;
use std::fmt::Write as _;

/// The card's budget, in tokens ([`estimate_tokens`]).
pub const BUDGET_TOKENS: usize = 2_000;

/// The events whose SUBs the card shows (their parameters are the
/// registry's): what programs bind most.
pub const EVENTS: &[&str] = &["OnClick", "OnChange", "OnTimer", "OnClose", "OnResize", "OnPaint", "OnKeyPress", "OnKeyDown", "OnMouseDown", "OnMouseMove"];

/// Words in the card that look like names but aren't the language's: prose
/// (BASIC, IDE) and what the card says RapidR doesn't have (BOOLEAN). A test
/// keeps each one out of the registry.
pub const NOT_NAMES: &[&str] = &["BASIC", "IDE", "BOOLEAN"];

/// RapidQ names whose RapidR names the card shows as examples.
const NAME_EXAMPLES: &[&str] = &["QFORM", "QBUTTON", "QEDIT"];

/// A text's size in tokens, estimated as model tokenizers count English
/// and code: about one token per 3.5 characters (a slight overestimate for
/// prose, close for code). The live evals print the provider's exact count.
pub fn estimate_tokens(text: &str) -> usize {
    (text.chars().count() * 2).div_ceil(7)
}

/// An event's parameters as most components have them (`None`: no
/// component has the event).
pub fn event_params(event: &str) -> Option<String> {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for c in COMPONENTS.iter().filter(|c| c.kind == Kind::Component) {
        for e in c.events.iter().filter(|e| e.name.eq_ignore_ascii_case(event) && !e.missing) {
            let sig = params_text(e.params);
            match counts.iter_mut().find(|(s, _)| *s == sig) {
                Some((_, n)) => *n += 1,
                None => counts.push((sig, 1)),
            }
        }
    }
    // (the most common; on a tie the first seen, in the registry's order)
    let mut best: Option<(String, usize)> = None;
    for (sig, n) in counts {
        if best.as_ref().is_none_or(|(_, b)| n > *b) {
            best = Some((sig, n));
        }
    }
    best.map(|(s, _)| s)
}

/// A rule's program in one line when it fits (statements joined by `: `),
/// else `None`.
fn one_line(code: &str) -> Option<String> {
    let lines: Vec<&str> = code.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let blocky = |l: &str| {
        let up = l.to_ascii_uppercase();
        ["SUB ", "FUNCTION ", "END ", "CREATE ", "TYPE ", "DECLARE "].iter().any(|k| up.starts_with(k)) || (up.starts_with("IF ") && up.ends_with("THEN"))
    };
    (lines.len() <= 4 && !lines.iter().any(|l| blocky(l))).then(|| lines.join(": "))
}

/// The card.
pub fn card() -> String {
    let mut out = String::new();
    out.push_str("# RapidR: the rules to get right\n\n");
    out.push_str("RapidR is a modern BASIC with its own IDE, RapidR Studio; it also runs older programs. Names are case-insensitive. What follows is checked by RapidR's compiler.\n\n");
    for r in RULES {
        let _ = write!(out, "- {}", r.text);
        // (a rule about a statement — FOR, CREATE —: its syntax, the registry's)
        if let Some(s) = statement(r.statement).filter(|_| !r.statement.is_empty()) {
            let _ = write!(out, " Syntax: `{}`.", s.syntax);
        }
        if !r.wrong.is_empty() {
            let _ = write!(out, " Not `{}`.", r.wrong);
        }
        if !r.right.is_empty() {
            match one_line(r.right) {
                Some(line) => {
                    let _ = write!(out, " E.g. `{line}`");
                }
                None => {
                    let _ = write!(out, "\n```basic\n{}\n```", r.right.trim_end());
                }
            }
        }
        out.push('\n');
    }
    out.push_str("\n## Types\n");
    let types: Vec<&str> = TYPE_NAMES.iter().filter(|t| !matches!(t.name, "QOBJECT" | "ROBJECT")).map(|t| t.name).collect();
    let _ = writeln!(out, "{}; `STRING * n` holds n characters; a component type (RButton …) or a TYPE of the program. No BOOLEAN.", types.join(", "));
    out.push_str("\n## Event SUBs (bound in the CREATE block: `OnClick = SubName`)\n");
    for e in EVENTS {
        if let Some(p) = event_params(e) {
            let p = if p.is_empty() { String::new() } else { format!(" {p}") };
            let _ = writeln!(out, "- {e}: `SUB Name{p}`");
        }
    }
    out.push_str("The component that fired it may come last, `Sender AS RButton` (its type): one SUB for many buttons. A few components' events differ: the reference has each one's.\n");
    out.push_str("\n## RapidR's names\n");
    let examples: Vec<String> = NAME_EXAMPLES.iter().filter_map(|q| component(q).map(|c| format!("{q} is {}", c.spelling()))).collect();
    let _ = writeln!(out, "Write components with RapidR's names, R for the classic Q: {}. The classic names are the same components: keep the names a file already uses.", examples.join(", "));
    out
}

/// The names in the card that look like the language's: ALL-CAPS words
/// (statements, keywords, types, builtins, constants), `R…` component
/// names and `On…` events — for the test that each is the registry's.
pub fn names_in(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut word = String::new();
    let flush = |w: &mut String, out: &mut Vec<String>| {
        if !w.is_empty() {
            out.push(std::mem::take(w));
        }
    };
    for c in text.chars() {
        if c.is_ascii_alphanumeric() || c == '_' || c == '$' {
            word.push(c);
        } else {
            flush(&mut word, &mut out);
        }
    }
    flush(&mut word, &mut out);
    let mut seen: HashMap<String, ()> = HashMap::new();
    out.into_iter()
        .filter(|w| {
            let caps = w.len() > 1 && w.chars().all(|c| c.is_ascii_uppercase() || c == '$' || c.is_ascii_digit()) && w.chars().next().is_some_and(|c| c.is_ascii_uppercase());
            let r_name = w.len() > 2 && w.starts_with('R') && w.chars().nth(1).is_some_and(|c| c.is_ascii_uppercase()) && w.chars().any(|c| c.is_ascii_lowercase());
            let event = w.len() > 2 && w.starts_with("On") && w.chars().nth(2).is_some_and(|c| c.is_ascii_uppercase());
            caps || r_name || event
        })
        .filter(|w| seen.insert(w.clone(), ()).is_none())
        .collect()
}

/// Whether `name` (as [`names_in`] finds it) is the registry's.
pub fn is_known(name: &str) -> bool {
    let n = name.trim_end_matches('$');
    if name.starts_with("On") && name.chars().nth(2).is_some_and(|c| c.is_ascii_uppercase()) {
        return COMPONENTS.iter().chain(GLOBALS).any(|c| c.events.iter().any(|e| e.name.eq_ignore_ascii_case(name)));
    }
    if name.starts_with('R') && name.chars().any(|c| c.is_ascii_lowercase()) {
        return component(name).is_some();
    }
    // (a statement's words: END, CASE, SELECT …; THEN, NEXT: keywords)
    let statement_word = STATEMENTS.iter().any(|s| s.name.split(' ').any(|w| w.eq_ignore_ascii_case(n)) || s.syntax.split(|c: char| !c.is_ascii_alphanumeric()).any(|w| w == n));
    statement_word
        || keyword(n).is_some()
        || type_name(n).is_some()
        || builtin(name).is_some()
        || directive(n).is_some()
        || component(name).is_some()
        || constant(n).is_some()
        || matches!(n, "TRUE" | "FALSE")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_card_is_under_budget() {
        let c = card();
        let tokens = estimate_tokens(&c);
        assert!(tokens <= BUDGET_TOKENS, "the card is about {tokens} tokens, over its budget of {BUDGET_TOKENS}:\n{c}");
        // (and it says something: every rule, the events, the names)
        assert!(RULES.len() >= 8);
        for r in RULES {
            assert!(c.contains(r.text), "rule {} isn't on the card", r.name);
        }
        assert!(c.contains("- OnClick: `SUB Name`") && c.contains("- OnMouseDown: `SUB Name (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)`"), "{c}");
        for q in NAME_EXAMPLES {
            assert!(component(q).is_some_and(|k| c.contains(&format!("{q} is {}", k.spelling()))), "{q}: not a component the card can name");
        }
        assert!(c.contains("FOR variable [AS type] = start TO end"));
    }

    #[test]
    fn the_card_names_only_what_the_registry_has() {
        let c = card();
        let unknown: Vec<String> = names_in(&c).into_iter().filter(|n| !is_known(n) && !NOT_NAMES.contains(&n.as_str())).collect();
        // (what the card says RapidR doesn't have, it doesn't)
        for n in NOT_NAMES {
            assert!(!is_known(n), "{n} is the registry's now: the card says otherwise");
        }
        assert!(unknown.is_empty(), "the card names what the registry doesn't have: {unknown:?}");
        // (the check itself catches a made-up name)
        assert!(!is_known("NSTR") && !is_known("RTextBox") && !is_known("OnPressed") && !is_known("BOOLEAN"));
        assert!(names_in("Use RTextBox and NSTR$ on OnPressed").len() == 3);
    }

    #[test]
    fn a_rules_statement_is_the_registrys() {
        for r in RULES.iter().filter(|r| !r.statement.is_empty()) {
            let s = statement(r.statement).unwrap_or_else(|| panic!("rule {}: no statement {}", r.name, r.statement));
            assert!(card().contains(&format!("Syntax: `{}`.", s.syntax)));
        }
        assert!(RULES.iter().any(|r| r.statement == "FOR"));
    }

    #[test]
    fn every_one_for_yes_builtin_is_on_the_card() {
        // (the registry's builtins that give 1 for yes: the card names each)
        let c = card();
        for b in BUILTINS.iter().filter(|b| b.doc.starts_with("True (1)")) {
            assert!(c.contains(b.name), "{} gives 1 for yes (its doc) but the card doesn't say so", b.name);
        }
    }

    #[test]
    fn events_have_their_usual_parameters() {
        assert_eq!(event_params("OnClick").as_deref(), Some(""));
        assert_eq!(event_params("OnKeyPress").as_deref(), Some("(Key AS BYTE)"));
        assert!(event_params("OnNothingAtAll").is_none());
        for e in EVENTS {
            assert!(event_params(e).is_some(), "{e}: no component has it");
        }
    }
}
