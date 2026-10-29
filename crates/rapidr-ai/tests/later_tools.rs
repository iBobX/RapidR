//! A tool call that comes later (RapidR Studio's app tools: the program
//! it runs clicks what the model asked, and answers when its handlers have
//! run): the turn waits for it — busy meanwhile —, then sends its result
//! and goes on; Stop while it waits answers every call as stopped.

use std::cell::RefCell;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;

use rapidr_ai::rai::{self, Fire, ToolOutcome, ToolProvider};
use rapidr_ai::Tool;
use rapidr_value::Value;
use serde_json::{json, Value as Json};

#[derive(Clone, Copy)]
struct Quiet;

impl Fire for Quiet {
    fn fire(self, _name: &str, _event: &str, _args: &[Value]) {}
}

thread_local! {
    /// The answer the fake program gives (None: not yet).
    static ANSWER: RefCell<Option<String>> = const { RefCell::new(None) };
    static ASKED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

struct Later;

impl ToolProvider for Later {
    fn tools(&self) -> Vec<Tool> {
        vec![Tool { name: "app_click".into(), description: "clicks".into(), input_schema: json!({"type": "object", "properties": {"caption": {"type": "string"}}}) }]
    }
    fn instructions(&self) -> String {
        String::new()
    }
    fn facts(&self) -> String {
        String::new()
    }
    fn begin_turn(&mut self) {}
    fn call(&mut self, name: &str, input: &Json) -> ToolOutcome {
        ASKED.with(|a| a.borrow_mut().push(format!("{name} {input}")));
        ToolOutcome::later("ticket-1")
    }
    fn turn_status(&self) -> Option<String> {
        None
    }
    fn finish_call(&mut self, _name: &str, ticket: &str) -> Option<ToolOutcome> {
        assert_eq!(ticket, "ticket-1");
        let text = ANSWER.with(|a| a.borrow_mut().take())?;
        Some(ToolOutcome { content: text, is_error: false, note: "Clicked 7".into(), read: None, risk: "ui", target: String::new() })
    }
}

fn factory(_rai: &str, _component: &str, _kind: &str) -> Result<Box<dyn ToolProvider>, String> {
    Ok(Box::new(Later))
}

fn sse(chunks: &[Json]) -> String {
    chunks.iter().map(|c| format!("data: {c}\n\n")).collect::<String>() + "data: [DONE]\n\n"
}

/// A compatible server on 127.0.0.1 answering each POST in turn; the
/// bodies it got come back from the thread.
fn mock(answers: Vec<String>) -> (String, std::thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    let h = std::thread::spawn(move || {
        let mut seen = Vec::new();
        let mut answers = answers.into_iter().peekable();
        while let Some(a) = answers.peek().cloned() {
            let (mut st, _) = listener.accept().unwrap();
            let mut r = BufReader::new(st.try_clone().unwrap());
            let mut len = 0;
            let mut first = String::new();
            loop {
                let mut l = String::new();
                r.read_line(&mut l).unwrap();
                if first.is_empty() {
                    first = l.clone();
                }
                if let Some(v) = l.to_ascii_lowercase().strip_prefix("content-length:") {
                    len = v.trim().parse().unwrap();
                }
                if l == "\r\n" {
                    break;
                }
            }
            // (the models list RAI asks for its window)
            if first.starts_with("GET ") {
                let models = r#"{"data":[{"id":"mock-1"}]}"#;
                let resp = format!("HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{models}", models.len());
                st.write_all(resp.as_bytes()).unwrap();
                continue;
            }
            answers.next();
            let mut b = vec![0; len];
            r.read_exact(&mut b).unwrap();
            seen.push(String::from_utf8(b).unwrap());
            let resp = format!("HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{a}", a.len());
            st.write_all(resp.as_bytes()).unwrap();
        }
        seen
    });
    (url, h)
}

fn busy(n: &str) -> bool {
    rai::get(n, "busy").unwrap().to_i64() != 0
}

fn set(n: &str, p: &str, v: &str) {
    rai::set(n, p, &Value::String(v.into()));
}

fn ask(n: &str, q: &str) {
    let ok = rai::call(n, "ask", &[Value::String(q.into())], |v| v.to_string_val()).unwrap().to_i64();
    assert_eq!(ok, -1, "{}", rai::get(n, "error").unwrap().to_string_val());
}

#[test]
fn a_call_that_comes_later_holds_the_turn_then_goes_on_and_stop_answers_it() {
    std::env::set_var("RAPIDR_AI_KEYSTORE", "memory");
    std::env::set_var("RAPIDR_AI_AUDIT", std::env::temp_dir().join(format!("rapidr-later-{}.jsonl", std::process::id())));
    rai::set_tool_factory(factory);
    let click = sse(&[json!({"choices": [{"delta": {"tool_calls": [{"index": 0, "id": "c1", "function": {"name": "app_click", "arguments": "{\"caption\":\"7\"}"}}]}, "finish_reason": "tool_calls"}]})]);
    let done = sse(&[json!({"choices": [{"delta": {"content": "It shows 7 now."}, "finish_reason": "stop"}]})]);
    let (url, server) = mock(vec![click.clone(), done, click]);
    let n = "later1";
    set(n, "provider", "compatible");
    set(n, "baseurl", &url);
    set(n, "model", "mock-1");
    let attached = rai::call(n, "attachcomponent", &[Value::String("Tools".into()), Value::String("project".into())], |v| v.to_string_val()).unwrap().to_i64();
    assert_eq!(attached, -1, "{}", rai::get(n, "error").unwrap().to_string_val());
    ask(n, "click 7");
    // (the model asks for the click; the program hasn't answered: the turn
    // waits, busy, no second request)
    for _ in 0..400 {
        rai::poll(Quiet);
        if ASKED.with(|a| !a.borrow().is_empty()) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(ASKED.with(|a| a.borrow().clone()), vec!["app_click {\"caption\":\"7\"}".to_string()]);
    for _ in 0..20 {
        rai::poll(Quiet);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(busy(n), "waiting for the program: busy");
    // (the program answers: the turn goes on with the result)
    ANSWER.with(|a| *a.borrow_mut() = Some("Clicked b7 (RButton \"7\") in form1.".into()));
    for _ in 0..1000 {
        rai::poll(Quiet);
        if !busy(n) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(!busy(n));
    assert_eq!(rai::get(n, "answer").unwrap().to_string_val(), "It shows 7 now.");
    assert_eq!(rai::get(n, "steps").unwrap().to_string_val(), "Clicked 7");
    // (again, and Stop while it waits: every call answered as stopped)
    ask(n, "click 7 again");
    for _ in 0..400 {
        rai::poll(Quiet);
        if ASKED.with(|a| a.borrow().len()) == 2 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(busy(n));
    rai::call(n, "cancel", &[], |v| v.to_string_val()).unwrap();
    assert!(!busy(n), "stopped");
    assert_eq!(rai::get(n, "status").unwrap().to_string_val(), "Stopped.");
    let seen = server.join().unwrap();
    // (the second request carried the click's result)
    let second: Json = serde_json::from_str(&seen[1]).unwrap();
    let tool = second["messages"].as_array().unwrap().iter().find(|m| m["role"] == "tool").unwrap();
    assert!(tool["content"].as_str().unwrap().contains("Clicked b7"), "{tool}");
}
