//! A running program's run-time error messages in the person's language.
//!
//! The runtime words its errors in English where it finds them — as
//! RapidQ's own did, `Division by zero` — and its own code, the debugger,
//! `Application.OnError` and the tests read those words, so they stay what
//! they are. Where a person reads them (the line a stopped program prints,
//! `rapidr run`'s report) the very text is found among the messages'
//! templates, its values taken out, and the template's translation filled
//! with them ([`localize`], the way the assistant's step notes are:
//! `rapidr_i18n::template`). The language is the program's
//! (`Application.Language`, else `RAPIDR_LANG`, else the system's); a
//! message no template knows, or one without a translation, stays English.
//!
//! Every template is written `tr_note!("…")`, so `rapidr i18n runtime`
//! lists it with RapidR's other texts and `crates/rapidr-value/lang/<language>.po`
//! translates it.

use crate::i18n::{language, tr_ui_fill};
use rapidr_i18n::template::match_whole;

/// Run-time error message `en` in the person's language: the translation of
/// the template it is, else `en`.
pub fn localize(en: &str) -> String {
    let lang = language();
    if lang == "en" || lang.starts_with("en-") {
        return en.to_string();
    }
    macro_rules! tr_note {
        ($tpl:literal) => {
            if let Some(v) = match_whole($tpl, en) {
                let v: Vec<(&str, &str)> = v.iter().map(|(n, x)| (n.as_str(), x.as_str())).collect();
                return tr_ui_fill($tpl, &v);
            }
        };
    }
    tr_note!("Division by zero");
    tr_note!("Subscript out of range: index {i} is outside {lo} TO {hi}");
    tr_note!("Out of DATA: READ found no more DATA items");
    tr_note!("indexing a value that is not an array");
    tr_note!("assigning to an element of a variable that is not an array (DIM it with a size first)");
    tr_note!("stack overflow: more than {n} nested calls (a SUB or FUNCTION calling itself without end?)");
    tr_note!("arrays have at most {n} dimensions");
    tr_note!("string too long: strings are limited to {n} characters");
    tr_note!("this variable does not refer to an object (reading a field of {what})");
    tr_note!("this variable does not refer to an object (setting a field of {what})");
    tr_note!("CALLFUNC: {ptr} is not a function pointer (use BIND or CODEPTR)");
    tr_note!("Unknown builtin function '{name}'");
    tr_note!("the evaluation ran too long and was stopped");
    tr_note!("END can't run in an evaluation");
    tr_note!("the evaluation ended the program");
    tr_note!("the evaluation would wait (a dialog or a form): not while the program is stopped");
    tr_note!("too many event arguments");
    tr_note!("no frame {i}");
    tr_note!("{what}({n}): strings are limited to {max} characters");
    tr_note!("{what} ran inside itself {n} times: its handler fires it again before it ends");
    tr_note!("events ran inside each other {n} deep (the last: {what}): their handlers fire events before they end");
    tr_note!("{name} reads or writes a hardware port, which a program can't do on a modern system");
    tr_note!("'{name}' is a function of a DLL ({base}): this program calls Windows itself, and the web can't load DLLs, so it runs on Windows only");
    tr_note!("'{name}' is a Windows function ({base}): this program calls Windows itself, so it runs on Windows only");
    tr_note!("'{name}' is a function of {base}.dll, a Windows DLL: this program calls a DLL, so it runs on Windows only");
    tr_note!("this variable does not refer to an object (reading field {slot} of {what})");
    tr_note!("this variable does not refer to an object (setting field {slot} of {what})");
    tr_note!("out of memory for addresses (VARPTR)");
    tr_note!("address {addr} belongs to an array or TYPE that no longer exists");
    tr_note!("address {addr} isn't memory of this program (use VARPTR of a variable, an array element, a TYPE, or a stream's Pointer)");
    tr_note!("{n} bytes at address {addr} run past the end of its memory ({left} bytes left)");
    tr_note!("a STRING isn't an address: use VARPTR(s$)");
    tr_note!("VARPTR of an element of something that isn't an array");
    tr_note!("a negative byte count ({n})");
    tr_note!("PEEK takes an address, or a page and an address");
    tr_note!("POKE takes an address and a byte, or a page, an address and a byte");
    tr_note!("Cannot open file {path}.");
    tr_note!("Cannot create file {path}.");
    tr_note!("{what}: argument is not an array, or it has no dimension {d}");
    // (a message under the object or the variable it came from: `Sales.At: …`,
    // `DIM Grid: …` — the message after it, translated, and the head as it is)
    if let Some((head, tail)) = en.split_once(": ") {
        let plain = |h: &str| !h.is_empty() && !h.contains(' ') && !h.contains('(');
        if (plain(head) || head.strip_prefix("DIM ").is_some_and(plain)) && !tail.is_empty() {
            let said = localize(tail);
            if said != tail {
                return format!("{head}: {said}");
            }
        }
    }
    en.to_string()
}

