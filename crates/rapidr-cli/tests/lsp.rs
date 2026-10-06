//! `rapidr lsp` over stdio, as an editor drives it: the protocol
//! (initialize, documents, every request the server offers, diagnostics,
//! shutdown) and diagnostics equal to the compiler's — for every
//! conformance error case, what `rapidr lsp` publishes is what
//! `rapidr build-bc` prints (message, line and column).

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

use serde_json::{json, Value};

struct Client {
    child: Child,
    stdin: ChildStdin,
    incoming: Receiver<Value>,
    next_id: i64,
    /// Notifications received while waiting for replies.
    notes: Vec<Value>,
}

fn read_message(reader: &mut BufReader<ChildStdout>) -> Option<Value> {
    let mut length = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some(v) = line.strip_prefix("Content-Length: ") {
            length = v.parse().ok()?;
        }
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body).ok()?;
    serde_json::from_slice(&body).ok()
}

impl Client {
    fn start(init_options: Value) -> Client {
        let mut child = Command::new(env!("CARGO_BIN_EXE_rapidr"))
            .args(["lsp", "--stdio"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("rapidr lsp starts");
        let stdin = child.stdin.take().unwrap();
        let mut reader = BufReader::new(child.stdout.take().unwrap());
        let (tx, incoming) = channel();
        std::thread::spawn(move || {
            while let Some(m) = read_message(&mut reader) {
                if tx.send(m).is_err() {
                    break;
                }
            }
        });
        let mut c = Client { child, stdin, incoming, next_id: 1, notes: Vec::new() };
        let caps = json!({ "textDocument": { "hover": { "contentFormat": ["markdown"] } } });
        let r = c.request("initialize", json!({ "processId": null, "rootUri": null, "capabilities": caps, "initializationOptions": init_options }));
        assert_eq!(r["capabilities"]["positionEncoding"], "utf-16");
        assert_eq!(r["serverInfo"]["name"], "rapidr");
        c.notify("initialized", json!({}));
        c
    }

    fn send(&mut self, msg: Value) {
        let body = msg.to_string();
        write!(self.stdin, "Content-Length: {}\r\n\r\n{}", body.len(), body).unwrap();
        self.stdin.flush().unwrap();
    }

    fn notify(&mut self, method: &str, params: Value) {
        self.send(json!({ "jsonrpc": "2.0", "method": method, "params": params }));
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        loop {
            let m = self.incoming.recv_timeout(Duration::from_secs(60)).expect("a reply");
            if m["id"] == id && m.get("method").is_none() {
                if let Some(e) = m.get("error") {
                    return json!({ "error": e });
                }
                return m["result"].clone();
            }
            self.notes.push(m);
        }
    }

    /// The diagnostics published for a URI (the latest), waiting for them.
    fn diagnostics_for(&mut self, uri: &str) -> Vec<Value> {
        let deadline = std::time::Instant::now() + Duration::from_secs(60);
        loop {
            if let Some(n) = self.notes.iter().rev().find(|n| n["method"] == "textDocument/publishDiagnostics" && n["params"]["uri"] == uri) {
                return n["params"]["diagnostics"].as_array().cloned().unwrap_or_default();
            }
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            let m = self.incoming.recv_timeout(left).expect("diagnostics");
            self.notes.push(m);
        }
    }

    fn open(&mut self, path: &Path) -> String {
        let uri = uri(path);
        let text = std::fs::read_to_string(path).unwrap();
        self.notify("textDocument/didOpen", json!({ "textDocument": { "uri": uri, "languageId": "rapidr", "version": 1, "text": text } }));
        uri
    }

    fn shutdown(mut self) {
        let r = self.request("shutdown", Value::Null);
        assert!(r.is_null());
        self.notify("exit", Value::Null);
        let status = self.child.wait().unwrap();
        assert!(status.success(), "rapidr lsp exits cleanly");
    }
}

fn uri(path: &Path) -> String {
    let mut out = String::from("file://");
    for b in path.to_string_lossy().replace('\\', "/").bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rapidr-lsp-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn pos(line: u32, character: u32) -> Value {
    json!({ "line": line, "character": character })
}

#[test]
fn an_editor_session() {
    let dir = scratch("session");
    let main = dir.join("main.bas");
    let util = dir.join("util.inc");
    std::fs::write(&util, "CONST Limit = 10\nFUNCTION AddOne(x AS INTEGER) AS INTEGER\n    AddOne = x + 1\nEND FUNCTION\n").unwrap();
    std::fs::write(
        &main,
        "$INCLUDE \"util.inc\"\nCREATE Form AS QFORM\n    Caption = \"Hi\"\nEND CREATE\nDIM total AS INTEGER\nSUB Greet(who AS STRING, times AS INTEGER)\nPRINT who\nEND SUB\ntotal = AddOne(Limit)\nGreet \"Ann\", total\nNope total\nForm.\n",
    )
    .unwrap();
    let mut c = Client::start(json!({}));
    let main_uri = c.open(&main);
    let doc = json!({ "uri": main_uri });

    // Line 12 doesn't parse — `Form.` with no member yet is RC.EXE's
    // "Member  not part of class FORM" — and the parser's error comes
    // before the compiler's (line 11's unknown SUB): its message, there.
    let diags = c.diagnostics_for(&main_uri);
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert_eq!(diags[0]["message"], "Member  not part of class FORM");
    assert_eq!(diags[0]["range"]["start"], pos(11, 4));
    assert_eq!(diags[0]["range"]["end"], pos(11, 5));
    assert_eq!(diags[0]["severity"], 1);
    assert_eq!(diags[0]["source"], "rapidr");

    // Completion: the form's members after `Form.`.
    let r = c.request("textDocument/completion", json!({ "textDocument": doc, "position": pos(11, 5) }));
    let labels: Vec<&str> = r["items"].as_array().unwrap().iter().map(|i| i["label"].as_str().unwrap()).collect();
    for want in ["Caption", "ShowModal", "OnClose"] {
        assert!(labels.contains(&want), "{want} in {labels:?}");
    }
    let caption = r["items"].as_array().unwrap().iter().find(|i| i["label"] == "Caption").unwrap();
    assert_eq!(caption["kind"], 10, "a property");
    assert_eq!(caption["textEdit"]["range"]["start"], pos(11, 5));

    // Completion on an empty place: the program's names, builtins, statements.
    let r = c.request("textDocument/completion", json!({ "textDocument": doc, "position": pos(9, 0) }));
    let labels: Vec<&str> = r["items"].as_array().unwrap().iter().map(|i| i["label"].as_str().unwrap()).collect();
    for want in ["total", "Greet", "AddOne", "Limit", "Form", "MID$", "PRINT"] {
        assert!(labels.contains(&want), "{want} in completion");
    }

    // Hover on `total` and on `Caption` in the CREATE.
    let r = c.request("textDocument/hover", json!({ "textDocument": doc, "position": pos(8, 2) }));
    let md = r["contents"]["value"].as_str().unwrap();
    assert!(md.contains("DIM total AS INTEGER"), "{md}");
    let r = c.request("textDocument/hover", json!({ "textDocument": doc, "position": pos(2, 6) }));
    assert!(r["contents"]["value"].as_str().unwrap().contains("property of QForm"));

    // Signature help inside `AddOne(`, and in `Greet "Ann", |`.
    let r = c.request("textDocument/signatureHelp", json!({ "textDocument": doc, "position": pos(8, 15) }));
    assert_eq!(r["signatures"][0]["label"], "FUNCTION AddOne(x AS INTEGER) AS INTEGER");
    assert_eq!(r["signatures"][0]["parameters"][0]["label"], json!([16, 28]));
    let r = c.request("textDocument/signatureHelp", json!({ "textDocument": doc, "position": pos(9, 13) }));
    assert_eq!(r["activeParameter"], 1);

    // Definition of AddOne: in the include file.
    let r = c.request("textDocument/definition", json!({ "textDocument": doc, "position": pos(8, 10) }));
    assert_eq!(r[0]["uri"], uri(&util));
    assert_eq!(r[0]["range"]["start"], pos(1, 9));

    // References of total; rename it.
    let r = c.request("textDocument/references", json!({ "textDocument": doc, "position": pos(4, 5), "context": { "includeDeclaration": true } }));
    assert_eq!(r.as_array().unwrap().len(), 4);
    let r = c.request("textDocument/prepareRename", json!({ "textDocument": doc, "position": pos(4, 5) }));
    assert_eq!(r["start"], pos(4, 4));
    let r = c.request("textDocument/rename", json!({ "textDocument": doc, "position": pos(4, 5), "newName": "count" }));
    assert_eq!(r["changes"][&main_uri].as_array().unwrap().len(), 4);
    let r = c.request("textDocument/rename", json!({ "textDocument": doc, "position": pos(4, 5), "newName": "Greet" }));
    assert!(r["error"]["message"].as_str().unwrap().contains("already the name"));

    // The outline.
    let r = c.request("textDocument/documentSymbol", json!({ "textDocument": doc }));
    let names: Vec<&str> = r.as_array().unwrap().iter().map(|s| s["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["Form", "total", "Greet"]);

    // Semantic tokens and formatting.
    let r = c.request("textDocument/semanticTokens/full", json!({ "textDocument": doc }));
    assert!(r["data"].as_array().unwrap().len() >= 5 * 4);
    let r = c.request("textDocument/formatting", json!({ "textDocument": doc, "options": { "tabSize": 4, "insertSpaces": true } }));
    let edits = r.as_array().unwrap();
    assert_eq!(edits.len(), 1, "PRINT who goes in one level");
    assert_eq!(edits[0]["newText"], "    PRINT who");

    // Fix the line: the diagnostics go.
    let fixed = std::fs::read_to_string(&main).unwrap().replace("Nope total\nForm.\n", "Form.ShowModal\n");
    c.notify("textDocument/didChange", json!({ "textDocument": { "uri": main_uri, "version": 2 }, "contentChanges": [{ "text": fixed }] }));
    c.notes.clear();
    let diags = c.diagnostics_for(&main_uri);
    assert!(diags.is_empty(), "{diags:?}");

    // Unknown requests are errors, not crashes.
    let r = c.request("textDocument/inlayHint", json!({ "textDocument": doc, "range": { "start": pos(0, 0), "end": pos(1, 0) } }));
    assert!(r.get("error").is_some());
    c.shutdown();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn rapidq_compatible_projects_get_fixes() {
    let dir = scratch("compat");
    let main = dir.join("main.bas");
    std::fs::write(&main, "DIM p AS QPLOT\nDIM n AS RNUM\n").unwrap();
    let mut c = Client::start(json!({ "rapidqCompatible": true }));
    let main_uri = c.open(&main);
    let diags = c.diagnostics_for(&main_uri);
    let messages: Vec<&str> = diags.iter().map(|d| d["message"].as_str().unwrap()).collect();
    assert!(messages.iter().any(|m| m.contains("RapidQ has no QPLOT")), "{messages:?}");
    assert!(messages.iter().any(|m| m.contains("RNum is RapidR's own component")), "{messages:?}");
    let r = c.request(
        "textDocument/codeAction",
        json!({ "textDocument": { "uri": main_uri }, "range": { "start": pos(0, 9), "end": pos(0, 14) }, "context": { "diagnostics": [] } }),
    );
    assert_eq!(r[0]["title"], "Write RPlot");
    assert_eq!(r[0]["edit"]["changes"][&main_uri][0]["newText"], "RPlot");
    c.shutdown();
    let _ = std::fs::remove_dir_all(&dir);
}

/// `file:LINE:COL: error: message` lines of the compiler → (line, col, message).
fn compiler_lines(stderr: &str, file: &Path) -> Vec<(u64, u64, String)> {
    let prefix = format!("{}:", file.display());
    stderr
        .lines()
        .filter_map(|l| {
            let rest = l.strip_prefix(&prefix)?;
            let mut parts = rest.splitn(3, ':');
            let line = parts.next()?.parse().ok()?;
            let col = parts.next()?.parse().ok()?;
            let msg = parts.next()?.trim();
            let msg = msg.strip_prefix("error:").unwrap_or(msg).trim();
            Some((line, col, msg.to_string()))
        })
        .collect()
}

#[test]
fn diagnostics_are_the_compilers() {
    let cases = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/cases");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&cases)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "expected-error"))
        .map(|p| p.with_extension("bas"))
        .collect();
    files.sort();
    assert!(files.len() >= 10);
    let dir = scratch("compiler");
    let mut c = Client::start(json!({}));
    for f in &files {
        let out = Command::new(env!("CARGO_BIN_EXE_rapidr"))
            .arg("build-bc")
            .arg(f)
            .arg("-o")
            .arg(dir.join("x.rrbc"))
            .env("RAPIDR_PRINT_TO", dir.join("print.txt"))
            .output()
            .unwrap();
        assert!(!out.status.success(), "{} fails to compile", f.display());
        let mut want = compiler_lines(&String::from_utf8_lossy(&out.stderr), f);
        let file_uri = c.open(f);
        let diags = c.diagnostics_for(&file_uri);
        let mut got: Vec<(u64, u64, String)> = diags
            .iter()
            .map(|d| (d["range"]["start"]["line"].as_u64().unwrap() + 1, d["range"]["start"]["character"].as_u64().unwrap() + 1, d["message"].as_str().unwrap().to_string()))
            .collect();
        want.sort();
        got.sort();
        assert_eq!(got, want, "{}", f.display());
        c.notify("textDocument/didClose", json!({ "textDocument": { "uri": file_uri } }));
    }
    c.shutdown();
    let _ = std::fs::remove_dir_all(&dir);
}
