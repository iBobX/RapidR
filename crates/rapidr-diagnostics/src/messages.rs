//! The compiler's messages in the user's language: a catalog of the
//! messages by stable ID (`RQ…` for the ones that mirror RapidQ's RC.EXE
//! word for word, `RR…` for RapidR's own), each with its English text as
//! a template (`Member {member} not part of class {class}`).
//!
//! The compiler itself always speaks English: its messages, the tests that
//! match them, the markers code looks for in them stay as they are. Only
//! what shows a message to a person translates it ([`localize`]): the
//! message is matched against the templates, its values taken out, and
//! the template's translation filled with them — RapidR Studio's Problems
//! in Studio's language, `rapidr check` / `run` / `build` in
//! `RAPIDR_LANG`'s or the system's ([`cli_language`]). A message no
//! template matches, or one whose translation isn't written yet, stays in
//! English. A message that mirrors RC.EXE keeps its original beside the
//! translation where the CLI prints it ([`localize_report`]), so the words
//! RapidQ's users know and search for are still there.
//!
//! Translations are PO files compiled in (`lang/<language>.po`: `msgctxt`
//! the ID, `msgid` the English template, `msgstr` the translation), the
//! same format as programs' and RapidR Studio's catalogs. The pseudo-
//! language `xx` translates every template (rapidr_i18n::pseudo).

use std::collections::HashMap;
use std::sync::OnceLock;

/// A message of the compiler's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Message {
    /// Its ID, never changed once given (tools and tests may name it).
    pub id: &'static str,
    /// Its English text, `{name}` where a value goes.
    pub en: &'static str,
}

impl Message {
    /// Mirrors RapidQ's RC.EXE (`.reference/rapidq-compiler-messages.txt`).
    pub fn rapidq(&self) -> bool {
        self.id.starts_with("RQ")
    }
}

const fn m(id: &'static str, en: &'static str) -> Message {
    Message { id, en }
}

/// Every message, the more specific of two that could match first. New
/// messages get the next free number; an ID is never reused.
pub const MESSAGES: &[Message] = &[
    // ---- RapidQ's (RC.EXE's words) ----
    m("RQ0001", "Member {member} not part of class {class}"),
    m("RQ0002", "{name} is a read-only value."),
    m("RQ0003", "Property {property} of {class} is read-only."),
    m("RQ0004", "Undeclared identifier {name}"),
    m("RQ0005", "Expected end-of-line but got {got}"),
    m("RQ0006", "Expected {expected} but got \"{got}\""),
    m("RQ0007", "Identifier {name} already used, try another name"),
    m("RQ0008", "Unknown type {name}"),
    m("RQ0009", "ICON file {file} does not exist."),
    m("RQ0010", "Array of {type} is not supported!"),
    m("RQ0011", "Create Method only works for QObjects, not {name}"),
    m("RQ0012", "Datatype {type} not supported in STRUCT"),
    m("RQ0013", "{name} is not a valid data type for your FUNCTION"),
    m("RQ0014", "Component assignment is not yet supported."),
    m("RQ0015", "Too many actual parameters for {name}"),
    m("RQ0016", "Expected end-of-line"),
    m("RQ0017", "Expected {expected}"),
    // ---- RapidR's ----
    m("RR0001", "Syntax error in {word} statement (did you mean {fix}?)"),
    m("RR0002", "Syntax error in {word} statement"),
    m("RR0003", "Syntax error near '{text}'"),
    m("RR0004", "Syntax error: unexpected '{text}'"),
    m("RR0005", "Syntax error: unexpected end of file"),
    m("RR0006", "Unknown SUB or FUNCTION '{name}'"),
    m("RR0007", "Label '{label}' not found in {routine}"),
    m("RR0008", "Label '{label}' is defined more than once"),
    m("RR0009", "EXIT {kind} is not inside a {kind} loop"),
    m("RR0010", "EXIT {kind} is not inside a {kind}"),
    m("RR0011", "'{word}' is a reserved word and can't be used as a name"),
    m("RR0012", "EVENT {name} is missing END EVENT"),
    m("RR0013", "PROPERTY SET {name} is missing END PROPERTY"),
    m("RR0014", "Field {field} in TYPE {type} needs a type: `{field} AS INTEGER`"),
    m("RR0015", "Unexpected '{text}' after field declaration"),
    m("RR0016", "GOSUB needs a label"),
    m("RR0017", "FOR {name} AS {type}: {name} is already declared AS {declared}"),
    m("RR0018", "{file}: no errors"),
];

