//! A headless replay of a conversation with RapidR Studio's assistant: the
//! same RAI and the same project tools Studio uses (RAIPROJECTTOOLS on a
//! project folder), against an OpenAI-compatible server (a local model) or
//! Anthropic — to see what a model does with them, turn by turn, without a
//! screen. Never part of the test suites: it talks to a real model.
//!
//!   cargo run --release -p rapidr-studio --example ai_eval -- SCRIPT --out DIR
//!
//! SCRIPT: one user turn a line (`#` lines and blank lines skipped).
//! Environment:
//!   RAPIDR_AI_EVAL_PROVIDER  compatible (default) or anthropic
//!   RAPIDR_AI_EVAL_URL       the server (default http://127.0.0.1:11234/v1)
//!   RAPIDR_AI_EVAL_MODEL     the model (required)
//!   RAPIDR_AI_EVAL_KEY       a key, when the server needs one (kept in memory, never printed)
//!   RAPIDR_AI_EVAL_WORKSPACE Anthropic's workspace id, when the key needs one
//!   RAPIDR_AI_EVAL_MAXTOKENS the reply cap Studio's settings give (default 65535, as a user may set)
//!
//! After each turn the proposals are applied as if the user pressed Apply
//! (a new program created and opened), and a line says what happened: the
//! tools called (ok / error), the prompt and reply tokens of each request,
//! whether the answer claims a change no tool made, and the status Studio
//! shows. At the end every source file is checked (check_code) and the
//! files are kept in DIR/final. The audit log goes to DIR/audit.jsonl.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rapidr_value::Value;
use serde_json::{json, Value as Json};

#[derive(Clone, Copy)]
struct Quiet;

thread_local! {
    static EVENTS: RefCell<Vec<(String, String)>> = const { RefCell::new(Vec::new()) };
}

impl rapidr_ai::rai::Fire for Quiet {
    fn fire(self, _name: &str, event: &str, args: &[Value]) {
        EVENTS.with(|e| e.borrow_mut().push((event.to_string(), args.iter().map(Value::to_string_val).collect::<Vec<_>>().join("|"))));
    }
}

const AI: &str = "evalai";
const TOOLS: &str = "evaltools";

fn s(v: impl Into<String>) -> Value {
    Value::String(v.into())
}

fn ai_set(p: &str, v: &str) {
    rapidr_ai::rai::set(AI, p, &s(v));
}

fn ai_get(p: &str) -> String {
    rapidr_ai::rai::get(AI, p).map(|v| v.to_string_val()).unwrap_or_default()
}

fn ai_call(m: &str, args: &[Value]) -> Value {
    rapidr_ai::rai::call(AI, m, args, |v| v.to_string_val()).unwrap_or(Value::Null)
}

fn tools_set(p: &str, v: &str) {
    rapidr_studio::ai::set(TOOLS, p, &s(v));
}

fn tools_call(m: &str, args: &[Value]) -> String {
    rapidr_studio::ai::call(TOOLS, m, args).map(|v| v.to_string_val()).unwrap_or_default()
}

/// The project open "in Studio".
#[derive(Default)]
struct Open {
    folder: String,
    name: String,
    main: String,
}

impl Open {
    fn files(&self) -> Vec<String> {
        let mut out = Vec::new();
        if !self.folder.is_empty() {
            walk(Path::new(&self.folder), Path::new(&self.folder), &mut out);
        }
        out.sort();
        out
    }

