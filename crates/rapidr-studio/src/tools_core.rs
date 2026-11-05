//! The shared tool set's newer core handlers (docs/ai-tools.md), the same
//! for RAI and AI apps: RapidR's examples found by what they are and use
//! (`find_example`'s second source), the workflows (`how_do_i`), a gap
//! written down (`report_gap`), and the project's versions read
//! (`list_versions`, `compare_versions`).

use std::cell::RefCell;

use serde_json::Value as Json;

use crate::ai_tools::{Outcome, Project};

fn ok(content: String, note: String, target: &str) -> Outcome {
    Outcome { content, is_error: false, note, read: None, risk: "read", target: target.to_string() }
}

fn err(content: impl Into<String>, note: String, target: &str) -> Outcome {
    Outcome { content: content.into(), is_error: true, note, read: None, risk: "read", target: target.to_string() }
}

fn s<'a>(input: &'a Json, key: &str) -> &'a str {
    input.get(key).and_then(Json::as_str).unwrap_or("").trim()
}

// ------------------------------------------------------------ the examples --

thread_local! {
    /// RapidR's examples folder (Studio's home + examples: RAIProjectTools.
    /// ExamplesFolder) and the examples read from it.
    static EXAMPLES: RefCell<(String, Option<Vec<Example>>)> = const { RefCell::new((String::new(), None)) };
}

/// One of RapidR's examples, as find_example searches it.
#[derive(Clone, Debug)]
pub struct Example {
    /// Its path under the examples folder (`data/json.rr`).
    pub rel: String,
    /// Its header comment's first sentence ("reading and writing JSON …").
    pub about: String,
    /// The components it uses (RapidR's names, upper case: RJSON).
    pub components: Vec<String>,
    /// The builtins it calls (upper case).
    pub builtins: Vec<String>,
    /// Its header's words and its file name's, folded.
    pub words: Vec<String>,
    pub text: String,
}

/// Where RapidR's examples are (Studio sets it: its home's `examples`).
pub fn set_examples_folder(folder: &str) {
    let folder = crate::slashes(folder.trim().trim_end_matches(['/', '\\']));
    EXAMPLES.with(|e| {
        let mut e = e.borrow_mut();
        if e.0 != folder {
            *e = (folder, None);
        }
    });
}

pub fn examples_folder() -> String {
    EXAMPLES.with(|e| e.borrow().0.clone())
}

fn read_examples(folder: &str) -> Vec<Example> {
    // (the web has no examples folder to read: none)
    #[cfg_attr(target_arch = "wasm32", allow(unused_mut))]
    let mut out = Vec::new();
    #[cfg(not(target_arch = "wasm32"))]
    {
        fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            let Ok(rd) = std::fs::read_dir(dir) else { return };
            let mut entries: Vec<_> = rd.flatten().map(|e| e.path()).collect();
            entries.sort();
            for p in entries {
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.starts_with('.') {
                    continue;
                }
                if p.is_dir() {
                    walk(&p, out);
                } else if matches!(p.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref(), Some("rr" | "bas")) {
                    out.push(p);
                }
            }
        }
        let root = std::path::Path::new(folder);
        let mut files = Vec::new();
        walk(root, &mut files);
        for f in files {
            let Ok(text) = std::fs::read_to_string(&f) else { continue };
            let rel = crate::slashes(&f.strip_prefix(root).unwrap_or(&f).to_string_lossy());
            out.push(example_of(&rel, text));
        }
    }
    #[cfg(target_arch = "wasm32")]
    let _ = folder;
    out
}