/// The message with ID `id`.
pub fn by_id(id: &str) -> Option<&'static Message> {
    MESSAGES.iter().find(|m| m.id.eq_ignore_ascii_case(id))
}

// ------------------------------------------------------------ matching --

#[derive(Debug)]
enum Part<'a> {
    Lit(&'a str),
    Var(&'a str),
}

fn parts(template: &str) -> Vec<Part<'_>> {
    let mut out = Vec::new();
    let mut rest = template;
    while let Some(i) = rest.find('{') {
        let Some(j) = rest[i..].find('}') else { break };
        if i > 0 {
            out.push(Part::Lit(&rest[..i]));
        }
        out.push(Part::Var(&rest[i + 1..i + j]));
        rest = &rest[i + j + 1..];
    }
    if !rest.is_empty() {
        out.push(Part::Lit(rest));
    }
    out
}

/// `text` against template parts from `k`: the values taken (in order), and
/// how much of `text` is left over when `prefix` allows that.
fn match_from(parts: &[Part], k: usize, text: &str, values: &mut Vec<(String, String)>, prefix: bool) -> Option<usize> {
    let Some(part) = parts.get(k) else {
        return if text.is_empty() || prefix { Some(text.len()) } else { None };
    };
    match part {
        Part::Lit(l) => {
            let rest = text.strip_prefix(l)?;
            match_from(parts, k + 1, rest, values, prefix)
        }
        Part::Var(name) => {
            // (the same name twice: the same value)
            if let Some((_, v)) = values.iter().find(|(n, _)| n == name).cloned() {
                let rest = text.strip_prefix(v.as_str())?;
                return match_from(parts, k + 1, rest, values, prefix);
            }
            match parts.get(k + 1) {
                // (the last: the rest of the text, or the next word when a
                // longer message may follow)
                None if !prefix => {
                    if text.is_empty() {
                        return None;
                    }
                    values.push((name.to_string(), text.to_string()));
                    Some(0)
                }
                None => {
                    let end = text.find(|c: char| c.is_whitespace() || c == ',' || c == ';').unwrap_or(text.len());
                    if end == 0 {
                        return None;
                    }
                    values.push((name.to_string(), text[..end].to_string()));
                    Some(text.len() - end)
                }
                Some(Part::Lit(next)) => {
                    // (the shortest value the rest matches with)
                    let mut from = 1;
                    while from <= text.len() {
                        let Some(at) = text.get(from..).and_then(|t| t.find(next)).map(|i| i + from) else { return None };
                        values.push((name.to_string(), text[..at].to_string()));
                        if let Some(left) = match_from(parts, k + 1, &text[at..], values, prefix) {
                            return Some(left);
                        }
                        values.pop();
                        from = at + 1;
                        while from < text.len() && !text.is_char_boundary(from) {
                            from += 1;
                        }
                    }
                    None
                }
                Some(Part::Var(_)) => None,
            }
        }
    }
}

/// What may follow a message that is matched whole by a template's start:
/// a hint the compiler adds (` — did you mean …`, ` (in …)`, `: …`).
fn tail_ok(tail: &str) -> bool {
    [" — ", " (", ": ", ". ", "; ", ", ", " - "].iter().any(|t| tail.starts_with(t))
}

/// The message `text` is, with its values (`name` → value), and what
/// follows it that the template doesn't have (a hint, left as it is).
pub fn identify(text: &str) -> Option<(&'static Message, Vec<(String, String)>, String)> {
    for m in MESSAGES {
        let ps = parts(m.en);
        let mut values = Vec::new();
        if match_from(&ps, 0, text, &mut values, false).is_some() {
            return Some((m, values, String::new()));
        }
    }
    for m in MESSAGES {
        let ps = parts(m.en);
        let mut values = Vec::new();
        if let Some(left) = match_from(&ps, 0, text, &mut values, true) {
            let tail = &text[text.len() - left..];
            if !tail.is_empty() && tail_ok(tail) {
                return Some((m, values, tail.to_string()));
            }
        }
    }
    None
}