    fn sync(&self) {
        tools_set("projectfolder", &self.folder);
        tools_set("projectfiles", &self.files().join("\n"));
        tools_set("mainfile", &self.main);
        tools_set("projectname", &self.name);
        tools_call("clearfiles", &[]);
    }
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        if p.is_dir() {
            walk(root, &p, out);
        } else if let Ok(rel) = p.strip_prefix(root) {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

fn is_source(p: &str) -> bool {
    let ext = p.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    matches!(ext.as_str(), "rr" | "bas" | "inc")
}

fn audit_lines(path: &Path) -> Vec<Json> {
    std::fs::read_to_string(path).unwrap_or_default().lines().filter_map(|l| serde_json::from_str(l).ok()).collect()
}

fn one_line(t: &str, n: usize) -> String {
    let t: String = t.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.chars().count() > n { format!("{}…", t.chars().take(n).collect::<String>()) } else { t }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--score") {
        score::main_score(&args);
        return;
    }
    let mut script = None;
    let mut out = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--out" => {
                out = args.get(i + 1).cloned();
                i += 1;
            }
            a => script = Some(a.to_string()),
        }
        i += 1;
    }
    let (Some(script), Some(out)) = (script, out) else {
        eprintln!("usage: ai_eval SCRIPT --out DIR   (environment: RAPIDR_AI_EVAL_MODEL, RAPIDR_AI_EVAL_URL, RAPIDR_AI_EVAL_PROVIDER …)");
        std::process::exit(2);
    };
    let env = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
    let provider = env("RAPIDR_AI_EVAL_PROVIDER").unwrap_or_else(|| "compatible".into());
    let url = env("RAPIDR_AI_EVAL_URL").unwrap_or_else(|| "http://127.0.0.1:11234/v1".into());
    let Some(model) = env("RAPIDR_AI_EVAL_MODEL") else {
        eprintln!("RAPIDR_AI_EVAL_MODEL: which model?");
        std::process::exit(2);
    };
    let max_tokens = env("RAPIDR_AI_EVAL_MAXTOKENS").unwrap_or_else(|| "65535".into());
    let turns: Vec<String> = std::fs::read_to_string(&script).unwrap_or_else(|e| panic!("{script}: {e}")).lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')).map(str::to_string).collect();

    let out = PathBuf::from(out);
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    let out = out.canonicalize().unwrap();
    // (the run's own audit log and keys: never the user's)
    let audit = out.join("audit.jsonl");
    std::env::set_var("RAPIDR_AI_AUDIT", &audit);
    std::env::set_var("RAPIDR_AI_KEYSTORE", "memory");
    std::env::set_var("RAPIDR_REGISTRY", out.join("registry"));

    rapidr_studio::ai::install();
    tools_set("permission", "propose");
    ai_set("provider", &provider);
    if provider == "compatible" {
        ai_set("baseurl", &url);
    }
    if let Some(k) = env("RAPIDR_AI_EVAL_KEY") {
        ai_set("apikey", &k);
    }
    if let Some(w) = env("RAPIDR_AI_EVAL_WORKSPACE") {
        ai_set("workspaceid", &w);
    }
    ai_set("model", &model);
    ai_set("maxtokens", &max_tokens);
    ai_set("audit", "-1");
    if ai_call("attachcomponent", &[s(TOOLS), s("project")]).to_i64() == 0 {
        panic!("attach: {}", ai_get("error"));
    }
    let (system, tail) = rapidr_ai::rai::prompts_of(AI);
    println!("== {provider} {model} · {} turns · system prompt {} chars (+{} facts) · max tokens setting {max_tokens}", turns.len(), system.len(), tail.len());

    let mut open = Open::default();
    let mut summary = Vec::new();
    let mut false_claims = 0;
    let mut created = 0;
    for (k, turn) in turns.iter().enumerate() {
        open.sync();
        let seen = audit_lines(&audit).len();
        EVENTS.with(|e| e.borrow_mut().clear());
        let started = Instant::now();
        if ai_call("ask", &[s(turn.as_str())]).to_i64() == 0 {
            println!("T{} ask refused: {}", k + 1, ai_get("error"));
            continue;
        }
        // (each request's size as sent: "what was sent" is the request)
        let mut requests: Vec<usize> = Vec::new();
        let mut last = String::new();
        let mut request_at = Instant::now();
        let mut aborted = false;
        loop {
            rapidr_ai::rai::poll(Quiet);
            let lr = ai_get("lastrequest");
            if lr != last {
                // (the request's body — never its headers — kept beside the
                // run, to replay one by hand)
                let body = lr.split_once("\n\n").map_or("", |(_, b)| b);
                requests.push(serde_json::from_str::<Json>(body).map_or(body.len(), |v| v.to_string().len()));
                let _ = std::fs::write(out.join(format!("request-{:02}-{}.json", k + 1, requests.len())), body);
                last = lr;
                request_at = Instant::now();
            }
            if rapidr_ai::rai::get(AI, "busy").map(|v| v.to_i64()).unwrap_or(0) == 0 {
                rapidr_ai::rai::poll(Quiet);
                break;
            }
            if request_at.elapsed() > Duration::from_secs(300) {
                ai_call("cancel", &[]);
                aborted = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let secs = started.elapsed().as_secs_f64();
        let new: Vec<Json> = audit_lines(&audit).into_iter().skip(seen).collect();
        let tools: Vec<String> = new.iter().filter(|v| v["op"] == "tool").map(|v| format!("{}({})", v["tool"].as_str().unwrap_or(""), v["result"].as_str().unwrap_or(""))).collect();
        let usage: Vec<String> = new.iter().filter(|v| v["op"] == "reply").map(|v| format!("{}/{}", v["inputTokens"], v["outputTokens"])).collect();
        let max_sent: Vec<String> = new.iter().filter(|v| v["op"] == "request").map(|v| v["maxTokens"].to_string()).collect();
        let nudges = new.iter().filter(|v| v["op"] == "nudge").count();
        let errors: Vec<String> = new.iter().filter(|v| v["op"] == "error").map(|v| one_line(v["message"].as_str().unwrap_or(""), 160)).collect();
        let n = rapidr_studio::ai::get(TOOLS, "proposalcount").map(|v| v.to_i64()).unwrap_or(0);
        let infos: Vec<String> = (0..n).map(|i| tools_call("proposalinfo", &[Value::Integer(i)])).collect();
        let answer = ai_get("answer");
        let status = ai_get("status");
        let claim = n == 0 && rapidr_ai::claims::claims_change(&answer);
        if claim {
            false_claims += 1;
        }
        let (transcript, _) = rapidr_ai::rai::peek(AI);
        let _ = std::fs::write(out.join(format!("transcript-{:02}.md", k + 1)), &transcript);
        println!("T{} \"{}\" ({secs:.0}s{})", k + 1, one_line(turn, 60), if aborted { ", ABORTED after 5 min" } else { "" });
        println!("   tools: {}", if tools.is_empty() { "none".into() } else { tools.join(" ") });
        println!("   requests: {} · sent ≈{} tokens (chars/4) · usage in/out {} · maxTokens {}", requests.len(), requests.iter().map(|c| (c / 4).to_string()).collect::<Vec<_>>().join(","), if usage.is_empty() { "-".into() } else { usage.join(",") }, max_sent.first().cloned().unwrap_or_default());
        println!("   proposals: {} {} · claim without a change: {} · nudges: {nudges}", n, infos.iter().map(|i| { let f: Vec<&str> = i.split('\t').collect(); format!("[{} {}{}]", f.first().unwrap_or(&""), f.get(3).filter(|p| !p.is_empty()).map(|p| format!("{p}/")).unwrap_or_default(), f.get(1).unwrap_or(&"")) }).collect::<Vec<_>>().join(" "), if claim { "YES" } else { "no" });
        println!("   status: {status}");
        if !errors.is_empty() {
            println!("   errors: {}", errors.join(" | "));
        }
        println!("   reply: {}", one_line(&answer, 220));
        // (Apply, as the user would: a new program created and opened; a
        // change to a file written)
        let mut applied = Vec::new();
        let mut project_made = false;
        for i in 0..n {
            let info = tools_call("proposalinfo", &[Value::Integer(i)]);
            let f: Vec<&str> = info.split('\t').collect();
            let (kind, path) = (f.first().copied().unwrap_or(""), f.get(1).copied().unwrap_or(""));
            if kind == "project" {
                if project_made {
                    continue;
                }
                project_made = true;
                created += 1;
                let folder = out.join(format!("projects{created}"));
                std::fs::create_dir_all(&folder).unwrap();
                // (its main file: the proposal marked main; CreateProject
                // gives the project file it writes beside it)
                let main_rel = (0..n).map(|j| tools_call("proposalinfo", &[Value::Integer(j)])).find(|i| i.split('\t').nth(4) == Some("1")).and_then(|i| i.split('\t').nth(1).map(str::to_string)).unwrap_or_else(|| path.to_string());
                let made = tools_call("createproject", &[s(folder.to_string_lossy())]);
                if made.starts_with("error:") || made.is_empty() {
                    applied.push(format!("create failed: {made}"));
                    continue;
                }
                let root = PathBuf::from(made).parent().unwrap().to_path_buf();
                open = Open { folder: root.to_string_lossy().to_string(), name: root.file_name().unwrap().to_string_lossy().to_string(), main: main_rel };
                applied.push(format!("created {}", open.name));
            } else if !open.folder.is_empty() {
                let full = Path::new(&open.folder).join(path);
                if let Some(d) = full.parent() {
                    let _ = std::fs::create_dir_all(d);
                }
                std::fs::write(&full, tools_call("proposalnew", &[Value::Integer(i)])).unwrap();
                tools_call("decide", &[Value::Integer(i), s("applied")]);
                applied.push(format!("applied {path}"));
            }
        }
        if !applied.is_empty() {
            println!("   applied: {}", applied.join(", "));
        }
        summary.push(json!({"turn": k + 1, "user": turn, "secs": secs, "tools": tools, "requests": requests, "usage": usage, "maxTokens": max_sent, "proposals": infos, "claim": claim, "nudges": nudges, "status": status, "errors": errors, "reply": answer, "aborted": aborted}));
    }

    // (every source file checked as the tools check it; the files kept)
    println!("== final project: {}", if open.folder.is_empty() { "none".into() } else { format!("{} (main {})", open.name, open.main) });
    let mut checks = Vec::new();
    if !open.folder.is_empty() {
        open.sync();
        let files = open.files();
        let buffers = std::collections::HashMap::new();
        let p = rapidr_studio::ai_tools::Project { folder: &open.folder, files: &files, main: &open.main, compat: "", buffers: &buffers };
        let final_dir = out.join("final");
        for f in &files {
            let text = std::fs::read_to_string(Path::new(&open.folder).join(f)).unwrap_or_default();
            let dest = final_dir.join(f);
            let _ = std::fs::create_dir_all(dest.parent().unwrap());
            let _ = std::fs::write(&dest, &text);
            if !is_source(f) {
                continue;
            }
            let mut props = Vec::new();
            let r = rapidr_studio::ai_tools::run("check_code", &json!({"path": f}), &p, &mut props, false);
            let first: Vec<&str> = r.content.lines().take(6).collect();
            println!("   {f} ({} lines): {}", text.lines().count(), first.join(" | "));
            checks.push(json!({"file": f, "lines": text.lines().count(), "check": r.content}));
        }
    }
    println!("== false claims: {false_claims}");
    // (the click test: the program run as a user would use it)
    let tests = score::parse(&std::fs::read_to_string(&script).unwrap_or_default());
    let result = if open.folder.is_empty() { score::Score { compiles: false, tests: Vec::new() } } else { score::score(Path::new(&open.folder), &open.main, &tests, &out.join("click")) };
    score::print(&result);
    let _ = std::fs::write(out.join("summary.json"), serde_json::to_string_pretty(&json!({"provider": provider, "model": model, "turns": summary, "falseClaims": false_claims, "checks": checks, "compiles": result.compiles, "clickTests": result.tests.iter().map(|(n, ok, why)| json!({"test": n, "pass": ok, "why": why})).collect::<Vec<_>>(), "score": result.pass()})).unwrap());
}

/// The click tests of tests/ai-eval/apps: a script's `#!` lines say what a
/// user does with the program and what they should see; the components are
/// found by what they show (a button's caption, the label beside an edit),
/// since a model names them as it likes. Score = it compiles and every test
/// passes.
mod score {
    use std::path::Path;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    use regex::Regex;
    use rapidr_introspect::{Reply, Request};
    use serde_json::{json, Value as Json};

    #[derive(Debug)]
    enum Step {
        Click { sel: String, optional: bool },
        Type { sel: String, text: String, optional: bool },
        Wait(usize),
        Pick { sel: String, index: i64 },
        Drag { sel: String, from: (i64, i64), to: (i64, i64) },
        ClickAll,
        Baseline,
    }

    #[derive(Debug)]
    enum Expect {
        Text(String),
        Changed(Option<String>),
        Count(Option<i64>),
        Picture,
    }

    #[derive(Debug, Default)]
    pub struct Test {
        name: String,
        steps: Vec<Step>,
        expects: Vec<Expect>,
        file_dialog: Option<String>,
    }

    pub struct Score {
        pub compiles: bool,
        pub tests: Vec<(String, bool, String)>,
    }

    impl Score {
        pub fn pass(&self) -> bool {
            self.compiles && !self.tests.is_empty() && self.tests.iter().all(|t| t.1)
        }
    }

    /// A selector's regex (`/…/`, any case), or None for a plain caption.
    fn regex_of(sel: &str) -> Option<Regex> {
        let inner = sel.strip_prefix('/')?.strip_suffix('/')?;
        Regex::new(&format!("(?i){inner}")).ok()
    }

    /// `#!` lines → tests.
    pub fn parse(script: &str) -> Vec<Test> {
        let mut tests: Vec<Test> = Vec::new();
        for l in script.lines() {
            let Some(l) = l.trim().strip_prefix("#!") else { continue };
            let l = l.trim();
            let (cmd, rest) = l.split_once(' ').map_or((l, ""), |(c, r)| (c, r.trim()));
            if cmd == "test" {
                tests.push(Test { name: rest.to_string(), ..Test::default() });
                continue;
            }
            let Some(t) = tests.last_mut() else { continue };
            // (a selector: /regex/ (may hold spaces) or one word)
            let split_sel = |r: &str| -> (String, String) {
                if let Some(rest) = r.strip_prefix('/') {
                    if let Some(end) = rest.find('/') {
                        return (r[..end + 2].to_string(), r[end + 2..].trim().to_string());
                    }
                }
                r.split_once(' ').map_or((r.to_string(), String::new()), |(a, b)| (a.to_string(), b.trim().to_string()))
            };
            match cmd {
                "click" | "click?" => t.steps.push(Step::Click { sel: rest.to_string(), optional: cmd.ends_with('?') }),
                "type" | "type?" => {
                    let (sel, text) = split_sel(rest);
                    t.steps.push(Step::Type { sel, text, optional: cmd.ends_with('?') });
                }
                "wait" => t.steps.push(Step::Wait(rest.parse().unwrap_or(10))),
                "pick" => {
                    let (sel, i) = split_sel(rest);
                    t.steps.push(Step::Pick { sel, index: i.parse().unwrap_or(0) });
                }
                "drag" => {
                    let (sel, nums) = split_sel(rest);
                    let n: Vec<i64> = nums.split_whitespace().filter_map(|x| x.parse().ok()).collect();
                    if n.len() == 4 {
                        t.steps.push(Step::Drag { sel, from: (n[0], n[1]), to: (n[2], n[3]) });
                    }
                }
                "click-all" => t.steps.push(Step::ClickAll),
                "baseline" => t.steps.push(Step::Baseline),
                "file-dialog" => t.file_dialog = Some(rest.to_string()),
                "expect" => t.expects.push(Expect::Text(rest.to_string())),
                "expect-changed" => t.expects.push(Expect::Changed((!rest.is_empty()).then(|| rest.to_string()))),
                "expect-count" => t.expects.push(Expect::Count(rest.strip_prefix('+').and_then(|n| n.parse().ok()))),
                "expect-picture-changed" => t.expects.push(Expect::Picture),
                _ => {}
            }
        }
        tests
    }

    /// A component as the source creates it.
    #[derive(Debug, Clone)]
    struct Comp {
        id: String,
        ty: String,
        caption: String,
        left: i64,
        top: i64,
    }

    impl Comp {
        fn key(&self) -> String {
            self.id.to_lowercase()
        }
        fn is(&self, kinds: &[&str]) -> bool {
            let t = self.ty.to_ascii_uppercase();
            kinds.iter().any(|k| t.ends_with(k))
        }
        fn button(&self) -> bool {
            self.is(&["BUTTON", "BITBTN", "SPEEDBUTTON"]) && !self.is(&["RADIOBUTTON"])
        }
        fn edit(&self) -> bool {
            self.is(&["EDIT", "MEMO", "SPINEDIT", "COMBOBOX", "RICHEDIT", "MASKEDIT"])
        }
        fn list(&self) -> bool {
            self.is(&["LISTBOX", "LISTVIEW", "COMBOBOX", "STRINGGRID", "CHECKLISTBOX"])
        }
        fn canvas(&self) -> bool {
            self.is(&["CANVAS", "PAINTBOX", "IMAGE"])
        }
        fn form(&self) -> bool {
            self.is(&["FORM"])
        }
    }

    fn clean(caption: &str) -> String {
        caption.replace('&', "").trim().to_string()
    }

    /// The components of the program (`main` and the includes beside it).
    fn components(folder: &Path, main: &str) -> Vec<Comp> {
        let mut files: Vec<(String, String)> = vec![(main.to_string(), std::fs::read_to_string(folder.join(main)).unwrap_or_default())];
        for e in std::fs::read_dir(folder).into_iter().flatten().flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if n.to_ascii_lowercase().ends_with(".inc") {
                files.push((n.clone(), std::fs::read_to_string(e.path()).unwrap_or_default()));
            }
        }
        let refs: Vec<(&str, &str)> = files.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
        let model = rapidr_introspect::design::DesignModel::in_memory(&refs);
        let Reply::Ok(list) = rapidr_introspect::handle(Some(&model), &Request::List { scope: None, type_name: None, limit: Some(1000), cursor: None }) else { return Vec::new() };
        let mut out = Vec::new();
        for o in list["objects"].as_array().into_iter().flatten() {
            let id = o["id"].as_str().unwrap_or("").to_string();
            let ty = o["type"].as_str().unwrap_or("").to_string();
            let props = ["Caption", "Left", "Top"].iter().map(|s| s.to_string()).collect::<Vec<_>>();
            let mut c = Comp { id: id.clone(), ty, caption: String::new(), left: 0, top: 0 };
            if let Reply::Ok(g) = rapidr_introspect::handle(Some(&model), &Request::Get { id, props: Some(props) }) {
                for v in g["values"].as_array().into_iter().flatten() {
                    let val = &v["value"];
                    match v["name"].as_str().unwrap_or("").to_ascii_lowercase().as_str() {
                        "caption" => c.caption = val.as_str().map_or_else(|| val.to_string(), str::to_string),
                        "left" => c.left = val.as_i64().unwrap_or(0),
                        "top" => c.top = val.as_i64().unwrap_or(0),
                        _ => {}
                    }
                }
            }
            out.push(c);
        }
        out
    }

    /// The component a step's selector names among `pool`.
    fn find<'a>(sel: &str, pool: &[&'a Comp], all: &'a [Comp], by_label: bool) -> Option<&'a Comp> {
        if let Some(n) = sel.strip_prefix('#').and_then(|n| n.parse::<usize>().ok()) {
            return pool.get(n.saturating_sub(1)).copied();
        }
        if sel.starts_with('#') {
            return pool.first().copied();
        }
        let matches = |text: &str| match regex_of(sel) {
            Some(re) => re.is_match(text),
            None => clean(text).eq_ignore_ascii_case(sel.trim()),
        };
        if let Some(c) = pool.iter().find(|c| !c.caption.is_empty() && matches(&c.caption)) {
            return Some(c);
        }
        if let Some(c) = pool.iter().find(|c| matches(&c.id)) {
            return Some(c);
        }
        if by_label {
            // (the edit nearest a label that says it: to its right, or under it)
            let label = all.iter().find(|c| c.is(&["LABEL"]) && matches(&c.caption))?;
            return pool.iter().filter(|c| c.top >= label.top - 12 && c.left >= label.left - 12).min_by_key(|c| (c.top - label.top).abs() * 3 + (c.left - label.left).abs()).copied();
        }
        None
    }

    /// Keys typed into `id` (a US keyboard), after Ctrl+A.
    fn type_keys(id: &str, text: &str) -> Vec<String> {
        let mut out = vec![format!("{id}.__key_65_16")];
        for c in text.chars() {
            let k = match c {
                'a'..='z' => (c.to_ascii_uppercase() as u32).to_string(),
                'A'..='Z' => format!("{}_256", c as u32),
                '0'..='9' | ' ' => (c as u32).to_string(),
                '.' => "190".into(),
                '-' => "189".into(),
                _ => continue,
            };
            out.push(format!("{id}.__key_{k}"));
        }
        out
    }

    struct Run {
        values: Vec<(String, String)>,
        picture: Vec<u8>,
        log: String,
    }

    fn rapidr() -> String {
        std::env::var("RAPIDR_AI_EVAL_RAPIDR").unwrap_or_else(|_| format!("{}/../../target/release/rapidr", env!("CARGO_MANIFEST_DIR")))
    }

    fn run(folder: &Path, main: &str, events: &[String], dump: &[String], file_dialog: Option<&str>, capture: &Path) -> Run {
        let _ = std::fs::create_dir_all(capture.parent().unwrap());
        let mut cmd = Command::new(rapidr());
        cmd.arg("run").arg(main).current_dir(folder);
        let reg = capture.parent().unwrap().join("registry");
        let print = capture.parent().unwrap().join("print");
        let _ = std::fs::create_dir_all(&print);
        cmd.env("RAPIDR_CAPTURE", capture).env("RAPIDR_CAPTURE_DELAY", "0.4").env("RAPIDR_TEST_EVENTS", events.join(",")).env("RAPIDR_TEST_DUMP", dump.join(","));
        cmd.env("RAPIDR_TEST_MESSAGE_DIALOG", "").env("RAPIDR_TEST_FILE_DIALOG", file_dialog.unwrap_or("")).env("RAPIDR_REGISTRY", reg).env("RAPIDR_PRINT_TO", print).env("RAPIDR_AI_KEYSTORE", "memory");
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(Stdio::null());
        let Ok(mut child) = cmd.spawn() else { return Run { values: Vec::new(), picture: Vec::new(), log: format!("couldn't start {}", rapidr()) } };
        let started = Instant::now();
        loop {
            if let Ok(Some(_)) = child.try_wait() {
                break;
            }
            if started.elapsed() > Duration::from_secs(40) {
                let _ = child.kill();
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let out = child.wait_with_output().map(|o| format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))).unwrap_or_default();
        // (the dump: `key=value` lines in order; a value may run over lines)
        let mut values: Vec<(String, String)> = Vec::new();
        for line in out.lines() {
            if let Some(k) = dump.iter().find(|k| line.starts_with(&format!("{k}="))) {
                values.push((k.clone(), line[k.len() + 1..].to_string()));
            } else if let Some(last) = values.last_mut().filter(|_| !line.starts_with("[rapidr]") && !line.starts_with("[WARN]")) {
                last.1.push('\n');
                last.1.push_str(line);
            }
        }
        let picture = std::fs::read(format!("{}-1.bmp", capture.display())).unwrap_or_default();
        Run { values, picture, log: out }
    }

    /// The program in `folder` (`main`) compiled, and each test played.
    pub fn score(folder: &Path, main: &str, tests: &[Test], work: &Path) -> Score {
        let files = vec![main.to_string()];
        let mut all_files: Vec<String> = files.clone();
        for e in std::fs::read_dir(folder).into_iter().flatten().flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if n.to_ascii_lowercase().ends_with(".inc") {
                all_files.push(n);
            }
        }
        // (compiles: the compiler's own errors only — Studio's extra checks
        // for the assistant aren't the measure; the click test is)
        let mut a = rapidr_langsvc::Analysis::new(rapidr_langsvc::Options { roots: Some(Vec::new()), ..rapidr_langsvc::Options::default() });
        for f in &all_files {
            a.update(folder.join(f), std::fs::read_to_string(folder.join(f)).unwrap_or_default());
        }
        let errors: Vec<String> = a.diagnostics(&folder.join(main)).into_iter().filter(|d| d.severity == rapidr_langsvc::Severity::Error).map(|d| d.message).collect();
        let compiles = errors.is_empty();
        let mut results = Vec::new();
        if !compiles {
            return Score { compiles, tests: vec![("compile".into(), false, errors.first().cloned().unwrap_or_default().chars().take(160).collect())] };
        }
        let comps = components(folder, main);
        let form = comps.iter().find(|c| c.form()).map(Comp::key).unwrap_or_else(|| "form".into());
        for (k, t) in tests.iter().enumerate() {
            let buttons: Vec<&Comp> = comps.iter().filter(|c| c.button()).collect();
            let captioned: Vec<&Comp> = comps.iter().filter(|c| !c.caption.is_empty() && !c.form() && !c.is(&["LABEL"])).collect();
            let edits: Vec<&Comp> = comps.iter().filter(|c| c.edit()).collect();
            let lists: Vec<&Comp> = comps.iter().filter(|c| c.list()).collect();
            let canvases: Vec<&Comp> = comps.iter().filter(|c| c.canvas()).collect();
            let mut events: Vec<String> = Vec::new();
            let mut base_events: Option<usize> = None;
            let mut typed: Vec<String> = Vec::new();
            let mut problem = String::new();
            for st in &t.steps {
                match st {
                    Step::Click { sel, optional } => match find(sel, &buttons, &comps, false).or_else(|| find(sel, &captioned, &comps, false)) {
                        Some(c) => events.push(format!("{}.onclick", c.key())),
                        None if *optional => {}
                        None => problem = format!("no button {sel}"),
                    },
                    Step::Type { sel, text, optional } => match find(sel, &edits, &comps, true).or_else(|| (!optional).then(|| edits.iter().find(|c| !typed.contains(&c.key())).copied()).flatten()) {
                        Some(c) => {
                            typed.push(c.key());
                            events.extend(type_keys(&c.key(), text));
                        }
                        None if *optional => {}
                        None => problem = format!("no edit {sel}"),
                    },
                    Step::Wait(n) => events.extend((0..*n).map(|_| format!("{form}.onwait"))),
                    Step::Pick { sel, index } => match find(sel, &lists, &comps, false) {
                        Some(c) => events.push(format!("{}.__item_{index}", c.key())),
                        None => problem = format!("no list {sel}"),
                    },
                    Step::Drag { sel, from, to } => {
                        let target = find(sel, &canvases, &comps, false).map(Comp::key).unwrap_or_else(|| form.clone());
                        events.push(format!("{target}.__mousedown_{}_{}", from.0, from.1));
                        for s in 1..=4 {
                            events.push(format!("{target}.__mousemove_{}_{}", from.0 + (to.0 - from.0) * s / 4, from.1 + (to.1 - from.1) * s / 4));
                        }
                        events.push(format!("{target}.__mouseup_{}_{}", to.0, to.1));
                    }
                    Step::ClickAll => {
                        let skip = Regex::new("(?i)quit|exit|close|cancel|again|reset|restart|new game").unwrap();
                        events.extend(buttons.iter().filter(|c| !skip.is_match(&c.caption)).map(|c| format!("{}.onclick", c.key())));
                    }
                    Step::Baseline => base_events = Some(events.len()),
                }
            }
            if !problem.is_empty() {
                results.push((t.name.clone(), false, problem));
                continue;
            }
            let mut dump: Vec<String> = Vec::new();
            for c in &comps {
                if !c.is(&["TIMER", "DIALOG", "MENU", "MENUITEM"]) {
                    dump.push(format!("{}.caption", c.key()));
                    dump.push(format!("{}.text", c.key()));
                }
                if c.list() {
                    dump.push(format!("{}.itemcount", c.key()));
                    if c.is(&["STRINGGRID"]) {
                        dump.push(format!("{}.rowcount", c.key()));
                    }
                }
                if c.is(&["STATUSBAR"]) {
                    dump.push(format!("{}.simpletext", c.key()));
                }
            }
            let dialog = t.file_dialog.as_ref().map(|f| {
                let path = folder.join(f);
                if !path.exists() {
                    let _ = std::fs::write(&path, "Name,City,Age\nAda,London,36\nGrace,New York,45\nLinus,Helsinki,28\n");
                }
                path.to_string_lossy().to_string()
            });
            let dir = work.join(format!("{}-{}", k + 1, t.name));
            let base = run(folder, main, &events[..base_events.unwrap_or(0)], &dump, dialog.as_deref(), &dir.join("base"));
            let full = run(folder, main, &events, &dump, dialog.as_deref(), &dir.join("full"));
            let _ = std::fs::write(dir.join("events.txt"), events.join("\n"));
            let _ = std::fs::write(dir.join("full.log"), &full.log);
            let shown = |r: &Run| -> Vec<(String, String)> { r.values.iter().filter(|(k, _)| !typed.iter().any(|t| k.starts_with(&format!("{t}.")))).cloned().collect() };
            let (b, f) = (shown(&base), shown(&full));
            let mut why = Vec::new();
            for e in &t.expects {
                let ok = match e {
                    Expect::Text(re) => {
                        let re = regex_of(re).unwrap_or_else(|| Regex::new(&regex::escape(re)).unwrap());
                        f.iter().any(|(k, v)| (k.ends_with(".caption") || k.ends_with(".text") || k.ends_with(".simpletext")) && re.is_match(v))
                    }
                    Expect::Changed(re) => {
                        let re = re.as_deref().and_then(regex_of);
                        let changed = f.iter().any(|(k, v)| b.iter().find(|(bk, _)| bk == k).is_some_and(|(_, bv)| bv != v) && re.as_ref().is_none_or(|re| re.is_match(v)));
                        changed || (re.is_none() && !full.picture.is_empty() && base.picture != full.picture)
                    }
                    Expect::Count(n) => f.iter().filter(|(k, _)| k.ends_with(".itemcount") || k.ends_with(".rowcount")).any(|(k, v)| {
                        let before: i64 = b.iter().find(|(bk, _)| bk == k).and_then(|(_, bv)| bv.trim().parse().ok()).unwrap_or(-1);
                        let now: i64 = v.trim().parse().unwrap_or(-1);
                        match n {
                            Some(n) => now == before + n,
                            None => now != before && now >= 0,
                        }
                    }),
                    Expect::Picture => !full.picture.is_empty() && base.picture != full.picture,
                };
                if !ok {
                    why.push(format!("{e:?} failed"));
                }
            }
            if full.values.is_empty() {
                why.push(format!("the program didn't run to the end: {}", full.log.lines().rev().take(3).collect::<Vec<_>>().join(" | ").chars().take(200).collect::<String>()));
            }
            results.push((t.name.clone(), why.is_empty(), why.join("; ")));
        }
        Score { compiles, tests: results }
    }

    pub fn print(s: &Score) {
        println!("== click test: compiles {} · {}", if s.compiles { "yes" } else { "no" }, s.tests.iter().map(|(n, ok, why)| format!("{n} {}{}", if *ok { "PASS" } else { "FAIL" }, if why.is_empty() { String::new() } else { format!(" ({why})") })).collect::<Vec<_>>().join(" · "));
        println!("== SCORE {}", if s.pass() { 1 } else { 0 });
    }

    /// `--score SCRIPT --project DIR [--main FILE] [--out DIR]`: an existing
    /// program scored (a run made earlier, a recipe).
    pub fn main_score(args: &[String]) {
        let get = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned();
        let (Some(script), Some(project)) = (get("--score"), get("--project")) else {
            eprintln!("usage: ai_eval --score SCRIPT --project DIR [--main main.rr] [--out DIR]");
            std::process::exit(2);
        };
        let main = get("--main").unwrap_or_else(|| "main.rr".into());
        let out = get("--out").map_or_else(|| std::env::temp_dir().join("rapidr-ai-score"), std::path::PathBuf::from);
        let _ = std::fs::remove_dir_all(&out);
        let tests = parse(&std::fs::read_to_string(&script).unwrap_or_default());
        let s = score(Path::new(&project), &main, &tests, &out);
        print(&s);
        let _ = std::fs::write(out.join("score.json"), json!({"compiles": s.compiles, "tests": s.tests.iter().map(|(n, ok, why)| json!({"test": n, "pass": ok, "why": why})).collect::<Vec<Json>>(), "score": s.pass()}).to_string());
    }
}