/// A debugged program's stop (`Session.StopMessage`: a run-time error's
/// message, why a program that waits for its events was paused, a
/// breakpoint's condition that failed) in the person's language.
pub fn localize_stop(en: &str) -> String {
    let lang = language();
    if lang == "en" || lang.starts_with("en-") || en.is_empty() {
        return en.to_string();
    }
    if en == "Waiting for events" {
        return crate::i18n::tr_ui("Waiting for events");
    }
    // (the VM words a program's error `run-time error: …`: the message after it)
    if let Some(message) = en.strip_prefix("run-time error: ") {
        return tr_ui_fill("run-time error: {message}", &[("message", &localize(message))]);
    }
    if let Some(v) = match_whole("The breakpoint's condition `{cond}` failed: {error}", en) {
        let get = |n: &str| v.iter().find(|(k, _)| k == n).map(|(_, x)| x.clone()).unwrap_or_default();
        return tr_ui_fill("The breakpoint's condition `{cond}` failed: {error}", &[("cond", &get("cond")), ("error", &localize(&get("error")))]);
    }
    localize(en)
}

/// A report that has a run-time error in it (`run-time error: Division by
/// zero (at calc.rr line 12)`, maybe after `vm error: `), each such line
/// with its message and its place in the person's language; any other line
/// as it is.
pub fn localize_report(report: &str) -> String {
    let lang = language();
    if lang == "en" || lang.starts_with("en-") {
        return report.to_string();
    }
    report
        .lines()
        .map(|line| {
            let Some(at) = line.find("run-time error: ") else { return line.to_string() };
            let (head, rest) = line.split_at(at);
            let rest = &rest["run-time error: ".len()..];
            // (the place, when the VM says it: ` (at calc.rr line 12)`)
            let (message, place) = match rest.rfind(" (at ") {
                Some(i) if rest.ends_with(')') && rest[i..].contains(" line ") => (&rest[..i], Some(&rest[i + 5..rest.len() - 1])),
                _ => (rest, None),
            };
            let message = localize(message);
            let mut out = format!("{head}{}", tr_ui_fill("run-time error: {message}", &[("message", &message)]));
            if let Some(place) = place {
                if let Some((file, n)) = place.rsplit_once(" line ") {
                    out.push_str(&format!(" {}", tr_ui_fill("(at {file} line {n})", &[("file", file), ("n", n)])));
                } else {
                    out.push_str(&format!(" (at {place})"));
                }
            }
            out
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_is_left_as_it_is() {
        crate::i18n::set_language("en");
        assert_eq!(localize("Division by zero"), "Division by zero");
        let r = "vm error: run-time error: Division by zero (at calc.rr line 12)";
        assert_eq!(localize_report(r), r);
    }

    #[test]
    fn a_stop_and_a_message_under_its_object_are_translated() {
        crate::i18n::set_language("es");
        assert_eq!(localize_stop("run-time error: Division by zero"), "error de ejecución: División por cero");
        assert_eq!(localize_stop(""), "");
        assert_eq!(localize("Cannot open file a.txt."), "No se puede abrir el archivo a.txt.");
        assert_eq!(localize("Sales.At: Division by zero"), "Sales.At: División por cero");
        assert_eq!(localize("DIM Grid: Something nobody says"), "DIM Grid: Something nobody says");
        let waiting = localize_stop("Waiting for events");
        assert_ne!(waiting, "Waiting for events");
        crate::i18n::set_language("en");
        assert_eq!(localize_stop("run-time error: Division by zero"), "run-time error: Division by zero");
        assert_eq!(localize("Sales.At: Division by zero"), "Sales.At: Division by zero");
    }

    #[test]
    fn a_known_message_is_translated_and_an_unknown_one_is_not() {
        crate::i18n::set_language("es");
        assert_ne!(localize("Division by zero"), "Division by zero");
        let said = localize("Subscript out of range: index 9 is outside 1 TO 3");
        assert!(said.contains('9') && said.contains('3') && !said.contains("outside"), "{said}");
        assert_eq!(localize("Something nobody says"), "Something nobody says");
        let r = localize_report("vm error: run-time error: Division by zero (at calc.rr line 12)\nplain");
        assert!(r.starts_with("vm error: ") && r.contains("calc.rr") && r.contains("12") && !r.contains("Division") && r.ends_with("\nplain"), "{r}");
        crate::i18n::set_language("en");
    }
}
