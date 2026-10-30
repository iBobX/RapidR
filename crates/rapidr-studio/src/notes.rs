//! What the assistant's tools say they did, in the person's language.
//!
//! A tool's note ("Read main.rr (12 lines)", "Checked main.rr: 2 errors, 0
//! warnings", "Refused a change to main.rr: a syntax error") is English on
//! purpose: the assistant's own code reads it (what is left to fix, which
//! refusal to show, which step label to make) and so does the audit log.
//! The person reads it too — as the steps under an answer ("✓ 6 steps ·
//! show") and as a refusal's card — so the very text is looked up here, at
//! the moment it is shown: found among the notes' templates, its values
//! taken out, and the template's translation filled with them
//! ([`ui_note`]). A note no template knows stays as it is.
//!
//! Every template is written `tr_note!("…")` so `rapidr i18n runtime` lists
//! it with RapidR's other texts (`crates/rapidr-value/lang/runtime.pot`).

use rapidr_i18n::template::match_whole;
use rapidr_value::i18n::{tr_ui, tr_ui_fill};

/// `en`, a note of a tool (or the step label made from it), in the
/// person's language; `en` itself when it is English's, or no template
/// matches it.
pub fn ui_note(en: &str) -> String {
    // (the language is English: nothing to look up)
    let language = rapidr_value::i18n::language();
    if language == "en" || language.starts_with("en-") {
        return en.to_string();
    }
    // (what is left to fix, said after a step: ": 2 errors to fix", " (errors to fix)")
    if let Some(head) = en.strip_suffix(" (errors to fix)") {
        return format!("{} {}", ui_note(head), tr_ui("(errors to fix)"));
    }
    if let Some((head, tail)) = en.rsplit_once(": ") {
        if tail.ends_with(" to fix") || tail == "errors to fix" {
            return format!("{}: {}", ui_note(head), ui_note(tail));
        }
    }
    let pairs = |v: &[(String, String)]| -> Vec<(String, String)> { v.to_vec() };
    macro_rules! tr_note {
        ($tpl:literal) => {
            if let Some(v) = match_whole($tpl, en) {
                let v = pairs(&v);
                let v: Vec<(&str, &str)> = v.iter().map(|(n, x)| (n.as_str(), x.as_str())).collect();
                return tr_ui_fill($tpl, &v);
            }
        };
    }
    // (the steps' own labels, and the notes they are made from: the more
    // specific template of two that could match first)
    tr_note!("Read {file} ({lines} lines)");
    tr_note!("Read {target}'s properties");
    tr_note!("Read {file}");
    tr_note!("Looked up {target} in RapidR's registry");
    tr_note!("Looked up {target}: not found");
    tr_note!("Looked up {target}");
    tr_note!("Looked at the form");
    tr_note!("Looked at the project: none is open");
    tr_note!("Looked for an example of {query}: none close");
    tr_note!("Listed the files");
    tr_note!("Listed the new project {name}'s files");
    tr_note!("Listed the project's files ({n})");
    tr_note!("Listed the forms' components ({n})");
    tr_note!("Checked: {what}");
    tr_note!("Checked the code");
    tr_note!("Checked {file}: no problems");
    tr_note!("Checked {file}: {errors} errors, {warnings} warnings");
    tr_note!("Found an example: {title}");
    tr_note!("Found an example");
    tr_note!("Laid out the buttons");
    tr_note!("Laid out {n} buttons");
    tr_note!("Wrote the program");
    tr_note!("Wrote the new program {title} ({files}): it compiles");
    tr_note!("Wrote the new program {title}");
    tr_note!("Changed the new project {name}: {file}");
    tr_note!("Changed {file}");
    tr_note!("Outlined the project: none is open");
    tr_note!("Outlined the project ({n} files)");
    tr_note!("Searched the manual for {query}: found in RapidR's reference");
    tr_note!("Searched the manual for {query}: nothing");
    tr_note!("Searched the manual for {query}: {where}");
    tr_note!("Couldn't read {file}: not a file of the project");
    tr_note!("Couldn't read {file}");
    tr_note!("Refused to read {file}: outside the project");
    tr_note!("Refused to write {file}: outside the project");
    tr_note!("Refused a change to {file}: {why}");
    tr_note!("Refused a whole-file rewrite of {file}: only a few lines change");
    tr_note!("Refused a shortened {file} that doesn't compile");
    tr_note!("Refused the new program {title}: its main file {main} wasn't sent");
    tr_note!("Refused the new program {title}: {file} is cut off");
    tr_note!("Refused {file}: {why}");
    tr_note!("A change to {file} didn't fit the file");
    tr_note!("Proposed a change to {file}: {summary}");
    tr_note!("Proposed a change to {file}");
    tr_note!("Proposed the new file {file}: {summary}");
    tr_note!("Proposed the new file {file}");
    tr_note!("Proposed a new text for {file}: {summary}");
    tr_note!("Proposed a new text for {file}");
    tr_note!("{n} errors to fix");
    tr_note!("1 error to fix");
    tr_note!("errors to fix");
    tr_note!("write_file: no files");
    tr_note!("write_file: no file named");
    tr_note!("write_file: no project open");
    tr_note!("propose_edit: {file} not found");
    tr_note!("check_code: outside the project");
    tr_note!("check_code: nothing to check");
    tr_note!("check_code: {file} not found");
    tr_note!("create_project {name}: no files");
    tr_note!("create_project {name}: a path outside it");
    tr_note!("create_project {name}: a file given twice");
    tr_note!("create_project {name}: no main file");
    tr_note!("layout: no rows");
    tr_note!("search_manual: no question");
    tr_note!("{name}: not allowed (read-only)");
    tr_note!("{name}: the program isn't running");
    tr_note!("{name}: arguments not understood");
    tr_note!("{name}: no such tool");
    tr_note!("{name}: no answer");
    tr_note!("{name}: cut off, to be written again in smaller parts");
    tr_note!("{name}: no project open");
    tr_note!("{name}: not done");
    tr_note!("{name}: you didn't allow it");
    // (why a change was refused: the card's second half)
    tr_note!("it is cut off at the end (again)");
    tr_note!("it is cut off at the end");
    tr_note!("the same syntax error again");
    tr_note!("a syntax error");
    en.to_string()
}

/// What a step in a project's history says it did ("Saved the version “first”",
/// "Went back to “first” (a1b2c3d)"), in the person's language: Versions'
/// Undo line. The words are written into the history in English (they are
/// data, with the versions' own messages); they are translated where shown.
pub fn ui_history(en: &str) -> String {
    let language = rapidr_value::i18n::language();
    if en.is_empty() || language == "en" || language.starts_with("en-") {
        return en.to_string();
    }
    if let Some(inner) = en.strip_prefix("Undid: ") {
        return tr_ui_fill("Undid: {words}", &[("words", &ui_history(inner))]);
    }
    macro_rules! tr_note {
        ($tpl:literal) => {
            if let Some(v) = match_whole($tpl, en) {
                let v: Vec<(&str, &str)> = v.iter().map(|(n, x)| (n.as_str(), x.as_str())).collect();
                return tr_ui_fill($tpl, &v);
            }
        };
    }
    tr_note!("Saved the version “{message}”");
    tr_note!("Went back to “{message}” ({id}) for {path}");
    tr_note!("Went back to “{message}” ({id})");
    tr_note!("Went back to the checkpoint “{label}”");
    tr_note!("Restored {path} from version {id}");
    tr_note!("Started the experiment “{name}”");
    tr_note!("Switched to the experiment “{name}”");
    tr_note!("Got the latest from the shared folder ({experiment})");
    en.to_string()
}
