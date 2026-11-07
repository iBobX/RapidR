//! The core's tests: what every transport shares — the definitions, the
//! arguments' check, a call's life (one at a time, answered then or later,
//! dropped, cancelled), the clients, and the answer in each transport's
//! form. A mock host (its own properties, handlers and clock) and a mock
//! sink stand in for a runtime and a transport.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use serde_json::{json, Value as Json};

use super::*;

type Handler = Rc<dyn Fn(&[Value])>;

thread_local! {
    static PROPS: RefCell<HashMap<(String, String), Value>> = RefCell::new(HashMap::new());
    static HANDLERS: RefCell<HashMap<(String, String), Handler>> = RefCell::new(HashMap::new());
    static FIRED: RefCell<Vec<(String, Vec<String>)>> = const { RefCell::new(Vec::new()) };
    static CLOCK: Cell<f64> = const { Cell::new(100.0) };
    static WOKEN: Cell<u32> = const { Cell::new(0) };
    /// Handlers' continuations held back (the interpreter runs them after
    /// the handler, later): run with `run_later`.
    static LATER: RefCell<Vec<Box<dyn FnOnce()>>> = RefCell::new(Vec::new());
    static HOLD: Cell<bool> = const { Cell::new(false) };
}

#[derive(Clone, Copy)]
struct Mock;

impl Host for Mock {
    fn get(self, name: &str, prop: &str) -> Value {
        PROPS.with(|p| p.borrow().get(&(name.to_lowercase(), prop.to_lowercase())).cloned().unwrap_or(Value::Null))
    }
    fn set(self, name: &str, prop: &str, v: Value) {
        PROPS.with(|p| p.borrow_mut().insert((name.to_lowercase(), prop.to_lowercase()), v));
    }
    fn has_handler(self, name: &str, event: &str) -> bool {
        HANDLERS.with(|h| h.borrow().contains_key(&(name.to_lowercase(), event.to_string())))
    }
    fn fire_then(self, name: &str, event: &str, args: &[Value], then: Box<dyn FnOnce()>) {
        FIRED.with(|f| f.borrow_mut().push((event.to_string(), args.iter().map(Value::to_string_val).collect())));
        let h = HANDLERS.with(|h| h.borrow().get(&(name.to_lowercase(), event.to_string())).cloned());
        if let Some(h) = h {
            h(args);
        }
        if HOLD.with(Cell::get) {
            LATER.with(|l| l.borrow_mut().push(then));
        } else {
            then();
        }
    }
    fn wake(self) {
        WOKEN.with(|w| w.set(w.get() + 1));
    }
    fn now(self) -> f64 {
        CLOCK.with(Cell::get)
    }
    fn read_file(self, path: &str) -> Result<Vec<u8>, String> {
        match path {
            "shot.png" => Ok(b"\x89PNG\r\n\x1a\nPIXELS".to_vec()),
            _ => Err("no such file".into()),
        }
    }
    fn scrub(self, text: &str) -> String {
        text.replace("sk-secret-key-0123456789abcdef", "(an API key, not shown)")
    }
}

fn on(name: &str, event: &str, h: impl Fn(&[Value]) + 'static) {
    HANDLERS.with(|m| m.borrow_mut().insert((name.to_lowercase(), event.to_string()), Rc::new(h)));
}

fn fired(event: &str) -> Vec<Vec<String>> {
    FIRED.with(|f| f.borrow().iter().filter(|(e, _)| e == event).map(|(_, a)| a.clone()).collect())
}

fn prop(name: &str, p: &str) -> String {
    Mock.get(name, p).to_string_val()
}

fn run_later() {
    let l: Vec<Box<dyn FnOnce()>> = LATER.with(|l| l.borrow_mut().drain(..).collect());
    for f in l {
        f();
    }
}

/// What a mock transport's sink heard.
#[derive(Default)]
struct Heard {
    done: Vec<Done>,
    progress: Vec<(String, String)>,
    waiting: u32,
}

struct Probe(Rc<RefCell<Heard>>);

