//! What `rapidr lsp` answers beyond completion and hover, over an in-memory
//! connection: the parameter names before literal arguments (inlay hints),
//! the colours written in the program and the ways to write one, the
//! highlights of a name, and RapidR's own Help requests (`rapidr/helpPage`,
//! `rapidr/helpSuggest`: the pages Studio's Help pane shows).

use std::time::Duration;

use lsp_server::{Connection, Message, Notification, Request, RequestId};
use serde_json::{json, Value};

struct Client {
    conn: Connection,
    next: i32,
    uri: String,
    handle: Option<std::thread::JoinHandle<()>>,
}

const PROGRAM: &str = "$APPTYPE GUI
SUB DrawDot(x AS INTEGER, y AS INTEGER, r AS INTEGER)
END SUB
DIM total AS INTEGER
DIM s AS STRING
x = 5
DrawDot x, 40, 12
CREATE Form1 AS RForm
  Color = RGB(255, 130, 20)
  CREATE Button1 AS RButton
    Caption = \"Go\"
    Color = &H0000FF
    FontColor = clRed
    OnClick = Greet
  END CREATE
END CREATE
SUB Greet
  total = total + 1
  Button1.Caption = \"x\"
END SUB
";

impl Client {
    fn start(program: &str) -> Client {
        let (server, client) = Connection::memory();
        let handle = std::thread::spawn(move || {
            rapidr_lsp::serve(&server).expect("serve");
        });
        let dir = std::env::temp_dir().join(format!("rapidr-lsp-features-{}", std::process::id()));
        let uri = format!("file://{}/features.bas", dir.display());
        let mut c = Client { conn: client, next: 1, uri, handle: Some(handle) };
        let init = c.request("initialize", json!({ "processId": null, "rootUri": null, "capabilities": {} }));
        assert!(init["capabilities"]["inlayHintProvider"].as_bool().unwrap_or(false), "{init}");
        c.notify("initialized", json!({}));
        let uri = c.uri.clone();
        c.notify("textDocument/didOpen", json!({ "textDocument": { "uri": uri, "languageId": "rapidr", "version": 1, "text": program } }));
        c
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
                _ => {}
            }
        }
    }

    fn at(&self, line: u32, character: u32) -> Value {
        json!({ "textDocument": { "uri": self.uri }, "position": { "line": line, "character": character } })
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        self.request("shutdown", Value::Null);
        self.notify("exit", Value::Null);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

/// The (line, column) of the first `needle` in the program, `inside` characters in.
fn find(needle: &str, inside: usize) -> (u32, u32) {
    let at = PROGRAM.find(needle).unwrap_or_else(|| panic!("{needle}"));
    let line = PROGRAM[..at].matches('\n').count() as u32;
    let col = at - PROGRAM[..at].rfind('\n').map_or(0, |i| i + 1) + inside;
    (line, col as u32)
}

#[test]
fn parameter_names_come_before_literal_arguments() {
    let mut c = Client::start(PROGRAM);
    let hints = c.request("textDocument/inlayHint", json!({ "textDocument": { "uri": c.uri }, "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 30, "character": 0 } } }));
    let labels: Vec<(u64, &str)> = hints.as_array().unwrap().iter().map(|h| (h["position"]["line"].as_u64().unwrap(), h["label"].as_str().unwrap())).collect();
    let (call_line, _) = find("DrawDot x, 40", 0);
    // (`x` for x needs no chip: the literals after it are named)
    assert_eq!(labels, [(call_line as u64, "y:"), (call_line as u64, "r:")], "{hints}");
    assert_eq!(hints[0]["kind"], 2, "a parameter's hint");
    // (a range that holds no call: no hints)
    let none = c.request("textDocument/inlayHint", json!({ "textDocument": { "uri": c.uri }, "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 2, "character": 0 } } }));
    assert_eq!(none.as_array().unwrap().len(), 0, "{none}");
}

#[test]
fn colours_have_swatches_and_a_picker_that_writes_them_as_the_program_does() {
    let mut c = Client::start(PROGRAM);
    let colors = c.request("textDocument/documentColor", json!({ "textDocument": { "uri": c.uri } }));
    let colors = colors.as_array().unwrap();
    assert_eq!(colors.len(), 3, "{colors:?}");
    // RGB(255, 130, 20)
    let first = &colors[0];
    assert_eq!(first["color"]["red"], 1.0);
    assert!((first["color"]["green"].as_f64().unwrap() - 130.0 / 255.0).abs() < 0.001);
    assert!((first["color"]["blue"].as_f64().unwrap() - 20.0 / 255.0).abs() < 0.001);
    // (&H0000FF is BBGGRR: red)
    assert_eq!(colors[1]["color"]["red"], 1.0);
    assert_eq!(colors[1]["color"]["blue"], 0.0);
    // The picker: the program's own form first.
    let pick = |c: &mut Client, range: &Value| {
        let p = c.request("textDocument/colorPresentation", json!({ "textDocument": { "uri": c.uri }, "color": { "red": 1.0, "green": 0.0, "blue": 0.5, "alpha": 1.0 }, "range": range }));
        p.as_array().unwrap().iter().map(|x| x["label"].as_str().unwrap().to_string()).collect::<Vec<_>>()
    };
    assert_eq!(pick(&mut c, &colors[0]["range"]), ["RGB(255, 0, 128)", "&H8000FF"]);
    assert_eq!(pick(&mut c, &colors[1]["range"]), ["&H8000FF", "RGB(255, 0, 128)"]);
}

#[test]
fn a_name_is_highlighted_where_it_is_used() {
    let mut c = Client::start(PROGRAM);
    let (line, col) = find("DIM total", 5);
    let got = c.request("textDocument/documentHighlight", c.at(line, col));
    let lines: Vec<u64> = got.as_array().unwrap().iter().map(|h| h["range"]["start"]["line"].as_u64().unwrap()).collect();
    // the DIM, and `total = total + 1`'s two uses
    assert_eq!(lines.len(), 3, "{got}");
    assert_eq!(lines[0], line as u64);
}

#[test]
fn the_help_page_of_a_word_a_member_and_the_programs_own_names() {
    let mut c = Client::start(PROGRAM);
    // a word: the language's entry, with the syntax to insert
    let page = c.request("rapidr/helpPage", json!({ "word": "RButton", "canInsert": true }));
    assert_eq!(page["kind"], "entry");
    assert!(page["markdown"].as_str().unwrap().starts_with("## RButton\n"), "{page}");
    assert!(page["insert"].as_str().unwrap().starts_with("CREATE Button1 AS RButton"), "{page}");
    assert!(page["markdown"].as_str().unwrap().contains("[Caption](help:Caption@RButton)"), "{page}");
    // (no editor to take the syntax: no Insert)
    let page = c.request("rapidr/helpPage", json!({ "word": "RButton" }));
    assert_eq!(page["insert"], "");
    assert!(!page["markdown"].as_str().unwrap().contains("help-insert"), "{page}");

    // the word at a position: a property set in a CREATE body is a member of its component
    let (line, col) = find("Caption = \"Go\"", 3);
    let page = c.request("rapidr/helpPage", c.at(line, col));
    assert_eq!(page["kind"], "entry", "{page}");
    assert!(page["markdown"].as_str().unwrap().starts_with("## RButton.Caption"), "{page}");
    // …and a member after a dot
    let (line, col) = find("Button1.Caption = \"x\"", 10);
    let page = c.request("rapidr/helpPage", c.at(line, col));
    assert!(page["markdown"].as_str().unwrap().starts_with("## RButton.Caption"), "{page}");

    // the program's own name: where it is declared, and its type's help
    let (line, col) = find("total = total + 1", 1);
    let page = c.request("rapidr/helpPage", c.at(line, col));
    assert_eq!(page["kind"], "mine", "{page}");
    let (decl_line, _) = find("DIM total", 0);
    assert_eq!(page["line"], decl_line);
    assert!(page["uri"].as_str().unwrap().ends_with("features.bas"), "{page}");
    assert!(page["markdown"].as_str().unwrap().contains("DIM total AS INTEGER"), "{page}");

    // a word nothing knows: the closest names
    let page = c.request("rapidr/helpPage", json!({ "word": "RButon" }));
    assert_eq!(page["kind"], "none");
    assert!(page["markdown"].as_str().unwrap().contains("[RButton](help:RButton)"), "{page}");
}

#[test]
fn the_help_box_suggests_the_programs_names_and_the_languages() {
    let mut c = Client::start(PROGRAM);
    let got = c.request("rapidr/helpSuggest", json!({ "word": "gree", "textDocument": { "uri": c.uri } }));
    assert_eq!(got[0]["name"], "Greet", "{got}");
    assert_eq!(got[0]["what"], "your sub", "{got}");
    let got = c.request("rapidr/helpSuggest", json!({ "word": "rbut" }));
    assert_eq!(got[0]["name"], "RButton", "{got}");
}

#[test]
fn hovering_a_type_after_as_says_the_type_not_a_function_of_the_same_name() {
    let mut c = Client::start(PROGRAM);
    let (line, col) = find("DIM s AS STRING", 11);
    let hover = c.request("textDocument/hover", c.at(line, col));
    let text = hover["contents"]["value"].as_str().unwrap();
    assert!(!text.contains("STRING$"), "{text}");
    assert!(text.contains("STRING"), "{text}");
}
