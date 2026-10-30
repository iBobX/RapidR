//! Replays tool calls of RapidR Studio's assistant (and Studio's MCP server,
//! which serves the same tools) in bulk, with no model: the REAL answers of
//! `check_code`, `find_example`, `search_manual`, `rapidr_describe`,
//! `list_files`, `read_file`, `write_file`, `propose_edit`, `create_project`
//! for a project given in the request. tools/rai_train uses it to make a
//! training corpus whose tool results are exactly what a model will see, and
//! to check that every program in it compiles.
//!
//!   rai_check < requests.jsonl > answers.jsonl
//!
//! One JSON object per line in:
//!   {"id": "…",                                  // echoed back
//!    "main": "main.rr",                          // the project's main file
//!    "files": [{"path": "main.rr", "text": "…"}],// the open project ([] / omitted: none open)
//!    "apply": true,                              // after each call, the proposals are applied
//!                                                // (Studio's MCP "Allow everything"); default true
//!    "calls": [{"tool": "check_code", "input": {"path": "main.rr"}}, …]}
//! A pseudo-call `{"tool": "__recipe_context", "input": {"question": "…"}}`
//! gives the example Studio would send with that request ("" when none).
//! `rai_check --dump` prints RAI's instructions and tool schemas (full and lite).
//! One JSON object per line out:
//!   {"id": "…", "results": [{"tool", "content", "is_error", "note"}, …]}
//!
//! Nothing is written to disk and nothing reaches the network; the tools run
//! on in-memory buffers, under the folder name `/project` (a new program:
//! `/new/<name>`), as they do for a model.

use std::collections::HashMap;
use std::io::{BufRead, Write};

use rapidr_studio::ai_tools::{self, Project, Proposal};
use serde_json::{json, Value};

fn main() {
    // (`--dump`: what a model is given besides the conversation — RAI's
    // instructions and tool schemas, full and for a small window)
    if std::env::args().nth(1).as_deref() == Some("--dump") {
        let tools = |v: Vec<rapidr_ai::Tool>| v.into_iter().map(|t| json!({"name": t.name, "description": t.description, "input_schema": t.input_schema})).collect::<Vec<_>>();
        let dump = json!({
            "instructions": rapidr_studio::ai::instructions(true),
            "instructions_lite": rapidr_studio::ai::instructions_lite(true),
            "tools": tools(ai_tools::tools(true)),
            "tools_lite": tools(ai_tools::tools_lite(true)),
        });
        println!("{dump}");
        return;
    }
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let answer = match serde_json::from_str::<Value>(&line) {
            Ok(req) => replay(&req),
            Err(e) => json!({"id": null, "error": format!("not JSON: {e}")}),
        };
        let _ = writeln!(out, "{answer}");
        let _ = out.flush();
    }
}

fn replay(req: &Value) -> Value {
    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let mut main = req.get("main").and_then(Value::as_str).unwrap_or("").to_string();
    let apply = req.get("apply").and_then(Value::as_bool).unwrap_or(true);
    let mut buffers: HashMap<String, String> = HashMap::new();
    let mut names: Vec<String> = Vec::new();
    for f in req.get("files").and_then(Value::as_array).into_iter().flatten() {
        let (path, text) = (f["path"].as_str().unwrap_or("").to_string(), f["text"].as_str().unwrap_or("").to_string());
        if !names.contains(&path) {
            names.push(path.clone());
        }
        buffers.insert(path, text);
    }
    let open = !names.is_empty();
    let mut proposals: Vec<Proposal> = Vec::new();
    let mut results = Vec::new();
    for call in req.get("calls").and_then(Value::as_array).into_iter().flatten() {
        let tool = call["tool"].as_str().unwrap_or("");
        let input = call.get("input").cloned().unwrap_or_else(|| json!({}));
        // (not a tool: the example Studio sends with a request for a new
        // program — RAI's `question_context` — or "" when it sends none)
        if tool == "__recipe_context" {
            let question = input["question"].as_str().unwrap_or("");
            let context = if rapidr_studio::recipes::asks_for_new_program(question, open) { rapidr_studio::recipes::context_for(question).unwrap_or_default() } else { String::new() };
            results.push(json!({"tool": tool, "content": context, "is_error": false, "note": ""}));
            continue;
        }
        let folder = if open || !buffers.is_empty() { "/project" } else { "" };
        let outcome = {
            let p = Project { folder, files: &names, main: &main, compat: "", buffers: &buffers };
            ai_tools::run(tool, &input, &p, &mut proposals, true)
        };
        results.push(json!({"tool": tool, "content": outcome.content, "is_error": outcome.is_error, "note": outcome.note}));
        if apply && !outcome.is_error {
            // (Studio's MCP in "Allow everything": what a tool proposed is applied at once)
            let new_project: Vec<Proposal> = proposals.iter().filter(|q| q.kind == "project").cloned().collect();
            if !new_project.is_empty() {
                buffers.clear();
                names.clear();
                for q in &new_project {
                    names.push(q.path.clone());
                    buffers.insert(q.path.clone(), q.new.clone());
                    if q.main {
                        main = q.path.clone();
                    }
                }
            } else {
                for q in &proposals {
                    if !names.contains(&q.path) {
                        names.push(q.path.clone());
                    }
                    buffers.insert(q.path.clone(), q.new.clone());
                }
            }
            proposals.clear();
        }
    }
    json!({"id": id, "results": results})
}
