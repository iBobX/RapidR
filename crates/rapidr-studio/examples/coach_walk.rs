//! Coach mode used as a student uses it, against a real model (the COACH-MODE
//! lane's walk; docs/manual/ai-assistant.md "Coach mode"): the same RAI and
//! project tools Studio uses, the dial on Coach, three small problems — a
//! button that counts clicks, a FOR loop that sums 1 to 10, an endless loop
//! to fix — each climbed hint 1 → 4 (a question, then Next Hint three
//! times, as the strip sends it). Never part of the test suites: it talks
//! to a real model.
//!
//!   cargo run --release -p rapidr-studio --example coach_walk -- OUT.md
//!
//! Environment: RAPIDR_COACH_URL (default http://127.0.0.1:11434/v1, a
//! local AI app), RAPIDR_COACH_MODEL (required), RAPIDR_COACH_SMALL (1: the
//! small-model mode, as Set Up RAI chooses it for a small local model),
//! RAPIDR_COACH_PROVIDER (compatible, or anthropic with RAPIDR_COACH_KEY —
//! kept in memory, never printed). Each answer is written to OUT.md with
//! the tools it called and whether anything was proposed (never, in Coach).

use std::time::{Duration, Instant};

use rapidr_value::Value;

#[derive(Clone, Copy)]
struct Quiet;

impl rapidr_ai::rai::Fire for Quiet {
    fn fire(self, _name: &str, _event: &str, _args: &[Value]) {}
}

const AI: &str = "coachai";
const TOOLS: &str = "coachtools";

fn s(v: impl Into<String>) -> Value {
    Value::String(v.into().into())
}

fn env(k: &str) -> Option<String> {
    std::env::var(k).ok().filter(|v| !v.is_empty())
}

/// The three problems: a project to start from, and the student's question.
const TASKS: &[(&str, &str, &str)] = &[
    (
        "counter",
        "DECLARE SUB CountClick\n\nCREATE Form AS RForm\n    Caption = \"Counter\"\n    Width = 240: Height = 140\n    CREATE Label1 AS RLabel\n        Caption = \"0\"\n        Left = 20: Top = 20\n    END CREATE\n    CREATE Button1 AS RButton\n        Caption = \"Count\"\n        Left = 20: Top = 50\n        OnClick = CountClick\n    END CREATE\nEND CREATE\n\nSUB CountClick\n    ' (what goes here?)\nEND SUB\n\nForm.ShowModal\n",
        "I want the label to count how many times I click the button. How do I do it?",
    ),
    ("sum", "' Add the numbers from 1 to 10 and print the total\n\n", "How do I add up the numbers 1 to 10 with a FOR loop?"),
    (
        "loop",
        "DIM x AS INTEGER\nx = 1\nDO WHILE x < 10\n    PRINT x\nLOOP\nPRINT \"done\"\n",
        "My program never stops: it prints 1 forever. Why?",
    ),
];