/// An example read: its header, what it uses, its words.
pub fn example_of(rel: &str, text: String) -> Example {
    // (the header: the comment lines it starts with, its file name first)
    let head: Vec<String> = text.lines().take_while(|l| l.trim_start().starts_with('\'') || l.trim().is_empty()).map(|l| l.trim_start().trim_start_matches('\'').trim().to_string()).take(30).collect();
    let file = rel.rsplit('/').next().unwrap_or(rel);
    let first = head.iter().find(|l| !l.is_empty()).cloned().unwrap_or_default();
    let about = first.strip_prefix(&format!("{file}:")).map(str::trim).unwrap_or(&first).to_string();
    let mut components: Vec<String> = Vec::new();
    let mut builtins: Vec<String> = Vec::new();
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with('\'') {
            continue;
        }
        // (`CREATE x AS RButton`, `DIM x AS RJson`, `x AS NEW RTimer`)
        let words: Vec<&str> = l.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$')).filter(|w| !w.is_empty()).collect();
        for pair in words.windows(2) {
            if pair[0].eq_ignore_ascii_case("AS") || pair[0].eq_ignore_ascii_case("NEW") {
                if let Some(c) = rapidr_lang::component(pair[1]) {
                    let n = c.name.to_ascii_uppercase();
                    if !components.contains(&n) {
                        components.push(n);
                    }
                }
            }
        }
        for w in &words {
            if w.len() > 2 && rapidr_lang::builtin(w).is_some() {
                let n = w.to_ascii_uppercase();
                if !builtins.contains(&n) {
                    builtins.push(n);
                }
            }
        }
    }
    let stem = file.rsplit_once('.').map_or(file, |(s, _)| s);
    let mut words = crate::recipes::words(&format!("{} {} {}", stem.replace('_', " "), rel.replace(['/', '_', '.'], " "), head.join(" ")));
    words.dedup();
    Example { rel: rel.to_string(), about, components, builtins, words, text }
}

/// RapidR's examples (read once from the folder Studio gave).
pub fn examples() -> Vec<Example> {
    EXAMPLES.with(|e| {
        let mut e = e.borrow_mut();
        if e.1.is_none() {
            // (none given — a test, a tool —: the checkout's, else the
            // working folder's)
            let built = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples");
            let folder = if !e.0.is_empty() {
                e.0.clone()
            } else if std::path::Path::new(built).join("README.md").is_file() {
                built.to_string()
            } else {
                "examples".to_string()
            };
            e.1 = Some(read_examples(&folder));
        }
        e.1.clone().unwrap_or_default()
    })
}

/// How an example matched.
pub enum Match {
    /// Its file name is the query (`rai_models`, `json.rr`).
    File,
    /// It uses the component or builtin the query names.
    Uses,
    /// Its header's words.
    Words(f64),
}

/// The examples close to `query`, best first: a file name first, then the
/// component or builtin it names, then the words.
pub fn search_examples(query: &str) -> Vec<(Match, Example)> {
    let all = examples();
    let q = query.trim();
    let stem = q.trim_start_matches("examples/").rsplit('/').next().unwrap_or(q);
    let stem = stem.rsplit_once('.').map_or(stem, |(s, _)| s).to_ascii_lowercase().replace([' ', '-'], "_");
    let by_file: Vec<(Match, Example)> = all.iter().filter(|e| e.rel.rsplit('/').next().and_then(|f| f.rsplit_once('.')).is_some_and(|(s, _)| s.eq_ignore_ascii_case(&stem))).cloned().map(|e| (Match::File, e)).collect();
    if !by_file.is_empty() {
        return by_file;
    }
    let named = named_in(q);
    if let Some(name) = &named {
        let found: Vec<(Match, Example)> = all.iter().filter(|e| e.components.contains(name) || e.builtins.contains(name)).cloned().map(|e| (Match::Uses, e)).collect();
        if !found.is_empty() {
            // (the smallest first: the clearest use)
            let mut found = found;
            found.sort_by_key(|(_, e)| e.text.len());
            return found;
        }
    }
    let qw = crate::recipes::words(q);
    if qw.is_empty() {
        return Vec::new();
    }
    let mut scored: Vec<(f64, Example)> = all
        .into_iter()
        .map(|e| {
            let hits = qw.iter().filter(|w| e.words.contains(w)).count() as f64;
            (hits / qw.len() as f64, e)
        })
        .filter(|(sc, _)| if qw.len() <= 2 { *sc >= 0.99 } else { *sc >= 0.66 })
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.text.len().cmp(&b.1.text.len())));
    scored.into_iter().map(|(sc, e)| (Match::Words(sc), e)).collect()
}

