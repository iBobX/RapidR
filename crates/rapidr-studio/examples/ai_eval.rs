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
                let main = tools_call("createproject", &[s(folder.to_string_lossy())]);
                if main.starts_with("error:") || main.is_empty() {
                    applied.push(format!("create failed: {main}"));
                    continue;
                }
                let main = PathBuf::from(main);
                let root = main.parent().unwrap().to_path_buf();
                open = Open { folder: root.to_string_lossy().to_string(), name: root.file_name().unwrap().to_string_lossy().to_string(), main: main.file_name().unwrap().to_string_lossy().to_string() };
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
    let _ = std::fs::write(out.join("summary.json"), serde_json::to_string_pretty(&json!({"provider": provider, "model": model, "turns": summary, "falseClaims": false_claims, "checks": checks})).unwrap());
}