// ------------------------------------------------------- translations --

/// The languages compiled in (`lang/<tag>.po`), English aside.
const CATALOGS: &[(&str, &str)] = &[("es", include_str!("../lang/es.po"))];

/// The languages messages are translated into (their tags), the
/// pseudo-language aside.
pub fn languages() -> Vec<&'static str> {
    CATALOGS.iter().map(|(t, _)| *t).collect()
}

/// A language's catalog file as compiled in.
pub fn catalog_text(tag: &str) -> Option<&'static str> {
    CATALOGS.iter().find(|(t, _)| *t == tag).map(|(_, text)| *text)
}

fn translations(tag: &str) -> Option<&'static HashMap<String, String>> {
    static ALL: OnceLock<HashMap<&'static str, HashMap<String, String>>> = OnceLock::new();
    let all = ALL.get_or_init(|| {
        CATALOGS
            .iter()
            .map(|(t, text)| {
                let cat = rapidr_i18n::parse_po(text);
                let map: HashMap<String, String> = MESSAGES.iter().filter_map(|m| cat.text(m.id, m.en).map(|s| (m.id.to_string(), s.to_string()))).collect();
                (*t, map)
            })
            .collect()
    });
    rapidr_i18n::fallback_chain(tag).iter().find_map(|t| all.get(t.as_str()))
}

/// The catalogs' other texts: entries whose context is no message's ID but
/// a hint's key (`hint.set` …, rapidr_langsvc::foreign): per language, each
/// key's (English, translation), and the translation of an English text.
type Extras = (HashMap<String, (String, String)>, HashMap<String, String>);

fn extras(tag: &str) -> Option<&'static Extras> {
    static ALL: OnceLock<HashMap<&'static str, Extras>> = OnceLock::new();
    let all = ALL.get_or_init(|| {
        CATALOGS
            .iter()
            .map(|(t, text)| {
                let mut by_key = HashMap::new();
                let mut by_text = HashMap::new();
                for m in rapidr_i18n::po::read_messages(text).iter().filter(|m| m.context.starts_with("hint.") && m.translated() && !m.has_flag("fuzzy")) {
                    by_key.insert(m.context.clone(), (m.id.clone(), m.forms[0].clone()));
                    by_text.insert(m.id.clone(), m.forms[0].clone());
                }
                (*t, (by_key, by_text))
            })
            .collect()
    });
    rapidr_i18n::fallback_chain(tag).iter().find_map(|t| all.get(t.as_str()))
}

/// A text of the catalog that isn't a compiler message — a hint under one
/// (`hint.<id>`, with its English `en`) — in language `tag`: the translation,
/// the pseudo-language's, or None (English). A translation of an older
/// English text isn't used.
pub fn translate_text(key: &str, en: &str, tag: &str) -> Option<String> {
    let tag = rapidr_i18n::normalize_tag(tag);
    if tag.is_empty() || tag == "en" || tag.starts_with("en-") {
        return None;
    }
    if tag == rapidr_i18n::pseudo::TAG {
        return Some(rapidr_i18n::pseudo::pseudo(en));
    }
    let (by_key, _) = extras(&tag)?;
    by_key.get(key).filter(|(e, _)| e == en).map(|(_, t)| t.clone())
}

/// The hint whose English text is `en`, in language `tag` (a report only
/// has the words, not the hint's key).
pub fn translate_hint(en: &str, tag: &str) -> Option<String> {
    let tag = rapidr_i18n::normalize_tag(tag);
    if tag.is_empty() || tag == "en" || tag.starts_with("en-") {
        return None;
    }
    if tag == rapidr_i18n::pseudo::TAG {
        return Some(rapidr_i18n::pseudo::pseudo(en));
    }
    extras(&tag)?.1.get(en).cloned()
}

/// The word that labels a hint under a message (`hint.label`: "hint"), in
/// language `tag`.
pub fn hint_label(tag: &str) -> String {
    translate_text("hint.label", "hint", tag).unwrap_or_else(|| "hint".to_string())
}