impl Sink for Probe {
    fn finish(self: Box<Self>, done: Done) -> bool {
        self.0.borrow_mut().done.push(done);
        true
    }
    fn progress(&mut self, server: &str, message: &str, _done: Option<f64>, _total: Option<f64>) -> bool {
        self.0.borrow_mut().progress.push((server.to_string(), message.to_string()));
        true
    }
    fn waiting(&mut self) {
        self.0.borrow_mut().waiting += 1;
    }
}

fn probe() -> (Box<dyn Sink>, Rc<RefCell<Heard>>) {
    let h = Rc::new(RefCell::new(Heard::default()));
    (Box::new(Probe(h.clone())), h)
}

/// A server `name` up with one tool, `echo(text)`.
fn server(name: &str) {
    open(Mock, name, Info { name: "rapidr-test".into(), version: "1.0".into(), ..Info::default() }, 900.0);
    assert!(add_tool(Mock, name, "echo", "Says it back.", "Text AS STRING, [Times AS INTEGER]", "read"));
}

/// A call of `tool` with `args` from `client`, checked and queued.
fn call(name: &str, tool: &str, args: Json, client: &str) -> (i64, Rc<RefCell<Heard>>) {
    let (target, args) = check_call(name, Kind::Tool, tool, args).expect("a good call");
    heard(Mock, name, client, "1", true);
    let (sink, h) = probe();
    let id = enqueue(Mock, name, Queued { kind: Kind::Tool, target, args, client: client.into() }, sink).expect("the server is up");
    (id, h)
}

fn text_of(d: &Done) -> String {
    match d {
        Done::Answer { outcome: Outcome::Text { text, .. }, .. } => text.clone(),
        other => format!("{other:?}"),
    }
}

#[test]
fn arguments_are_checked_with_words_an_ai_can_act_on() {
    let d = ToolDef { name: "read_file".into(), title: String::new(), description: String::new(), schema: json!({"type": "object", "properties": {"path": {"type": "string", "description": "the file's path"}, "lines": {"type": "integer"}, "mode": {"type": "string", "enum": ["a", "b"]}, "edits": {"type": "array", "items": {"type": "object"}}}, "required": ["path"], "additionalProperties": false}), annotations: None, untrusted: false, consequential: None };
    assert!(check_args(&d, &json!({"path": "main.rr"})).is_ok());
    let e = check_args(&d, &json!({})).unwrap_err();
    assert!(e.contains("it needs `path` (a string: the file's path)"), "{e}");
    let e = check_args(&d, &json!({"path": 3})).unwrap_err();
    assert!(e.contains("`path` must be a string, not 3"), "{e}");
    assert!(check_args(&d, &json!({"path": "x", "lines": 2.0})).is_ok(), "2.0 is a whole number");
    assert!(check_args(&d, &json!({"path": "x", "lines": 2.5})).is_err());
    let e = check_args(&d, &json!({"path": "x", "file": "y"})).unwrap_err();
    assert!(e.contains("no argument `file`") && e.contains("path, lines, mode, edits"), "{e}");
    assert!(check_args(&d, &json!({"path": "x", "mode": "c"})).unwrap_err().contains("one of \"a\", \"b\""));
    assert!(check_args(&d, &json!({"path": "x", "edits": [{}, 3]})).unwrap_err().contains("`edits`[1] must be an object"));
    assert!(check_args(&d, &json!([1])).is_err());
    let free = ToolDef { schema: json!({"type": "object"}), ..d.clone() };
    assert!(check_args(&free, &json!({"anything": 1})).is_ok(), "no schema to keep to: anything");
}