/// The component or builtin a query names (RapidR's name, upper case), if
/// it is one word that is one.
pub fn named_in(query: &str) -> Option<String> {
    let w = query.trim().trim_matches(|c: char| c == '"' || c == '`' || c == '(' || c == ')');
    if w.is_empty() || w.contains(char::is_whitespace) {
        return None;
    }
    rapidr_lang::component(w).map(|c| c.name.to_ascii_uppercase()).or_else(|| rapidr_lang::builtin(w).map(|b| b.name.to_ascii_uppercase()))
}

/// The display name of what `named_in` gave (RJson, MID$).
fn display_of(name: &str) -> String {
    rapidr_lang::component(name).map(|c| c.pretty(c.name)).or_else(|| rapidr_lang::builtin(name).map(|b| b.name.to_string())).unwrap_or_else(|| name.to_string())
}

/// The most of an example's code a model is given at once (lines).
const EXAMPLE_LINES: usize = 160;

/// find_example's answer from RapidR's examples, when they have one.
pub fn examples_answer(query: &str) -> Option<Outcome> {
    let found = search_examples(query);
    let (how, best) = found.first()?;
    let named = named_in(query).map(|n| display_of(&n));
    let mut text = String::new();
    if let (Some(n), Match::Uses) = (&named, how) {
        let what = if rapidr_lang::component(n).is_some() { format!("{n} is a component: rapidr_describe {{\"type\": \"{n}\"}} says its properties, methods and events (the truth about what exists).") } else { format!("{n} is a builtin: rapidr_describe {{\"builtin\": \"{n}\"}} says its syntax.") };
        text.push_str(&what);
        text.push('\n');
    }
    let list: Vec<String> = found.iter().take(4).map(|(_, e)| format!("examples/{} — {}", e.rel, e.about)).collect();
    text.push_str(&format!("RapidR's examples{}: {}\n", named.as_ref().filter(|_| matches!(how, Match::Uses)).map(|n| format!(" that use {n}")).unwrap_or_default(), list.join("; ")));
    let lines: Vec<&str> = best.text.lines().collect();
    let shown = lines.iter().take(EXAMPLE_LINES).copied().collect::<Vec<_>>().join("\n");
    let more = if lines.len() > EXAMPLE_LINES { format!("\n(… {} more lines: open_project examples/{} or read it in Studio's Examples)", lines.len() - EXAMPLE_LINES, best.rel) } else { String::new() };
    text.push_str(&format!("\nexamples/{}, working code to start from — adapt it, don't copy names you don't need:\n```basic\n{}\n```{more}", best.rel, shown.trim_end()));
    Some(ok(text, format!("Found RapidR's example {}", best.rel), query))
}

// ------------------------------------------------------------------ how_do_i --

