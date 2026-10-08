// SEC-18 regressions for `rapidr lsp` (docs/security-audit.md). Compiled
// into crates/rapidr-lsp's unit tests (src/lib.rs, `mod
// security_regressions`); run by `tools/regress.sh security`
// (cargo test -p rapidr-lsp security).
//
// The server read any path a `file:` URI or an `$INCLUDE` named: a request
// about a document the editor never opened (`file:///…/secret.bas`) was
// answered from the disk — its outline, its formatting — and a project's
// `$INCLUDE "../outside/secret.inc"` was read and offered in completion,
// hover and diagnostics. Now the server reads only the workspace folders the
// client opened and the folders of the documents it opened.

use super::*;
use lsp_server::Connection;
use serde_json::{json, Value};
use std::time::Duration;

struct Client {
    conn: Connection,
    next: i32,
    notes: Vec<Value>,
}

impl Client {
    fn start(root: Option<&Path>) -> (Client, std::thread::JoinHandle<()>) {
        let (server, client) = Connection::memory();
        let handle = std::thread::spawn(move || {
            serve(&server).expect("serve");
        });
        let mut c = Client { conn: client, next: 1, notes: Vec::new() };
        let root_uri = root.map(|r| path_to_uri(r).unwrap().as_str().to_string());
        let folders = root.map(|r| json!([{ "uri": path_to_uri(r).unwrap().as_str(), "name": "ws" }]));
        c.request("initialize", json!({ "processId": null, "rootUri": root_uri, "workspaceFolders": folders, "capabilities": {} }));
        c.notify("initialized", json!({}));
        (c, handle)
    }

    fn notify(&mut self, method: &str, params: Value) {
        self.conn.sender.send(Message::Notification(Notification::new(method.into(), params))).unwrap();
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next;
        self.next += 1;
        self.conn.sender.send(Message::Request(Request::new(RequestId::from(id), method.into(), params))).unwrap();
        loop {
            match self.conn.receiver.recv_timeout(Duration::from_secs(60)).expect("a reply") {
                Message::Response(r) if r.id == RequestId::from(id) => return r.response_result.unwrap_or(Value::Null),
                Message::Notification(n) => self.notes.push(json!({ "method": n.method, "params": n.params })),
                _ => {}
            }
        }
    }

    fn shutdown(mut self, handle: std::thread::JoinHandle<()>) {
        self.request("shutdown", Value::Null);
        self.notify("exit", Value::Null);
        handle.join().unwrap();
    }
}

fn uri(p: &Path) -> String {
    path_to_uri(p).unwrap().as_str().to_string()
}

/// A workspace beside a folder it doesn't include: `ws/main.bas` includes
/// `../outside/secret.inc`; `outside/secret.bas` is never opened.
fn layout(tag: &str) -> (PathBuf, PathBuf, PathBuf) {
    let base = std::env::temp_dir().join(format!("rapidr-lsp-sec18-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let ws = base.join("ws");
    let outside = base.join("outside");
    std::fs::create_dir_all(&ws).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("secret.inc"), "SUB IncludedSecretRoutine\nEND SUB\n").unwrap();
    std::fs::write(outside.join("secret.bas"), "SUB LeakedSecretName\nEND SUB\n").unwrap();
    std::fs::write(ws.join("local.inc"), "SUB LocalHelper\nEND SUB\n").unwrap();
    (base, ws, outside)
}

const MAIN: &str = "$INCLUDE \"local.inc\"\n$INCLUDE \"../outside/secret.inc\"\nLoc";

#[test]
fn security_sec18_lsp_reads_only_the_workspace() {
    let (base, ws, outside) = layout("ws");
    let (mut c, handle) = Client::start(Some(&ws));
    let main = ws.join("main.bas");
    c.notify("textDocument/didOpen", json!({ "textDocument": { "uri": uri(&main), "languageId": "rapidr", "version": 1, "text": MAIN } }));

    // A document outside the workspace, never opened: nothing of it.
    let symbols = c.request("textDocument/documentSymbol", json!({ "textDocument": { "uri": uri(&outside.join("secret.bas")) } }));
    assert!(!symbols.to_string().contains("LeakedSecretName"), "read a file outside the workspace: {symbols}");
    let format = c.request("textDocument/formatting", json!({ "textDocument": { "uri": uri(&outside.join("secret.bas")) }, "options": { "tabSize": 2, "insertSpaces": true } }));
    assert!(!format.to_string().contains("LeakedSecretName"), "{format}");
    // …nor by a `..` in a URI that starts in the workspace.
    let sneaky = format!("{}/../outside/secret.bas", uri(&ws));
    let symbols = c.request("textDocument/documentSymbol", json!({ "textDocument": { "uri": sneaky } }));
    assert!(!symbols.to_string().contains("LeakedSecretName"), "{symbols}");

    // The include outside isn't read; the one inside is.
    let items = c.request("textDocument/completion", json!({ "textDocument": { "uri": uri(&main) }, "position": { "line": 2, "character": 3 } }));
    let text = items.to_string();
    assert!(text.contains("LocalHelper"), "the workspace's include is read: {text}");
    assert!(!text.contains("IncludedSecretRoutine"), "an include outside the workspace was read: {text}");
    // (and the program is told why)
    let _ = c.request("textDocument/hover", json!({ "textDocument": { "uri": uri(&main) }, "position": { "line": 0, "character": 0 } }));
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    let said = loop {
        if let Some(n) = c.notes.iter().find(|n| n["method"] == "textDocument/publishDiagnostics" && n["params"]["uri"] == uri(&main)) {
            break n["params"]["diagnostics"].to_string();
        }
        match c.conn.receiver.recv_timeout(deadline.saturating_duration_since(std::time::Instant::now())) {
            Ok(Message::Notification(n)) => c.notes.push(json!({ "method": n.method, "params": n.params })),
            Ok(_) => {}
            Err(_) => panic!("no diagnostics for main.bas"),
        }
    };
    assert!(said.contains("outside the workspace"), "{said}");
    c.shutdown(handle);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn security_sec18_lsp_without_a_folder_reads_beside_the_open_files() {
    // (an editor with one file open and no folder: that file's folder)
    let (base, ws, outside) = layout("nofolder");
    let (mut c, handle) = Client::start(None);
    let symbols = c.request("textDocument/documentSymbol", json!({ "textDocument": { "uri": uri(&outside.join("secret.bas")) } }));
    assert!(!symbols.to_string().contains("LeakedSecretName"), "{symbols}");
    let main = ws.join("main.bas");
    c.notify("textDocument/didOpen", json!({ "textDocument": { "uri": uri(&main), "languageId": "rapidr", "version": 1, "text": MAIN } }));
    let items = c.request("textDocument/completion", json!({ "textDocument": { "uri": uri(&main) }, "position": { "line": 2, "character": 3 } }));
    let text = items.to_string();
    assert!(text.contains("LocalHelper") && !text.contains("IncludedSecretRoutine"), "{text}");
    let symbols = c.request("textDocument/documentSymbol", json!({ "textDocument": { "uri": uri(&ws.join("local.inc")) } }));
    assert!(symbols.to_string().contains("LocalHelper"), "a file beside the open one is read: {symbols}");
    c.shutdown(handle);
    let _ = std::fs::remove_dir_all(&base);
}