#[test]
fn a_list_sent_as_json_text_is_the_list() {
    // (a small model's tool call layer: "edits" as a string of JSON, "20" for a number)
    let d = ToolDef { name: "edit".into(), title: String::new(), description: String::new(), schema: json!({"type": "object", "properties": {"path": {"type": "string"}, "edits": {"type": "array", "items": {"type": "object"}}, "wait": {"type": "integer"}, "debug": {"type": "boolean"}, "meta": {"type": "object"}}, "required": ["path"], "additionalProperties": false}), annotations: None, untrusted: false, consequential: None };
    let mut a = json!({"path": "main.rr", "edits": "[{\"old_text\": \"a\", \"new_text\": \"b\"}]", "wait": " 20 ", "debug": "true", "meta": "{\"k\": 1}"});
    coerce_args(&d, &mut a);
    assert_eq!(a["edits"], json!([{"old_text": "a", "new_text": "b"}]));
    assert_eq!(a["wait"], json!(20));
    assert_eq!(a["debug"], json!(true));
    assert_eq!(a["meta"], json!({"k": 1}));
    assert!(check_args(&d, &a).is_ok());
    // (a string that isn't the value stays, and is refused in words; a string argument is never parsed)
    let mut b = json!({"path": "[1, 2]", "edits": "[{\"old_text\": \"a\"}]\n}", "wait": "soon"});
    coerce_args(&d, &mut b);
    assert_eq!(b["path"], json!("[1, 2]"));
    assert_eq!(b["edits"], json!("[{\"old_text\": \"a\"}]\n}"));
    let e = check_args(&d, &b).unwrap_err();
    assert!(e.contains("`edits` must be a list") && e.contains("`wait` must be a whole number"), "{e}");
    // (an object where a list is wanted is not made a list)
    let mut c = json!({"path": "x", "edits": "{\"old_text\": \"a\"}"});
    coerce_args(&d, &mut c);
    assert!(c["edits"].is_string());
}

#[test]
fn an_argument_in_another_case_is_that_argument() {
    // (`Path` for `path`: a tool list that spelled it so, a model that guesses)
    let base = ToolDef { name: "t".into(), title: String::new(), description: String::new(), schema: json!({"type": "object", "properties": {"path": {"type": "string"}, "wait": {"type": "integer"}}, "required": ["path"], "additionalProperties": false}), annotations: None, untrusted: false, consequential: None };
    let mut a = json!({"Path": "main.rr", "WAIT": "5"});
    coerce_args(&base, &mut a);
    assert_eq!(a, json!({"path": "main.rr", "wait": 5}));
    assert!(check_args(&base, &a).is_ok());
    // (both given: the schema's own spelling wins, the other is refused as unknown)
    let mut both = json!({"path": "a", "Path": "b"});
    coerce_args(&base, &mut both);
    assert_eq!(both["path"], json!("a"));
    assert!(check_args(&base, &both).is_err());
    // (`ToX` for `to_x`: an older tool list's spelling, underscores aside)
    let d = ToolDef { schema: json!({"type": "object", "properties": {"to_x": {"type": "integer"}, "to_y": {"type": "integer"}}, "required": ["to_x", "to_y"], "additionalProperties": false}), ..base };
    let mut a = json!({"ToX": 180, "ToY": "20"});
    coerce_args(&d, &mut a);
    assert_eq!(a, json!({"to_x": 180, "to_y": 20}));
    assert!(check_args(&d, &a).is_ok());
}

#[test]
fn a_subs_parameters_make_a_schema() {
    let s = params_schema("Text AS STRING, [Count AS INTEGER], Done AS BOOLEAN, [Tags AS LIST]").unwrap();
    assert_eq!(s["properties"]["Text"]["type"], "string");
    assert_eq!(s["properties"]["Count"]["type"], "integer");
    assert_eq!(s["properties"]["Done"]["type"], "boolean");
    assert_eq!(s["properties"]["Tags"]["type"], "array");
    assert_eq!(s["required"], json!(["Text", "Done"]));
    assert_eq!(params_schema("Name$").unwrap()["properties"]["Name"]["type"], "string");
    assert!(params_schema("X AS PICTURE").unwrap_err().contains("PICTURE"));
    assert!(params_schema("a b c d").is_err());
}