fn main() {
    let out_md = std::env::args().nth(1).unwrap_or_else(|| "coach-walk.md".into());
    let model = env("RAPIDR_COACH_MODEL").expect("RAPIDR_COACH_MODEL");
    let provider = env("RAPIDR_COACH_PROVIDER").unwrap_or_else(|| "compatible".into());
    let url = env("RAPIDR_COACH_URL").unwrap_or_else(|| "http://127.0.0.1:11434/v1".into());
    let small = env("RAPIDR_COACH_SMALL").as_deref() == Some("1");
    let work = std::env::temp_dir().join(format!("rapidr-coach-walk-{}", std::process::id()));
    std::fs::create_dir_all(&work).unwrap();
    // (the run's own audit log, keys and settings: never the user's)
    std::env::set_var("RAPIDR_AI_AUDIT", work.join("audit.jsonl"));
    std::env::set_var("RAPIDR_AI_KEYSTORE", "memory");
    std::env::set_var("RAPIDR_REGISTRY", work.join("registry"));
    rapidr_studio::ai::install();
    let mut md = format!("# Coach mode with a real model\n\nModel: `{model}` ({provider}{}).\n\n", if small { ", small-model mode" } else { "" });
    for (k, (name, code, question)) in TASKS.iter().enumerate() {
        let dir = work.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("main.rr"), code).unwrap();
        let ai = format!("{AI}{k}");
        let tools = format!("{TOOLS}{k}");
        let set_t = |p: &str, v: &str| rapidr_studio::ai::set(&tools, p, &s(v));
        set_t("projectfolder", &dir.to_string_lossy());
        set_t("projectfiles", "main.rr");
        set_t("mainfile", "main.rr");
        set_t("permission", "propose");
        set_t("mode", "coach");
        let set_a = |p: &str, v: &str| rapidr_ai::rai::set(&ai, p, &s(v));
        set_a("provider", &provider);
        if provider == "compatible" {
            set_a("baseurl", &url);
        }
        if let Some(key) = env("RAPIDR_COACH_KEY") {
            set_a("apikey", &key);
        }
        set_a("model", &model);
        set_a("maxtokens", "2048");
        set_a("audit", "-1");
        set_a("replylanguage", "auto");
        set_a("smallmodel", if small { "1" } else { "0" });
        set_a("insertlinks", "0");
        let call = |m: &str, args: &[Value]| rapidr_ai::rai::call(&ai, m, args, |v| v.to_string_val()).map(|v| v.to_i64()).unwrap_or(0);
        assert_ne!(call("attachcomponent", &[s(tools.as_str()), s("project")]), 0, "attach");
        md.push_str(&format!("## {}. {question}\n\n```rapidr\n{}```\n\n", k + 1, code));
        for rung in 1..=4u8 {
            let ask = if rung == 1 {
                rapidr_studio::ai::call(&tools, "coachask", &[]);
                question.to_string()
            } else {
                rapidr_studio::ai::call(&tools, "coachnext", &[]);
                if rung == 4 { "Show me the code.".to_string() } else { "Give me the next hint.".to_string() }
            };
            let started = Instant::now();
            assert_ne!(call("ask", &[s(ask.as_str())]), 0, "ask: {}", rapidr_ai::rai::get(&ai, "error").map(|v| v.to_string_val()).unwrap_or_default());
            loop {
                rapidr_ai::rai::poll(Quiet);
                if rapidr_ai::rai::get(&ai, "busy").map(|v| v.to_i64()).unwrap_or(0) == 0 {
                    rapidr_ai::rai::poll(Quiet);
                    break;
                }
                if started.elapsed() > Duration::from_secs(300) {
                    call("cancel", &[]);
                    break;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            let answer = rapidr_ai::rai::get(&ai, "answer").map(|v| v.to_string_val()).unwrap_or_default();
            let steps = rapidr_ai::rai::get(&ai, "steps").map(|v| v.to_string_val()).unwrap_or_default().replace('\n', "; ");
            let proposed = rapidr_studio::ai::get(&tools, "proposalcount").map(|v| v.to_i64()).unwrap_or(0);
            let unchanged = std::fs::read_to_string(dir.join("main.rr")).ok().as_deref() == Some(*code);
            println!("{name} hint {rung}: {:.1} s, steps [{steps}], proposals {proposed}, file unchanged {unchanged}", started.elapsed().as_secs_f64());
            md.push_str(&format!(
                "### Hint {rung} of 4 — *{ask}* ({:.1} s{}{})\n\n{}\n\n",
                started.elapsed().as_secs_f64(),
                if steps.is_empty() { String::new() } else { format!("; tools: {steps}") },
                if proposed == 0 && unchanged { "; nothing proposed, file unchanged" } else { "; **SOMETHING PROPOSED OR CHANGED**" },
                answer.trim()
            ));
        }
    }
    std::fs::write(&out_md, md).unwrap();
    println!("written {out_md}");
    let _ = std::fs::remove_dir_all(&work);
}