/// Message `id`'s text in language `tag` (its template, `{name}` where a
/// value goes): the translation, the pseudo-language's, or None (English).
pub fn translation(id: &str, tag: &str) -> Option<String> {
    let m = by_id(id)?;
    if rapidr_i18n::normalize_tag(tag) == rapidr_i18n::pseudo::TAG {
        return Some(rapidr_i18n::pseudo::pseudo(m.en));
    }
    translations(tag)?.get(m.id).cloned()
}

/// `text` (a compiler message, in English) in language `tag`: the
/// template's translation with the message's values; `text` itself when
/// no template matches or none is translated.
pub fn localize(text: &str, tag: &str) -> String {
    localized(text, tag).map(|(t, _)| t).unwrap_or_else(|| text.to_string())
}

/// [`localize`], and the message it was (None: left in English).
pub fn localized(text: &str, tag: &str) -> Option<(String, &'static Message)> {
    let tag = rapidr_i18n::normalize_tag(tag);
    if tag.is_empty() || tag == "en" || tag.starts_with("en-") {
        return None;
    }
    let (m, values, tail) = identify(text)?;
    let t = translation(m.id, &tag)?;
    let filled = rapidr_i18n::fill(&t, |name| values.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, v)| v.clone()));
    Some((format!("{filled}{tail}"), m))
}

/// The language the CLI speaks: `RAPIDR_LANG`'s, else the system's
/// (`rapidr_i18n::tags::system_language`).
pub fn cli_language() -> String {
    std::env::var("RAPIDR_LANG").ok().map(|l| rapidr_i18n::normalize_tag(&l)).filter(|t| !t.is_empty()).unwrap_or_else(rapidr_i18n::tags::system_language)
}