#[test]
fn images_are_known_by_their_bytes() {
    assert_eq!(image_mime(b"\x89PNG\r\n\x1a\n...."), Some("image/png"));
    assert_eq!(image_mime(&[0xFF, 0xD8, 0xFF]), Some("image/jpeg"));
    assert_eq!(image_mime(b"BM...."), None);
}

/// AddTool's Kind: MCP's annotations from the words, the browser's own
/// two hints kept apart (MCP never sees them); a wrong word refused.
#[test]
fn how_a_tool_acts() {
    let name = "core-hints";
    assert!(add_tool(Mock, name, "look", "", "", "read untrusted"));
    let t = tool(name, "look").unwrap();
    assert_eq!(t.annotations, Some(json!({"readOnlyHint": true, "openWorldHint": false})));
    assert!(t.untrusted);
    assert_eq!(t.browser_annotations(), json!({"readOnlyHint": true, "untrustedContentHint": true, "consequentialHint": false}));
    assert!(add_tool(Mock, name, "save", "", "", "change"));
    assert_eq!(tool(name, "save").unwrap().browser_annotations(), json!({"readOnlyHint": false, "untrustedContentHint": false, "consequentialHint": true}));
    assert!(add_tool(Mock, name, "peek", "", "", ""));
    assert_eq!(tool(name, "peek").unwrap().annotations, None, "no Kind: no annotations (as before)");
    // (as JSON: the browser's hints taken out of MCP's annotations)
    assert!(add_tool(Mock, name, "book", "", "", r#"{"readOnlyHint": false, "title": "Book", "consequentialHint": true, "untrustedContentHint": true}"#));
    let b = tool(name, "book").unwrap();
    assert_eq!(b.annotations, Some(json!({"readOnlyHint": false, "title": "Book"})));
    assert_eq!(b.title, "Book");
    assert_eq!((b.untrusted, b.consequential), (true, Some(true)));
    assert!(!add_tool(Mock, name, "bad", "", "", "maybe"));
    assert!(prop(name, "error").contains("\"maybe\" isn't read, change, delete, network, untrusted, consequential"), "{}", prop(name, "error"));
    assert!(!add_tool(Mock, name, "twice", "", "", "read change"));
    assert!(!add_tool(Mock, name, "bad name", "", "", ""));
    assert!(prop(name, "error").contains("isn't a tool name"));
    // (every change moves the revision; a changed tool its fingerprint)
    let r = revision(name);
    let f = tool(name, "look").unwrap().fingerprint();
    assert!(add_tool(Mock, name, "look", "Looks again.", "", "read untrusted"));
    assert!(revision(name) > r);
    assert_ne!(tool(name, "look").unwrap().fingerprint(), f);
    assert_eq!(tools(name).iter().map(|t| t.name.as_str()).collect::<Vec<_>>(), ["look", "save", "peek", "book"], "replaced in its place");
    assert!(method(Mock, name, "removetool", &[v_str("SAVE")]).unwrap().to_bool());
    assert_eq!(method(Mock, name, "toolcount", &[]).unwrap().to_i64(), 3);
    forget(name);
}

/// A call's life: checked (an alias its tool's, a wrong call told what it
/// needs, an unknown tool named with the ones there are), queued, handed
/// to the program with Call / Tool / ClientName / Arguments set, answered
/// once — scrubbed — and gone.
#[test]
fn a_call_from_check_to_answer() {
    let name = "core-life";
    server(name);
    assert!(add_alias(Mock, name, "say", "echo"));
    match check_call(name, Kind::Tool, "nope", json!({})) {
        Err(Refused::UnknownTool(s)) => assert_eq!(s, "Unknown tool: nope. The tools are: echo."),
        other => panic!("{other:?}"),
    }
    match check_call(name, Kind::Tool, "echo", json!({"Times": 2})) {
        Err(Refused::BadArguments(s)) => assert!(s.starts_with("Call echo again: it needs `Text`"), "{s}"),
        other => panic!("{other:?}"),
    }
    assert_eq!(check_call(name, Kind::Resource, "app://x", json!({})), Err(Refused::UnknownResource("app://x".into())));
    let asked = Rc::new(RefCell::new(Vec::new()));
    let seen = asked.clone();
    on(name, "ontoolcall", move |args| {
        seen.borrow_mut().push(args.iter().map(Value::to_string_val).collect::<Vec<_>>());
        let id = args[2].to_i64();
        let said = argument("core-life", "text", id);
        assert!(method(Mock, "core-life", "reply", &[v_int(id), v_str(&format!("{said} sk-secret-key-0123456789abcdef"))]).unwrap().to_bool());
    });
    let (id, h) = call(name, "say", json!({"text": "hi", "times": "3"}), "Claude");
    assert_eq!(prop(name, "pendingcount"), "1");
    assert!(is_waiting(name, id));
    poll(Mock, name);
    assert_eq!(asked.borrow()[0], vec!["echo".to_string(), r#"{"Text":"hi","Times":3}"#.to_string(), id.to_string()], "the tool's own name, the arguments as its schema spells them");
    assert_eq!((prop(name, "call"), prop(name, "tool"), prop(name, "clientname")), (id.to_string(), "echo".to_string(), "Claude".to_string()));
    let done = &h.borrow().done;
    assert_eq!(done.len(), 1);
    assert_eq!(done[0], Done::Answer { kind: Kind::Tool, target: "echo".into(), mime: String::new(), outcome: Outcome::Text { text: "hi (an API key, not shown)".into(), error: false } });
    assert_eq!(prop(name, "pendingcount"), "0");
    assert!(!is_waiting(name, id));
    assert!(!method(Mock, name, "reply", &[v_int(id), v_str("again")]).unwrap().to_bool(), "answered once");
    assert_eq!(clients(name)[0].calls, 1);
    close(Mock, name);
    forget(name);
}

/// One call at a time: the next waits until the last one's handler has
/// run (in the interpreter, its continuation comes later); a call the
/// handler didn't answer waits — its sink told — and is answered later,
/// with Progress meanwhile.
#[test]
fn one_at_a_time_and_answered_later() {
    let name = "core-later";
    server(name);
    on(name, "ontoolcall", |_| {});
    HOLD.with(|h| h.set(true));
    let (a, ha) = call(name, "echo", json!({"Text": "1"}), "app");
    let (b, hb) = call(name, "echo", json!({"Text": "2"}), "app");
    poll(Mock, name);
    poll(Mock, name);
    assert_eq!(fired("ontoolcall").iter().filter(|f| f[2] == a.to_string() || f[2] == b.to_string()).count(), 1, "the second waits for the first's handler");
    run_later();
    HOLD.with(|h| h.set(false));
    assert_eq!(ha.borrow().waiting, 1, "not answered in its handler: it waits");
    poll(Mock, name);
    assert_eq!(fired("ontoolcall").iter().filter(|f| f[2] == b.to_string()).count(), 1);
    assert!(method(Mock, name, "progress", &[v_int(a), v_str("Asking the user…")]).unwrap().to_bool());
    assert_eq!(ha.borrow().progress, vec![("rapidr-test".to_string(), "Asking the user…".to_string())]);
    assert!(method(Mock, name, "reply", &[v_int(b), v_str("two")]).unwrap().to_bool());
    assert!(method(Mock, name, "reply", &[v_int(a), v_str("one"), v_int(1)]).unwrap().to_bool());
    assert_eq!(ha.borrow().done[0], Done::Answer { kind: Kind::Tool, target: "echo".into(), mime: String::new(), outcome: Outcome::Text { text: "one".into(), error: true } });
    assert_eq!(text_of(&hb.borrow().done[0]), "two");
    close(Mock, name);
    forget(name);
}

/// No OnToolCall: the call is told so at once, and the next goes on.
#[test]
fn a_program_without_a_handler_says_so() {
    let name = "core-nohandler";
    server(name);
    let (_, h) = call(name, "echo", json!({"Text": "x"}), "app");
    poll(Mock, name);
    assert_eq!(h.borrow().done[0], Done::Answer { kind: Kind::Tool, target: "echo".into(), mime: String::new(), outcome: Outcome::Text { text: "This program doesn't answer tool calls yet (it has no OnToolCall handler).".into(), error: true } });
    close(Mock, name);
    forget(name);
}

/// Stop, Disconnect and a caller that gives up: each waiting call ends
/// with its reason; Disconnect keeps the app out (the transport hears the
/// names before OnDisconnect); a cancelled call never reaches the program,
/// OnToolCancel says which.
#[test]
fn calls_that_end_without_an_answer() {
    let name = "core-dropped";
    server(name);
    on(name, "ondisconnect", |_| {});
    on(name, "ontoolcancel", |_| {});
    let (_, h1) = call(name, "echo", json!({"Text": "x"}), "Goose");
    let (_, h2) = call(name, "echo", json!({"Text": "y"}), "Other");
    let mut heard_first = Vec::new();
    let out = disconnect(Mock, name, "goose", |n| heard_first = n.to_vec()).unwrap();
    assert_eq!(out, vec!["Goose".to_string()]);
    assert_eq!(heard_first, vec!["Goose".to_string()]);
    assert_eq!(h1.borrow().done, vec![Done::Dropped(Dropped::Disconnected)]);
    assert!(shut_out(name, "GOOSE"));
    assert!(fired("ondisconnect").contains(&vec!["Goose".to_string()]));
    assert!(clients(name).iter().all(|c| c.name != "Goose"));
    // (cancelled before its turn: the program never sees it)
    let (c, h3) = call(name, "echo", json!({"Text": "z"}), "Other");
    assert!(cancel(Mock, name, c));
    assert_eq!(h3.borrow().done, vec![Done::Dropped(Dropped::Cancelled)]);
    assert!(fired("ontoolcancel").contains(&vec![c.to_string()]));
    assert!(!cancel(Mock, name, c));
    assert!(close(Mock, name));
    assert_eq!(h2.borrow().done, vec![Done::Dropped(Dropped::Stopped)]);
    assert_eq!((prop(name, "listening"), prop(name, "pendingcount"), prop(name, "clientcount")), ("0".to_string(), "0".to_string(), "0".to_string()));
    assert!(!is_open(name));
    assert!(enqueue(Mock, name, Queued { kind: Kind::Tool, target: "echo".into(), args: json!({}), client: "x".into() }, probe().0).is_none(), "not up: nothing queued");
    forget(name);
}

/// Clients: heard (OnConnect once), counted, published; silent past
/// IdleSeconds with nothing waiting: gone (OnDisconnect).
#[test]
fn clients_come_and_go() {
    let name = "core-clients";
    server(name);
    on(name, "onconnect", |_| {});
    on(name, "ondisconnect", |_| {});
    heard(Mock, name, "Claude", "2.1", false);
    heard(Mock, name, "Claude", "2.1", true);
    assert_eq!(fired("onconnect").iter().filter(|a| a[0] == "Claude").count(), 1);
    assert_eq!(prop(name, "clientcount"), "1");
    assert_eq!(prop(name, "clients"), "Claude\t2.1\t0\t1");
    CLOCK.with(|c| c.set(c.get() + 901.0));
    poll(Mock, name);
    assert!(clients(name).is_empty());
    assert!(fired("ondisconnect").contains(&vec!["Claude".to_string()]));
    close(Mock, name);
    forget(name);
}

/// A resource's answer carries its MIME type; a picture is read through
/// the host and goes as it is (its bytes never scrubbed); the Token never
/// leaves.
#[test]
fn resources_pictures_and_the_token() {
    let name = "core-res";
    server(name);
    Mock.set(name, "token", v_str("tok-0123456789abcdef0123"));
    assert!(add_resource(Mock, name, "app://notes", "Notes", "", "text/markdown"));
    let (target, args) = check_call(name, Kind::Resource, "app://notes", json!({})).unwrap();
    let (sink, h) = probe();
    let id = enqueue(Mock, name, Queued { kind: Kind::Resource, target, args, client: "app".into() }, sink).unwrap();
    assert!(method(Mock, name, "reply", &[v_int(id), v_str("the token is tok-0123456789abcdef0123")]).unwrap().to_bool());
    assert_eq!(h.borrow().done[0], Done::Answer { kind: Kind::Resource, target: "app://notes".into(), mime: "text/markdown".into(), outcome: Outcome::Text { text: "the token is (the token, not shown)".into(), error: false } });
    let (id, h) = call(name, "echo", json!({"Text": "x"}), "app");
    assert!(method(Mock, name, "replyimage", &[v_int(id), v_str("shot.png"), v_str("Studio")]).unwrap().to_bool());
    match &h.borrow().done[0] {
        Done::Answer { outcome: Outcome::Image { text, data, mime }, .. } => assert_eq!((text.as_str(), data.as_slice(), *mime), ("Studio", &b"\x89PNG\r\n\x1a\nPIXELS"[..], "image/png")),
        other => panic!("{other:?}"),
    }
    let (id, h) = call(name, "echo", json!({"Text": "x"}), "app");
    assert!(method(Mock, name, "replyimage", &[v_int(id), v_str("missing.png")]).unwrap().to_bool());
    assert_eq!(text_of(&h.borrow().done[0]), "The picture couldn't be read (no such file).");
    let (id, _) = call(name, "echo", json!({"Text": "x"}), "app");
    assert!(!method(Mock, name, "replyjson", &[v_int(id), v_str("[1]")]).unwrap().to_bool());
    assert!(prop(name, "error").starts_with("ReplyJson"));
    close(Mock, name);
    forget(name);
}

/// The answer as each transport sends it: MCP's content list, and the
/// browser's value — a string, `{error}` as data, a picture as a data URL,
/// ReplyJson's structured part — cut past the limit, saying so.
#[test]
fn an_answer_in_each_transports_form() {
    let ok = Outcome::Text { text: "fine".into(), error: false };
    let bad = Outcome::Text { text: "Call it with a path.".into(), error: true };
    assert_eq!(ok.mcp_tool_result(), json!({"content": [{"type": "text", "text": "fine"}]}));
    assert_eq!(bad.mcp_tool_result(), json!({"content": [{"type": "text", "text": "Call it with a path."}], "isError": true}));
    assert_eq!(ok.browser_value(0), json!("fine"));
    assert_eq!(bad.browser_value(0), json!({"error": "Call it with a path."}));
    let pic = Outcome::Image { text: "Studio".into(), data: vec![1, 2, 3], mime: "image/png" };
    assert_eq!(pic.mcp_tool_result(), json!({"content": [{"type": "text", "text": "Studio"}, {"type": "image", "data": "AQID", "mimeType": "image/png"}]}));
    assert_eq!(pic.browser_value(0), json!({"text": "Studio", "image": "data:image/png;base64,AQID"}));
    let structured = Outcome::Result(json!({"content": [{"type": "text", "text": "3 rows"}], "structuredContent": {"rows": 3}}));
    assert_eq!(structured.browser_value(0), json!({"rows": 3}));
    assert_eq!(Outcome::Result(json!({"content": [{"type": "text", "text": "a"}, {"type": "text", "text": "b"}]})).browser_value(0), json!("a\nb"));
    assert_eq!(Outcome::Result(json!({"content": [{"type": "text", "text": "no"}], "isError": true})).browser_value(0), json!({"error": "no"}));
    let long = Outcome::Text { text: "x".repeat(5000), error: false };
    let v = long.browser_value(1500);
    let s = v.as_str().unwrap();
    assert!(s.chars().count() <= 1500, "{}", s.chars().count());
    assert!(s.ends_with("more characters not shown: ask for a smaller part)"), "{s}");
    assert!(s.contains("(3580 more"), "{}", &s[s.len() - 80..]);
    assert_eq!(cut("héllo ✓", 0), "héllo ✓");
    assert_eq!(cut("héllo ✓", 7), "héllo ✓");
}