/// The workflows, each a numbered checklist with the exact tools.
const HOW: &[(&str, &[&str], &str)] = &[
    (
        "change code",
        &["change", "edit", "modify", "update", "add", "feature", "code", "button", "cambiar"],
        "To change code in the user's program:
1. studio_status — what is open; the open project IS the user's program.
2. save_version {\"message\": \"Before: <the change>\"} — a point to come back to.
3. read_file {\"path\": \"main.rr\"} — the code as it is now.
4. propose_edit — only the lines that change (old_text copied exactly from the file).
5. check_code — fix every error (its hint line says the RapidR form) until Problems = 0.
6. run_program {\"action\": \"start\"} — then app_layout or app_screenshot to see it, read_output for what it printed.
7. save_version {\"message\": \"<the change> works\"}.
Never use git or a shell for the user's project: the Versions tools are the way.",
    ),
    (
        "fix an error",
        &["error", "fix", "problem", "problems", "compile", "doesn't compile", "warning", "hint"],
        "To fix an error:
1. check_code — each error has its line, column and a hint line with the RapidR form.
2. read_file the file, then propose_edit the lines in error as the hint says.
3. check_code again. Problems must be 0 before you run, measure or report.
4. The same error twice: stop rewriting; rapidr_describe the component or builtin, or find_example and start from working code.
5. A member \"not part of\" a component: rapidr_describe {\"type\": \"<the component>\"} lists what exists — never call a member it doesn't list.",
    ),
    (
        "debug a crash",
        &["debug", "crash", "crashes", "breakpoint", "step", "variable", "variables", "stops", "wrong value", "hangs"],
        "To debug a crash or a wrong value:
1. run_program {\"action\": \"start\", \"debug\": true} — read what it printed and where it stopped.
2. read_output — the error message and its line.
3. debug {\"action\": \"breakpoint\", \"file\": \"main.rr\", \"line\": <a line before it>}.
4. run_program {\"action\": \"restart\", \"debug\": true} — it stops at the breakpoint.
5. debug {\"action\": \"state\"} — where it is, the call stack and the variables, in text.
6. debug {\"action\": \"step_over\"} (or step_into, continue) and state again, until you see the wrong value.
7. Fix it with propose_edit, check_code, run_program. Never attach another debugger or a shell.",
    ),
    (
        "add a form",
        &["form", "window", "dialog", "screen", "second form", "ventana"],
        "To add a window (a form) to the program:
1. find_example {\"query\": \"dialog\"} — a working form to start from.
2. write_file {\"path\": \"settings_form.rr\", …} — CREATE SettingsForm AS RForm … END CREATE, its controls and handlers.
3. propose_edit main.rr — $INCLUDE \"settings_form.rr\" at the top, and SettingsForm.ShowModal where it opens.
4. check_code — Problems = 0; project_outline if a name is already used.
5. layout_report — nothing cut off or on top of another; set_property to move or resize.
6. run_program, then app_layout and app_click to open it and see it.",
    ),
    (
        "learn a component",
        &["learn", "component", "how does", "what is", "members", "property", "method", "event", "use"],
        "To learn a component or builtin:
1. rapidr_describe {\"type\": \"RJson\"} (or {\"builtin\": \"MID$\"}) — the truth: its properties, methods, events, an example.
2. find_example {\"query\": \"RJson\"} — RapidR's examples that use it, working code.
3. search_manual {\"query\": \"read a JSON file\"} — the guide's sections.
Never read RapidR's own source code to learn the language, and never call a member rapidr_describe didn't list.",
    ),
    (
        "start a new app",
        &["new app", "new program", "start", "build me", "create", "empty", "make"],
        "To start a new program:
1. studio_status — an empty project open: write into it; nothing open: create_project.
2. find_example {\"query\": \"<what the user wants, in a few words>\"} — start from working code.
3. write_file {\"path\": \"main.rr\", …} (or create_project) — the example adapted: names, captions, logic.
4. check_code until Problems = 0.
5. run_program, then app_screenshot or app_layout to see it.
6. save_version {\"message\": \"First version\"} (start_versions first if list_versions says there are none).",
    ),
    (
        "test the running app",
        &["test", "click", "type", "running", "try", "use the app", "ui", "screenshot", "see"],
        "To test the program like a person:
1. check_code — Problems = 0.
2. run_program {\"action\": \"start\"}.
3. app_layout — its windows and controls, with the names the tools take.
4. app_click {\"caption\": \"OK\"}, app_type {\"label\": \"Name\", \"text\": \"Ada\"}, app_key {\"keys\": \"Enter\"}.
5. app_layout or app_screenshot again — what changed; read_output for what it printed.
6. run_program {\"action\": \"stop\"} when done. Report only what you saw.",
    ),
    (
        "versions",
        &["version", "versions", "save", "undo", "restore", "go back", "history", "git", "commit", "revert", "backup"],
        "To keep and use versions (never git or a shell):
1. list_versions — the saved versions and their ids (none yet: save_version starts Versions and saves the first).
2. save_version {\"message\": \"…\"} — before a big step and after each step that works.
3. compare_versions {} — what changed since the last version (or {\"from\": \"<id>\"}).
4. restore_version {\"version\": \"<id>\"} — back to that version as a new one (the user is asked; nothing is lost, Undo takes it back).",
    ),
    (
        "report a gap",
        &["gap", "missing", "report", "can't find", "no tool", "docs", "bug"],
        "When a tool you need is missing, or the docs didn't answer:
1. how_do_i, search_manual and rapidr_describe first.
2. report_gap {\"what\": \"…\", \"kind\": \"missing_tool\", \"tried\": \"…\"} — written down in RAPIDR-ISSUES.md for RapidR's makers.
3. Tell the user in a sentence, and go on with what you can do. Never improvise with a shell or another program.",
    ),
];

/// Every workflow, one after the other (Studio's MCP resource
/// rapidr://guide/workflows).
pub fn all_workflows() -> String {
    HOW.iter().map(|(title, _, steps)| format!("## How to {title}\n\n{steps}\n")).collect::<Vec<_>>().join("\n")
}

/// how_do_i: the workflow closest to `task` (or the list of them).
pub fn how_do_i(input: &Json) -> Outcome {
    let task = s(input, "task").to_lowercase();
    let topics: Vec<&str> = HOW.iter().map(|(t, _, _)| *t).collect();
    if task.is_empty() {
        return err(format!("give `task`: what you want to do — {}", topics.join(", ")), "how_do_i: no task".into(), "");
    }
    let words = crate::recipes::words(&task);
    let score = |(title, keys, _): &(&str, &[&str], &str)| -> usize {
        let mut sc = 0;
        if task.contains(title) {
            sc += 10;
        }
        for k in keys.iter() {
            if k.contains(' ') {
                if task.contains(k) {
                    sc += 3;
                }
            } else if words.iter().any(|w| w == k || w.trim_end_matches('s') == *k) {
                sc += 2;
            }
        }
        sc
    };
    let best = HOW.iter().max_by_key(|h| score(h)).filter(|h| score(h) > 0);
    match best {
        Some((title, _, steps)) => ok(steps.to_string(), format!("Looked up how to {title}"), title),
        None => ok(format!("No workflow for \"{task}\". The workflows: {}. Ask how_do_i with one of them, or search_manual.", topics.join(", ")), "how_do_i: none close".into(), ""),
    }
}

// ---------------------------------------------------------------- report_gap --

/// The file report_gap writes, in the project's folder.
pub const ISSUES_FILE: &str = "RAPIDR-ISSUES.md";
/// The largest the file may grow to (bytes): past it, report_gap refuses.
const ISSUES_MAX: usize = 256 * 1024;

/// A text a model gave, safe in a Markdown list: control characters gone,
/// no heading or HTML started by it, at most `max` characters.
fn clean(text: &str, max: usize) -> String {
    let t: String = text.chars().filter(|c| !c.is_control() || *c == '\n').take(max).collect();
    t.lines().map(|l| l.trim().trim_start_matches(['#', '>']).trim()).filter(|l| !l.is_empty()).collect::<Vec<_>>().join(" / ").replace('<', "‹").replace('>', "›")
}

/// report_gap: a short entry at the end of the project's RAPIDR-ISSUES.md
/// (made when absent); nothing else is ever written.
pub fn report_gap(input: &Json, p: &Project) -> Outcome {
    let what = clean(s(input, "what"), 600);
    if what.is_empty() {
        return err("give `what`: what is missing, in one or two sentences", "report_gap: nothing said".into(), "");
    }
    if p.folder.is_empty() {
        return err("No project is open, so there is no RAPIDR-ISSUES.md to write: tell the user about the gap instead.", "report_gap: no project".into(), "");
    }
    let kind = match s(input, "kind") {
        "missing_tool" => "a missing tool",
        "docs" => "the docs didn't answer",
        "bug" => "something didn't work",
        _ => "a gap",
    };
    let tried = clean(s(input, "tried"), 300);
    let who = match s(input, "_caller") {
        "" => "an AI app".to_string(),
        c => clean(c, 60),
    };
    let path = crate::join(p.folder, ISSUES_FILE);
    // (only that file, for real: never through a link out of the project)
    #[cfg(not(target_arch = "wasm32"))]
    if std::fs::symlink_metadata(&path).is_ok_and(|m| !m.file_type().is_file()) {
        return err(format!("{ISSUES_FILE} isn't a plain file in the project: nothing written. Tell the user about the gap instead."), "report_gap: not a plain file".into(), ISSUES_FILE);
    }
    let before = crate::read_text(&path).unwrap_or_default();
    if before.len() > ISSUES_MAX {
        return err(format!("{ISSUES_FILE} is full ({} KB): nothing written. Tell the user about the gap instead.", before.len() / 1024), "report_gap: the file is full".into(), ISSUES_FILE);
    }
    let when = rapidr_ai::audit::now_iso().replace('T', " ").trim_end_matches('Z').chars().take(16).collect::<String>();
    let mut text = if before.trim().is_empty() {
        "# RapidR issues\n\nWhat AI tools found missing while working on this project: a tool they needed, a question RapidR's docs didn't answer, something that didn't work. Each line is short; send the ones that matter to RapidR's makers.\n".to_string()
    } else {
        before.clone()
    };
    if !text.ends_with('\n') {
        text.push('\n');
    }
    let n = text.lines().filter(|l| l.starts_with("- ")).count() + 1;
    let tried = if tried.is_empty() { String::new() } else { format!(" Tried: {}.", tried.trim_end_matches('.')) };
    let stop = if what.ends_with(['.', '!', '?']) { "" } else { "." };
    text.push_str(&format!("- {when} — {kind} (from {who}): {what}{stop}{tried}\n"));
    if let Err(e) = crate::write_text(&path, &text) {
        return err(format!("{ISSUES_FILE} couldn't be written ({e}). Tell the user about the gap instead."), "report_gap: not written".into(), ISSUES_FILE);
    }
    Outcome { content: format!("Noted in {ISSUES_FILE} (entry {n}). Thank you: go on with what you can do, and tell the user about the gap in a sentence."), is_error: false, note: format!("Noted a gap in {ISSUES_FILE}"), read: None, risk: "state", target: ISSUES_FILE.to_string() }
}

// ------------------------------------------------------------------ versions --

/// "3 minutes ago", "2 days ago".
fn ago(seconds: i64) -> String {
    let now = crate::local_history::now();
    let d = (now - seconds).max(0);
    match d {
        0..=59 => "just now".into(),
        60..=3599 => format!("{} min ago", d / 60),
        3600..=86_399 => format!("{} h ago", d / 3600),
        _ => format!("{} days ago", d / 86_400),
    }
}

/// list_versions: the project's versions, newest first, and what changed
/// since the last one.
pub fn list_versions(input: &Json, p: &Project) -> Outcome {
    if p.folder.is_empty() {
        return err("No project is open: there are no versions to list.", "list_versions: no project".into(), "");
    }
    let limit = input.get("limit").and_then(Json::as_u64).unwrap_or(20).clamp(1, 200) as usize;
    match crate::repository::read::timeline(p.folder, limit) {
        Err(why) => err(why, "list_versions: no versions".into(), ""),
        Ok(None) => ok("This project doesn't keep versions yet: save_version starts them and saves the first one (or start_versions only starts them). Never use git or a shell instead.".into(), "Looked at the versions: none yet".into(), ""),
        Ok(Some(rows)) => {
            let mut out = Vec::new();
            if rows.is_empty() {
                out.push("No version saved yet: save_version saves the first one.".to_string());
            } else {
                out.push("Versions, newest first (id — when — who — name):".to_string());
                for r in &rows {
                    let mark = if r.checkpoint { " (a checkpoint)" } else { "" };
                    out.push(format!("{} — {} — {} — {}{mark}", r.short, ago(r.seconds), r.author, r.message));
                }
            }
            let changed = crate::repository::read::changed(p.folder).unwrap_or_default();
            if changed.is_empty() {
                out.push("Nothing changed since the last version.".into());
            } else {
                out.push(format!("Changed since the last version (saved files; unsaved editor changes aren't counted until saved): {}.", changed.iter().map(|(st, f)| format!("{f} ({})", state_word(*st))).collect::<Vec<_>>().join(", ")));
            }
            ok(out.join("\n"), format!("Listed the versions ({})", rows.len()), "")
        }
    }
}

fn state_word(c: char) -> &'static str {
    match c {
        'N' => "new",
        'D' => "removed",
        _ => "changed",
    }
}

/// The most diff lines compare_versions gives at once.
const DIFF_LINES: usize = 400;

/// compare_versions: what changed between `from` (default the last
/// version) and `to` (default the files as they are now, the editor's
/// unsaved text included), as a diff per file.
pub fn compare_versions(input: &Json, p: &Project) -> Outcome {
    if p.folder.is_empty() {
        return err("No project is open: there is nothing to compare.", "compare_versions: no project".into(), "");
    }
    let rows = match crate::repository::read::timeline(p.folder, 1) {
        Err(why) => return err(why, "compare_versions: no versions".into(), ""),
        Ok(None) => return err("This project doesn't keep versions yet: start_versions, then save_version.", "compare_versions: no versions".into(), ""),
        Ok(Some(r)) => r,
    };
    let from = match s(input, "from") {
        "" => match rows.first() {
            Some(r) => r.id.clone(),
            None => return err("No version saved yet: save_version first.", "compare_versions: no version".into(), ""),
        },
        f => f.to_string(),
    };
    let to = s(input, "to").to_string();
    let only = match s(input, "path") {
        "" => None,
        path => match crate::ai_tools::resolve(p, path) {
            Ok(rel) => Some(rel),
            Err(e) => return err(e, "compare_versions: not a project file".into(), path),
        },
    };
    // (the files either side may have)
    let mut paths: Vec<String> = Vec::new();
    let mut add = |f: &str| {
        if !f.is_empty() && !paths.iter().any(|x| x == f) {
            paths.push(f.to_string());
        }
    };
    match &only {
        Some(rel) => add(rel),
        None => {
            for f in p.files.iter().chain(p.buffers.keys()) {
                add(f);
            }
            for f in crate::repository::read::files_in(p.folder, &from) {
                add(&f);
            }
            if !to.is_empty() {
                for f in crate::repository::read::files_in(p.folder, &to) {
                    add(&f);
                }
            }
        }
    }
    let mut out = Vec::new();
    let mut lines = 0usize;
    let mut cut = false;
    let mut changed = 0;
    for rel in &paths {
        let old = crate::repository::read::file_at(p.folder, rel, &from);
        let new = if to.is_empty() { p.text(rel).ok() } else { crate::repository::read::file_at(p.folder, rel, &to) };
        if old.is_none() && new.is_none() {
            continue;
        }
        let (a, b) = (old.clone().unwrap_or_default(), new.clone().unwrap_or_default());
        if a.replace('\r', "") == b.replace('\r', "") {
            continue;
        }
        changed += 1;
        let head = match (&old, &new) {
            (None, _) => format!("{rel} (new)"),
            (_, None) => format!("{rel} (removed)"),
            _ => rel.clone(),
        };
        out.push(format!("--- {head}"));
        for l in crate::repository::read::diff(&a, &b) {
            if lines >= DIFF_LINES {
                cut = true;
                break;
            }
            out.push(l);
            lines += 1;
        }
    }
    let side = if to.is_empty() { "now".to_string() } else { to.clone() };
    if changed == 0 {
        return ok(format!("No difference between {from} and {side}{}.", only.as_ref().map(|r| format!(" in {r}")).unwrap_or_default()), "Compared versions: no difference".into(), only.as_deref().unwrap_or(""));
    }
    if cut {
        out.push(format!("(… cut at {DIFF_LINES} lines: compare one file with path)"));
    }
    let short = |id: &str| id.chars().take(7).collect::<String>();
    ok(format!("Changes from {} to {side} ({changed} files; - before, + after):\n{}", short(&from), out.join("\n")), format!("Compared {} with {side}", short(&from)), only.as_deref().unwrap_or(""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

    fn repo_examples() {
        set_examples_folder(&format!("{}/../../examples", env!("CARGO_MANIFEST_DIR")));
    }

    #[test]
    fn an_example_is_found_by_its_file_name_and_by_what_it_uses() {
        repo_examples();
        let o = examples_answer("rai_models").expect("found");
        assert!(o.content.contains("examples/ai/rai_models.rr"), "{}", o.content);
        let o = examples_answer("rai_models.rr").expect("found");
        assert!(o.content.contains("examples/ai/rai_models.rr"), "{}", o.content);
        let o = examples_answer("RJson").expect("found");
        assert!(o.content.contains("rapidr_describe {\"type\": \"RJson\"}"), "{}", o.content);
        for f in ["examples/data/json.rr", "examples/data/json_source.rr", "examples/network/http_json.rr"] {
            assert!(o.content.contains(f), "{f}: {}", o.content);
        }
        // (a classic name finds RapidR's)
        let o = examples_answer("QTIMER").expect("found");
        assert!(o.content.contains("RTimer"), "{}", &o.content[..200.min(o.content.len())]);
        assert!(examples_answer("zzqx qqvv").is_none());
    }

    #[test]
    fn how_do_i_gives_the_workflow_with_the_tools() {
        let h = |t: &str| how_do_i(&json!({"task": t})).content;
        assert!(h("change code").contains("save_version") && h("change code").contains("propose_edit") && h("change code").contains("check_code"));
        assert!(h("debug a crash").contains("debug {\"action\": \"state\"}"));
        assert!(h("how do I go back to an older version").contains("restore_version"));
        assert!(h("git commit").contains("never git"), "{}", h("git commit"));
        assert!(h("learn RJson").contains("rapidr_describe"));
        assert!(h("add a dialog window").contains("$INCLUDE"));
        assert!(h("qwerty").contains("No workflow"));
    }

    #[test]
    fn a_gap_is_written_to_the_projects_issues_file_only() {
        let dir = std::env::temp_dir().join(format!("rapidr-gap-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let folder = crate::slashes(&dir.to_string_lossy());
        let files = vec!["main.rr".to_string()];
        let buffers = HashMap::new();
        let p = Project { folder: &folder, files: &files, main: "main.rr", compat: "", buffers: &buffers };
        let o = report_gap(&json!({"what": "No tool to rename a file\n# not a heading <b>", "kind": "missing_tool", "tried": "how_do_i", "_caller": "RAI"}), &p);
        assert!(!o.is_error, "{}", o.content);
        let o2 = report_gap(&json!({"what": "search_manual found nothing on MIDI clocks", "kind": "docs"}), &p);
        assert!(o2.content.contains("entry 2"), "{}", o2.content);
        let text = std::fs::read_to_string(dir.join(ISSUES_FILE)).unwrap();
        assert!(text.starts_with("# RapidR issues"), "{text}");
        assert!(text.contains("a missing tool (from RAI): No tool to rename a file / not a heading ‹b›. Tried: how_do_i."), "{text}");
        assert!(text.contains("the docs didn't answer (from an AI app)"), "{text}");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1, "only that file");
        assert!(report_gap(&json!({"what": ""}), &p).is_error);
        let none = Project { folder: "", files: &files, main: "", compat: "", buffers: &buffers };
        assert!(report_gap(&json!({"what": "x"}), &none).is_error);
        // (a link in its place: never followed)
        #[cfg(unix)]
        {
            std::fs::remove_file(dir.join(ISSUES_FILE)).unwrap();
            let outside = std::env::temp_dir().join(format!("rapidr-gap-outside-{}", std::process::id()));
            std::fs::write(&outside, "x").unwrap();
            std::os::unix::fs::symlink(&outside, dir.join(ISSUES_FILE)).unwrap();
            assert!(report_gap(&json!({"what": "y"}), &p).is_error);
            assert_eq!(std::fs::read_to_string(&outside).unwrap(), "x");
            let _ = std::fs::remove_file(&outside);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn versions_are_listed_and_compared() {
        let dir = std::env::temp_dir().join(format!("rapidr-tool-versions-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("main.rr"), "PRINT \"one\"\n").unwrap();
        let folder = crate::slashes(&dir.to_string_lossy());
        let files = vec!["main.rr".to_string()];
        let mut buffers = HashMap::new();
        let p = Project { folder: &folder, files: &files, main: "main.rr", compat: "", buffers: &buffers };
        let o = list_versions(&json!({}), &p);
        assert!(o.content.contains("doesn't keep versions yet") && o.content.contains("start_versions"), "{}", o.content);
        let id = crate::repository::read::start_and_save(&folder, "First").expect("saved");
        let o = list_versions(&json!({}), &p);
        assert!(o.content.contains(&id[..7]) && o.content.contains("First") && o.content.contains("Nothing changed"), "{}", o.content);
        // (the editor's unsaved text counts as now)
        buffers.insert("main.rr".to_string(), "PRINT \"two\"\n".to_string());
        let p = Project { folder: &folder, files: &files, main: "main.rr", compat: "", buffers: &buffers };
        let o = compare_versions(&json!({}), &p);
        assert!(o.content.contains("-PRINT \"one\"") && o.content.contains("+PRINT \"two\""), "{}", o.content);
        let o = compare_versions(&json!({"path": "nope/../../x.rr"}), &p);
        assert!(o.is_error, "{}", o.content);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