/// A report as the compiler prints it — lines `file:line:col: error:
/// message` — in language `tag`: each message translated
/// ([`localize`]), a translated RapidQ message followed by its original
/// in brackets. Other lines as they are.
pub fn localize_report(report: &str, tag: &str) -> String {
    report
        .lines()
        .map(|line| {
            for mark in [": error: ", ": warning: ", ": note: "] {
                if let Some(i) = line.find(mark) {
                    let (head, msg) = line.split_at(i + mark.len());
                    return match localized(msg, tag) {
                        Some((t, m)) if m.rapidq() => format!("{head}{t} [{msg}]"),
                        Some((t, _)) => format!("{head}{t}"),
                        None => line.to_string(),
                    };
                }
            }
            // (the fix a hint gives, under a message: `  hint: …`)
            if let Some(text) = line.strip_prefix("  hint: ") {
                return match translate_hint(text, tag) {
                    Some(t) => format!("  {}: {t}", hint_label(tag)),
                    None => line.to_string(),
                };
            }
            if let Some(msg) = line.strip_prefix("error: ") {
                if let Some((t, _)) = localized(msg, tag) {
                    return format!("error: {t}");
                }
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The template (POT) of the messages: `msgctxt` the ID, `msgid` the
/// English text — what a translator starts a language's file from.
pub fn template() -> Vec<rapidr_i18n::Message> {
    let mut out = vec![rapidr_i18n::tools::header("RapidR compiler messages", "", "")];
    for m in MESSAGES {
        let note = if m.rapidq() { vec!["RapidQ's own message (RC.EXE): the English stays beside the translation".to_string()] } else { Vec::new() };
        out.push(rapidr_i18n::Message { context: m.id.to_string(), id: m.en.to_string(), notes: note, ..Default::default() });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_templates_read() {
        let mut ids: Vec<&str> = MESSAGES.iter().map(|m| m.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), MESSAGES.len());
        for m in MESSAGES {
            assert!(m.id.len() == 6 && (m.id.starts_with("RQ") || m.id.starts_with("RR")), "{}", m.id);
        }
    }

    #[test]
    fn messages_are_identified() {
        let (m, v, tail) = identify("Member CLICK not part of class BTN").unwrap();
        assert_eq!((m.id, v.clone(), tail.as_str()), ("RQ0001", vec![("member".into(), "CLICK".into()), ("class".into(), "BTN".into())], ""));
        let (m, v, _) = identify("Syntax error in END statement (did you mean END SUB?)").unwrap();
        assert_eq!((m.id, v[1].1.as_str()), ("RR0001", "END SUB"));
        assert_eq!(identify("Syntax error in DEFINT statement").unwrap().0.id, "RR0002");
        let (m, _, tail) = identify("Identifier total already used, try another name — it is declared in a.rr:3 already").unwrap();
        assert_eq!((m.id, tail.as_str()), ("RQ0007", " — it is declared in a.rr:3 already"));
        assert_eq!(identify("EXIT FOR is not inside a FOR loop").unwrap().0.id, "RR0009");
        assert!(identify("EXIT FOR is not inside a DO loop").is_none_or(|(m, _, _)| m.id != "RR0009"));
        assert!(identify("Something nobody says").is_none());
    }

    #[test]
    fn localized_in_the_catalogs_languages() {
        // (the sample es.po: RQ0001 and RQ0004 at least)
        let es = localize("Member CLICK not part of class BTN", "es");
        assert_ne!(es, "Member CLICK not part of class BTN");
        assert!(es.contains("CLICK") && es.contains("BTN"), "{es}");
        assert_eq!(localize("Member CLICK not part of class BTN", "en"), "Member CLICK not part of class BTN");
        assert_eq!(localize("Something nobody says", "es"), "Something nobody says");
        let xx = localize("Undeclared identifier foo", "xx");
        assert!(xx.contains("foo") && xx.starts_with('['), "{xx}");
        let report = localize_report("a.rr:3:1: error: Member CLICK not part of class BTN\nplain line", "es-UY");
        assert!(report.starts_with("a.rr:3:1: error: ") && report.contains("[Member CLICK not part of class BTN]"), "{report}");
        assert!(report.ends_with("\nplain line"));
    }

    #[test]
    fn hints_are_translated_through_the_same_catalog() {
        // (English, the pseudo-language, and a hint no catalog has)
        assert_eq!(translate_text("hint.set", "RapidR has no SET", "en"), None);
        assert_eq!(hint_label("en"), "hint");
        let xx = translate_text("hint.set", "RapidR has no SET", "xx").unwrap();
        assert!(xx.starts_with('['), "{xx}");
        let report = "a.rr:2:1: error: Syntax error in SET statement\n  hint: something no catalog has";
        assert_eq!(localize_report(report, "es").lines().nth(1), Some("  hint: something no catalog has"));
        // (a hint of the catalog: its translation, with the hint's label in the language)
        let es = localize_report("  hint: it's END IF, two words", "es");
        assert!(es.starts_with("  sugerencia: ") && !es.contains("two words"), "{es}");
    }

    #[test]
    fn the_template_is_current() {
        // (`rapidr i18n compiler` writes it from MESSAGES)
        let file = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/lang/compiler.pot")).expect("lang/compiler.pot");
        // (the messages first, then the hints under them — rapidr_langsvc::foreign's,
        // written by the same command)
        assert!(file.starts_with(&rapidr_i18n::tools::write(&template())), "lang/compiler.pot is out of date: rapidr i18n compiler");
    }

    #[test]
    fn translations_keep_their_markers() {
        for tag in languages() {
            let file = rapidr_i18n::po::read_messages(catalog_text(tag).unwrap());
            for m in file.iter().filter(|m| !m.is_header() && m.translated()) {
                if m.context.starts_with("hint.") {
                    let problems = rapidr_i18n::markers::check_for(tag, &m.id, &m.forms[0]);
                    assert!(problems.is_empty(), "{tag}.po {}: {problems:?}", m.context);
                    continue;
                }
                let msg = by_id(&m.context).unwrap_or_else(|| panic!("{tag}.po: {} is no message's ID", m.context));
                assert_eq!(msg.en, m.id, "{tag}.po: {}'s English differs from the catalog's", m.context);
                let problems = rapidr_i18n::markers::check_for(tag, &m.id, &m.forms[0]);
                assert!(problems.is_empty(), "{tag}.po {}: {problems:?}", m.context);
            }
        }
    }
}
