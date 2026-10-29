//! RMcpServer (RapidR's; desktop only): a program's tools served to AI
//! apps over MCP's Streamable HTTP transport, on 127.0.0.1 with a token —
//! RapidR Studio's own server is one (ide/mcp.inc), and any program can
//! have one.
//!
//! The program registers its tools (AddTool: a name, a description, the
//! JSON Schema of its arguments) and its resources (AddResource); the
//! component speaks the protocol itself — every message, every check —
//! and gives the program only the calls: OnToolCall(Tool, Arguments, Call)
//! on the program's own thread, one at a time, answered then or later
//! with Reply / ReplyImage / ReplyJson (a call can wait for the user, and
//! say how it goes with Progress meanwhile).
//!
//! Both eras of the protocol are answered on the same endpoint (one POST
//! a message):
//! - the current revision ([`MODERN`], 2026-07-28): no handshake and no
//!   session — each request says its version, its client and its method
//!   in `_meta` and in headers (`MCP-Protocol-Version`, `Mcp-Method`,
//!   `Mcp-Name`), checked against each other (`HeaderMismatch`);
//!   `server/discover` says what the server is;
//! - the earlier ones ([`LEGACY`]): `initialize` starts a session, its
//!   `Mcp-Session-Id` comes back with every request, DELETE ends it; GET
//!   (a stream of the server's own messages) is answered 405, as those
//!   revisions allow.
//!
//! What the server refuses before anything (RHttpServer's checks, one
//! implementation: crate::httpserver): a Host that isn't its own (DNS
//! rebinding), a web page's request (`Origin`, `Sec-Fetch-Site`), a
//! missing or wrong `Authorization: Bearer` token — and Start itself
//! without a Token. A tool's arguments are checked against its schema
//! before the program sees them: a wrong call gets a plain sentence
//! saying what the tool needs, so the AI can call it again right.
//!
//! The protocol is implemented here from the public specification
//! (modelcontextprotocol.io); no SDK.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use serde_json::{json, Map, Value as Json};

use crate::httpserver::{self, Arrived, ListenOptions, Out};
use crate::netframe::Request;
use crate::object::{rp_comp_get, rp_comp_set};
use crate::value::{v_int, v_null, v_str, Value};

/// The revision served statelessly (newest first).
pub const MODERN: &[&str] = &["2026-07-28"];
/// The initialize-based revisions served with a session (newest first).
pub const LEGACY: &[&str] = &["2025-11-25", "2025-06-18", "2025-03-26"];

const META_VERSION: &str = "io.modelcontextprotocol/protocolVersion";
const META_CLIENT: &str = "io.modelcontextprotocol/clientInfo";
const META_SERVER: &str = "io.modelcontextprotocol/serverInfo";

/// JSON-RPC's and MCP's error codes.
const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
const INTERNAL_ERROR: i64 = -32603;
const HEADER_MISMATCH: i64 = -32020;
const UNSUPPORTED_VERSION: i64 = -32022;
const RESOURCE_NOT_FOUND: i64 = -32002;
/// (the earlier revisions' sessions: refused, ended)
const SESSION_REFUSED: i64 = -32000;
const SESSION_ENDED: i64 = -32001;

/// Every version answered, the current one first.
fn supported() -> Vec<&'static str> {
    MODERN.iter().chain(LEGACY).copied().collect()
}

/// A tool the program offers.
#[derive(Clone, Debug)]
struct ToolDef {
    name: String,
    description: String,
    schema: Json,
    annotations: Option<Json>,
    title: String,
}

/// A resource the program offers (resources/list; resources/read asks it).
#[derive(Clone, Debug)]
struct ResourceDef {
    uri: String,
    name: String,
    description: String,
    mime: String,
}

#[derive(Default)]
struct Defs {
    tools: Vec<ToolDef>,
    resources: Vec<ResourceDef>,
}

/// An earlier revision's session (initialize's Mcp-Session-Id).
struct Session {
    client: String,
    protocol: String,
}

/// An AI app the server has heard from.
struct Client {
    name: String,
    version: String,
    last: Instant,
    calls: u64,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    Tool,
    Resource,
}

/// A call the program hasn't answered yet.
struct Call {
    conn: mpsc::Sender<Out>,
    request: Request,
    /// The JSON-RPC id.
    id: Json,
    kind: Kind,
    /// The tool's name, or the resource's URI.
    name: String,
    args: Json,
    client: String,
    modern: bool,
    /// The client takes an event stream as the answer.
    events: bool,
    progress: Option<Json>,
    /// Once its answer is a stream: whether its events go as chunks.
    stream: Option<bool>,
}

struct Server {
    running: Arc<AtomicBool>,
    events: mpsc::Receiver<Arrived>,
    path: String,
    info: Info,
    sessions: HashMap<String, Session>,
    clients: Vec<Client>,
    /// The AI apps the program disconnected (lowercase names): refused
    /// until the server starts again.
    shut_out: Vec<String>,
    calls: HashMap<i64, Call>,
    /// Calls whose OnToolCall / OnReadResource hasn't run yet.
    queue: VecDeque<i64>,
    busy: Rc<Cell<bool>>,
    idle: Duration,
}

/// What the server says it is.
#[derive(Clone, Default)]
struct Info {
    name: String,
    title: String,
    version: String,
    instructions: String,
}

thread_local! {
    static SERVERS: RefCell<HashMap<String, Server>> = RefCell::new(HashMap::new());
    static DEFS: RefCell<HashMap<String, Defs>> = RefCell::new(HashMap::new());
}

fn with_server<R>(name: &str, f: impl FnOnce(&mut Server) -> R) -> Option<R> {
    SERVERS.with(|s| s.borrow_mut().get_mut(name).map(f))
}

fn with_defs<R>(name: &str, f: impl FnOnce(&mut Defs) -> R) -> R {
    DEFS.with(|d| f(d.borrow_mut().entry(name.to_string()).or_default()))
}

fn prop_text(name: &str, prop: &str) -> String {
    rp_comp_get(name, prop).to_string_val()
}

/// RMcpServer `name`'s method.
pub fn method(name: &str, method: &str, args: &[Value]) -> Value {
    let name = name.to_lowercase();
    let text = |i: usize| args.get(i).map(Value::to_string_val).unwrap_or_default();
    let flag = |i: usize| args.get(i).is_some_and(Value::to_bool);
    let call = |i: usize| call_arg(&name, args, i);
    match method {
        "start" => start(&name),
        "stop" => {
            stop(&name);
            v_null()
        }
        "newtoken" => v_str(&new_token(&name)),
        "addtool" => v_int(add_tool(&name, &text(0), &text(1), &text(2), &text(3)) as i64),
        "removetool" => v_int(with_defs(&name, |d| {
            let n = d.tools.len();
            let t = text(0);
            d.tools.retain(|x| !x.name.eq_ignore_ascii_case(&t));
            d.tools.len() != n
        }) as i64),
        "cleartools" => {
            with_defs(&name, |d| d.tools.clear());
            v_null()
        }
        "toolcount" => v_int(with_defs(&name, |d| d.tools.len()) as i64),
        "addresource" => v_int(add_resource(&name, &text(0), &text(1), &text(2), &text(3)) as i64),
        "clearresources" => {
            with_defs(&name, |d| d.resources.clear());
            v_null()
        }
        "reply" => v_int(reply_text(&name, call(0), &text(1), flag(2)) as i64),
        "replyimage" => v_int(reply_image(&name, call(0), &text(1), &text(2)) as i64),
        "replyjson" => v_int(reply_json(&name, call(0), &text(1)) as i64),
        "progress" => v_int(progress(&name, call(0), &text(1), args.get(2).map(Value::to_f64), args.get(3).map(Value::to_f64)) as i64),
        "argument" => v_str(&argument(&name, &text(0), call_arg(&name, args, 1))),
        "isopen" => v_int(with_server(&name, |s| s.calls.contains_key(&call(0))).unwrap_or(false) as i64),
        "disconnect" => v_int(disconnect(&name, &text(0)) as i64),
        _ => {
            eprintln!("[WARN] RMcpServer.{method}() not implemented");
            v_null()
        }
    }
}

/// A method's Call argument (at `at`), else the call being handled.
fn call_arg(name: &str, args: &[Value], at: usize) -> i64 {
    match args.get(at) {
        Some(v) if !matches!(v, Value::Null) && !v.to_string_val().is_empty() => v.to_i64(),
        _ => rp_comp_get(name, "call").to_i64(),
    }
}

fn new_token(name: &str) -> String {
    match httpserver::random_token() {
        Some(t) => {
            rp_comp_set(name, "token", v_str(&t));
            t
        }
        None => {
            rp_comp_set(name, "error", v_str("the system's secure random source gave nothing: no token was made"));
            String::new()
        }
    }
}

/// AddTool: a tool named `tool` (letters, digits, `_`, `-`, `.`), what it
/// does, the JSON Schema of its arguments ("" — none) and how it acts:
/// "read" (it only reads), "change", "delete" … or annotations as JSON.
/// Added again: replaced, in its place. False (Error says why) when the
/// name or the schema isn't right.
fn add_tool(name: &str, tool: &str, description: &str, schema: &str, how: &str) -> bool {
    let tool = tool.trim();
    let fail = |why: String| {
        rp_comp_set(name, "error", v_str(&why));
        false
    };
    if tool.is_empty() || tool.len() > 128 || !tool.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')) {
        return fail(format!("AddTool: \"{tool}\" isn't a tool name (1 to 128 letters, digits, _ - .)"));
    }
    let schema = if schema.trim().is_empty() {
        // ("properties": {} said: valid MCP without it, but a model server that builds a grammar from the
        // tool's schema — Ollama, for some models — refuses a list with `properties must be an object`:
        // found by tools/ai_eval, where one of eight local models couldn't use any of the 26 tools)
        json!({"type": "object", "properties": {}, "additionalProperties": false})
    } else if !schema.trim_start().starts_with('{') {
        // (a SUB's parameter list: `Text AS STRING, [Count AS INTEGER]`)
        match params_schema(schema) {
            Ok(v) => v,
            Err(why) => return fail(format!("AddTool {tool}: {why}")),
        }
    } else {
        match serde_json::from_str::<Json>(schema) {
            Ok(v) if v.is_object() => v,
            Ok(_) => return fail(format!("AddTool {tool}: its schema must be a JSON object ({{\"type\": \"object\", \"properties\": …}})")),
            Err(e) => return fail(format!("AddTool {tool}: its schema isn't JSON ({e})")),
        }
    };
    let how = how.trim();
    let annotations = if how.starts_with('{') {
        match serde_json::from_str::<Json>(how) {
            Ok(v) if v.is_object() => Some(v),
            _ => return fail(format!("AddTool {tool}: its annotations aren't a JSON object")),
        }
    } else {
        match how.to_ascii_lowercase().as_str() {
            "" => None,
            "read" => Some(json!({"readOnlyHint": true, "openWorldHint": false})),
            "change" => Some(json!({"readOnlyHint": false, "destructiveHint": false, "openWorldHint": false})),
            "delete" | "destructive" => Some(json!({"readOnlyHint": false, "destructiveHint": true, "openWorldHint": false})),
            "network" => Some(json!({"readOnlyHint": false, "openWorldHint": true})),
            other => return fail(format!("AddTool {tool}: \"{other}\" isn't read, change, delete, network or annotations in JSON")),
        }
    };
    let title = annotations.as_ref().and_then(|a| a.get("title")).and_then(Json::as_str).unwrap_or("").to_string();
    let def = ToolDef { name: tool.to_string(), description: description.to_string(), schema, annotations, title };
    with_defs(name, |d| match d.tools.iter_mut().find(|t| t.name == def.name) {
        Some(t) => *t = def,
        None => d.tools.push(def),
    });
    true
}

/// A tool's arguments written as a SUB's parameters — `Text AS STRING,
/// [Count AS INTEGER]`, `[ ]` around the optional ones — as the JSON
/// Schema AI apps read. STRING; INTEGER, LONG, SHORT, BYTE, WORD, DWORD;
/// SINGLE, DOUBLE, NUMBER; BOOLEAN; LIST; OBJECT.
fn params_schema(text: &str) -> Result<Json, String> {
    let mut props = Map::new();
    let mut required = Vec::new();
    for part in text.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let optional = part.starts_with('[') && part.ends_with(']');
        let part = part.trim_start_matches('[').trim_end_matches(']').trim();
        let words: Vec<&str> = part.split_whitespace().collect();
        let (key, kind) = match words.as_slice() {
            [k] => (*k, "STRING"),
            [k, as_, t] if as_.eq_ignore_ascii_case("as") => (*k, *t),
            _ => return Err(format!("\"{part}\" isn't Name AS TYPE")),
        };
        let key = key.trim_end_matches('$');
        if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(format!("\"{key}\" isn't an argument's name"));
        }
        let t = match kind.to_ascii_uppercase().as_str() {
            "STRING" => "string",
            "INTEGER" | "LONG" | "SHORT" | "BYTE" | "WORD" | "DWORD" | "INT64" => "integer",
            "SINGLE" | "DOUBLE" | "NUMBER" => "number",
            "BOOLEAN" | "BOOL" => "boolean",
            "LIST" | "ARRAY" => "array",
            "OBJECT" => "object",
            other => return Err(format!("{key}'s type {other} isn't STRING, INTEGER, DOUBLE, BOOLEAN, LIST or OBJECT")),
        };
        props.insert(key.to_string(), json!({"type": t}));
        if !optional {
            required.push(json!(key));
        }
    }
    Ok(json!({"type": "object", "properties": props, "required": required, "additionalProperties": false}))
}

fn add_resource(name: &str, uri: &str, title: &str, description: &str, mime: &str) -> bool {
    let uri = uri.trim();
    if uri.is_empty() || !uri.contains(':') {
        rp_comp_set(name, "error", v_str(&format!("AddResource: \"{uri}\" isn't a URI (scheme:…, as file:///notes.txt or app://status)")));
        return false;
    }
    let def = ResourceDef { uri: uri.to_string(), name: if title.trim().is_empty() { uri.to_string() } else { title.trim().to_string() }, description: description.to_string(), mime: if mime.trim().is_empty() { "text/plain".into() } else { mime.trim().to_string() } };
    with_defs(name, |d| match d.resources.iter_mut().find(|r| r.uri == def.uri) {
        Some(r) => *r = def,
        None => d.resources.push(def),
    });
    true
}

fn failed(name: &str, why: String) -> Value {
    eprintln!("[McpServer] {why}");
    rp_comp_set(name, "error", v_str(&why));
    rp_comp_set(name, "listening", v_int(0));
    v_int(0)
}

fn start(name: &str) -> Value {
    stop(name);
    let mut options = ListenOptions::of(name);
    if options.token.is_empty() {
        return failed(name, "Set a Token before Start (NewToken makes one): an MCP server always needs one, so only the AI apps you give it to can call the program's tools.".into());
    }
    if options.token.len() < 16 {
        return failed(name, "The Token is too short to keep anyone out: use NewToken (64 random hex digits).".into());
    }
    // (an MCP client is never a web page: only the server's own origin)
    if options.origins.trim().is_empty() {
        options.origins = String::new();
    }
    let l = match httpserver::listen(&options) {
        Ok(l) => l,
        Err(why) => return failed(name, why),
    };
    let mut path = prop_text(name, "path").trim().to_string();
    if path.is_empty() {
        path = "/mcp".into();
    }
    if !path.starts_with('/') {
        path.insert(0, '/');
    }
    let idle = rp_comp_get(name, "idleseconds").to_f64();
    let info = Info {
        name: {
            let n = prop_text(name, "servername").trim().to_string();
            if n.is_empty() { "rapidr-app".into() } else { n }
        },
        title: prop_text(name, "servertitle").trim().to_string(),
        version: {
            let v = prop_text(name, "serverversion").trim().to_string();
            if v.is_empty() { "1.0".into() } else { v }
        },
        instructions: prop_text(name, "instructions"),
    };
    let url = format!("{}{}", l.url.trim_end_matches('/'), path);
    SERVERS.with(|s| {
        s.borrow_mut().insert(
            name.to_string(),
            Server {
                running: l.running,
                events: l.events,
                path,
                info,
                sessions: HashMap::new(),
                clients: Vec::new(),
                shut_out: Vec::new(),
                calls: HashMap::new(),
                queue: VecDeque::new(),
                busy: Rc::new(Cell::new(false)),
                idle: Duration::from_secs_f64(if idle > 0.0 { idle } else { 15.0 * 60.0 }),
            },
        )
    });
    rp_comp_set(name, "port", v_int(i64::from(l.port)));
    rp_comp_set(name, "url", v_str(&url));
    rp_comp_set(name, "error", v_str(""));
    rp_comp_set(name, "listening", v_int(1));
    publish_clients(name);
    v_int(1)
}

fn stop(name: &str) {
    if let Some(mut s) = SERVERS.with(|s| s.borrow_mut().remove(name)) {
        s.running.store(false, Ordering::Relaxed);
        // (each call still waiting: told, then closed)
        for (_, c) in s.calls.drain() {
            let answer = error_message(&c.id, INTERNAL_ERROR, "The server stopped before this call was answered.");
            finish_call(c, &answer);
        }
        rp_comp_set(name, "listening", v_int(0));
        rp_comp_set(name, "clients", v_str(""));
        rp_comp_set(name, "clientcount", v_int(0));
        rp_comp_set(name, "pendingcount", v_int(0));
    }
}

/// Whether a server is listening.
pub fn busy() -> bool {
    SERVERS.with(|s| !s.borrow().is_empty())
}

/// The window loop's turn (and DOEVENTS): the messages that arrived are
/// answered, the next call goes to the program.
pub fn poll() -> bool {
    let names: Vec<String> = SERVERS.with(|s| s.borrow().keys().cloned().collect());
    for name in &names {
        poll_one(name);
    }
    !names.is_empty()
}

fn poll_one(name: &str) {
    loop {
        let Some(a) = with_server(name, |s| s.events.try_recv().ok()).flatten() else { break };
        arrived(name, a);
    }
    idle_check(name);
    next_call(name);
}

// ---------------------------------------------------------------- answers --

/// A JSON-RPC message as ASCII: every other character a `\u` escape (it
/// only appears inside strings), so it means the same whatever the client
/// takes the bytes to be.
fn ascii(json: &Json) -> String {
    let text = json.to_string();
    if text.is_ascii() {
        return text;
    }
    let mut out = String::with_capacity(text.len() + 16);
    for c in text.chars() {
        if c.is_ascii() {
            out.push(c);
        } else {
            let mut units = [0u16; 2];
            for u in c.encode_utf16(&mut units) {
                out.push_str(&format!("\\u{u:04x}"));
            }
        }
    }
    out
}

fn result_message(id: &Json, result: Json) -> Json {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

fn error_message(id: &Json, code: i64, message: &str) -> Json {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

fn error_data(id: &Json, code: i64, message: &str, data: Json) -> Json {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message, "data": data}})
}

fn send_json(a: &Arrived, status: u16, extra: &[(&str, &str)], message: &Json) {
    let mut headers = vec![("Content-Type".to_string(), "application/json".to_string())];
    headers.extend(extra.iter().map(|(n, v)| (n.to_string(), v.to_string())));
    httpserver::write_response(&a.conn, &a.request, status, headers, ascii(message).as_bytes());
}

fn send_plain(a: &Arrived, status: u16, extra: &[(&str, &str)], body: &str) {
    let headers = extra.iter().map(|(n, v)| (n.to_string(), v.to_string())).collect();
    httpserver::write_response(&a.conn, &a.request, status, headers, body.as_bytes());
}

/// A call's answer written: as the last event of its stream, or as JSON.
fn finish_call(c: Call, message: &Json) -> bool {
    let text = ascii(message);
    match c.stream {
        Some(chunked) => httpserver::write_event(&c.conn, chunked, &text, "message") && httpserver::write_stream_end(&c.conn, &c.request, chunked),
        None => httpserver::write_response(&c.conn, &c.request, 200, vec![("Content-Type".into(), "application/json".into())], text.as_bytes()),
    }
}

/// The program's answer to call `id`: its result (a tool's, a resource's).
fn answer(name: &str, id: i64, mut result: Json, failed_text: Option<&str>) -> bool {
    let Some(c) = with_server(name, |s| s.calls.remove(&id)).flatten() else { return false };
    let message = match (c.kind, failed_text) {
        (Kind::Resource, Some(why)) => error_message(&c.id, INTERNAL_ERROR, why),
        _ => {
            if c.modern {
                if let Some(o) = result.as_object_mut() {
                    o.entry("resultType").or_insert(json!("complete"));
                }
            }
            result_message(&c.id, result)
        }
    };
    let ok = finish_call(c, &message);
    let left = with_server(name, |s| s.calls.len()).unwrap_or(0);
    rp_comp_set(name, "pendingcount", v_int(left as i64));
    ok
}

/// Reply: a call answered with text (a tool's result, marked as an error
/// when `failed`; a resource's contents).
fn reply_text(name: &str, id: i64, text: &str, failed: bool) -> bool {
    let Some((kind, uri, mime)) = with_server(name, |s| {
        s.calls.get(&id).map(|c| {
            let mime = with_defs(name, |d| d.resources.iter().find(|r| r.uri == c.name).map(|r| r.mime.clone())).unwrap_or_else(|| "text/plain".into());
            (c.kind, c.name.clone(), mime)
        })
    })
    .flatten() else {
        return false;
    };
    match kind {
        Kind::Tool => {
            let mut r = json!({"content": [{"type": "text", "text": text}]});
            if failed {
                r["isError"] = json!(true);
            }
            answer(name, id, r, None)
        }
        Kind::Resource if failed => answer(name, id, Json::Null, Some(text)),
        Kind::Resource => answer(name, id, json!({"contents": [{"uri": uri, "mimeType": mime, "text": text}]}), None),
    }
}

/// The MIME type of an image's bytes, by its signature.
fn image_mime(b: &[u8]) -> Option<&'static str> {
    if b.starts_with(b"\x89PNG") {
        Some("image/png")
    } else if b.starts_with(&[0xFF, 0xD8]) {
        Some("image/jpeg")
    } else if b.starts_with(b"GIF8") {
        Some("image/gif")
    } else if b.len() > 12 && &b[..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

/// ReplyImage: a tool's answer with a picture (a PNG, JPEG, GIF or WebP
/// file; a BMP — a capture's — goes as PNG) and, when given, a text.
fn reply_image(name: &str, id: i64, file: &str, text: &str) -> bool {
    let bytes = match std::fs::read(file) {
        Ok(b) => b,
        Err(e) => return reply_text(name, id, &format!("The picture couldn't be read ({e})."), true),
    };
    let (data, mime) = match image_mime(&bytes) {
        Some(m) => (bytes, m),
        None if bytes.starts_with(b"BM") => match rapidr_value::objects::codec::decode_bmp(&bytes).and_then(|img| rapidr_value::objects::codec::encode_png(&img)) {
            Ok(png) => (png, "image/png"),
            Err(e) => return reply_text(name, id, &format!("The picture couldn't be read ({e})."), true),
        },
        None => return reply_text(name, id, "The picture isn't a PNG, JPEG, GIF, WebP or BMP image.", true),
    };
    let mut content = Vec::new();
    if !text.is_empty() {
        content.push(json!({"type": "text", "text": text}));
    }
    content.push(json!({"type": "image", "data": rapidr_value::objects::codec::base64_encode(&data), "mimeType": mime}));
    answer(name, id, json!({"content": content}), None)
}

/// ReplyJson: a call answered with a whole result object, as written.
fn reply_json(name: &str, id: i64, text: &str) -> bool {
    match serde_json::from_str::<Json>(text) {
        Ok(v) if v.is_object() => answer(name, id, v, None),
        _ => {
            rp_comp_set(name, "error", v_str("ReplyJson: the result must be a JSON object ({\"content\": […]})"));
            false
        }
    }
}

/// Progress: a word on how a call goes, while it waits — as a progress
/// notification (when the client asked for them) or a log message, on the
/// call's event stream. False when the client takes no stream or left.
fn progress(name: &str, id: i64, message: &str, done: Option<f64>, total: Option<f64>) -> bool {
    with_server(name, |s| {
        let Some(c) = s.calls.get_mut(&id) else { return false };
        if c.stream.is_none() {
            if !c.events {
                return false;
            }
            match httpserver::write_stream_head(&c.conn, &c.request, "") {
                Some(chunked) => c.stream = Some(chunked),
                None => return false,
            }
        }
        let note = match &c.progress {
            Some(token) => {
                let mut p = json!({"progressToken": token, "progress": done.unwrap_or(0.0), "message": message});
                if let Some(t) = total.filter(|t| *t > 0.0) {
                    p["total"] = json!(t);
                }
                json!({"jsonrpc": "2.0", "method": "notifications/progress", "params": p})
            }
            None => json!({"jsonrpc": "2.0", "method": "notifications/message", "params": {"level": "info", "logger": s.info.name, "data": message}}),
        };
        httpserver::write_event(&c.conn, c.stream.unwrap_or(true), &ascii(&note), "message")
    })
    .unwrap_or(false)
}

/// Argument: one of a call's arguments as text — a string as it is, a
/// number or true/false as written, a list or an object as JSON, "" when
/// it wasn't given. `a.b` reaches inside an object.
fn argument(name: &str, key: &str, id: i64) -> String {
    with_server(name, |s| {
        let c = s.calls.get(&id)?;
        let mut v = &c.args;
        for part in key.split('.') {
            v = v.get(part)?;
        }
        Some(match v {
            Json::String(t) => t.clone(),
            Json::Null => String::new(),
            other => other.to_string(),
        })
    })
    .flatten()
    .unwrap_or_default()
}

// -------------------------------------------------------------- clients --

fn client_title(info: Option<&Json>) -> (String, String) {
    let field = |k: &str| info.and_then(|i| i.get(k)).and_then(Json::as_str).map(str::trim).filter(|t| !t.is_empty()).map(|t| t.chars().take(80).collect::<String>());
    let name = field("title").or_else(|| field("name")).unwrap_or_else(|| "an AI app".into());
    (name, field("version").unwrap_or_default())
}

/// The program's Clients / ClientCount, as they are now.
fn publish_clients(name: &str) {
    let lines = with_server(name, |s| {
        let now = Instant::now();
        s.clients.iter().map(|c| format!("{}\t{}\t{}\t{}", c.name, c.version, now.duration_since(c.last).as_secs(), c.calls)).collect::<Vec<_>>()
    })
    .unwrap_or_default();
    rp_comp_set(name, "clientcount", v_int(lines.len() as i64));
    rp_comp_set(name, "clients", v_str(&lines.join("\n")));
}

/// A request from `client`: it is in (OnConnect the first time).
fn heard(name: &str, client: &str, version: &str, call: bool) {
    let new = with_server(name, |s| match s.clients.iter_mut().find(|c| c.name == client) {
        Some(c) => {
            c.last = Instant::now();
            c.calls += u64::from(call);
            false
        }
        None => {
            s.clients.push(Client { name: client.to_string(), version: version.to_string(), last: Instant::now(), calls: u64::from(call) });
            true
        }
    })
    .unwrap_or(false);
    publish_clients(name);
    if new && crate::object::rp_has_handler(name, "onconnect") {
        crate::object::rp_fire_event_then(name, "onconnect", &[v_str(client)], |_| {});
    }
}

/// Clients quiet for IdleSeconds (and with no call waiting) are gone:
/// OnDisconnect.
fn idle_check(name: &str) {
    let gone = with_server(name, |s| {
        let pending: Vec<String> = s.calls.values().map(|c| c.client.clone()).collect();
        let idle = s.idle;
        let (gone, kept): (Vec<Client>, Vec<Client>) = std::mem::take(&mut s.clients).into_iter().partition(|c| c.last.elapsed() > idle && !pending.contains(&c.name));
        s.clients = kept;
        gone.into_iter().map(|c| c.name).collect::<Vec<_>>()
    })
    .unwrap_or_default();
    if gone.is_empty() {
        return;
    }
    publish_clients(name);
    for c in gone {
        if crate::object::rp_has_handler(name, "ondisconnect") {
            crate::object::rp_fire_event_then(name, "ondisconnect", &[v_str(&c)], |_| {});
        }
    }
}

/// Disconnect: the AI app named (every one, when "") is out until the
/// server starts again — its sessions end, its calls waiting are told,
/// its next requests refused (403). The token stays: a new token (and a
/// Start) is what keeps everyone out for sure.
fn disconnect(name: &str, who: &str) -> bool {
    let who = who.trim().to_lowercase();
    let out = with_server(name, |s| {
        let names: Vec<String> = s.clients.iter().map(|c| c.name.clone()).filter(|n| who.is_empty() || n.to_lowercase() == who).collect();
        let mut out = names.clone();
        if !who.is_empty() && !out.iter().any(|n| n.to_lowercase() == who) {
            out.push(who.clone());
        }
        for n in &out {
            let low = n.to_lowercase();
            if !s.shut_out.contains(&low) {
                s.shut_out.push(low);
            }
        }
        s.sessions.retain(|_, x| !out.iter().any(|n| n.eq_ignore_ascii_case(&x.client)));
        s.clients.retain(|c| !out.iter().any(|n| n.eq_ignore_ascii_case(&c.name)));
        let ids: Vec<i64> = s.calls.iter().filter(|(_, c)| out.iter().any(|n| n.eq_ignore_ascii_case(&c.client))).map(|(k, _)| *k).collect();
        for id in ids {
            if let Some(c) = s.calls.remove(&id) {
                let m = error_message(&c.id, SESSION_REFUSED, "Disconnected by the person using the app.");
                finish_call(c, &m);
            }
        }
        s.queue.retain(|id| s.calls.contains_key(id));
        names
    });
    let Some(names) = out else { return false };
    publish_clients(name);
    for n in &names {
        if crate::object::rp_has_handler(name, "ondisconnect") {
            crate::object::rp_fire_event_then(name, "ondisconnect", &[v_str(n)], |_| {});
        }
    }
    true
}

// ------------------------------------------------------------- requests --

/// A request that arrived, answered — or, a call, kept for the program.
fn arrived(name: &str, a: Arrived) {
    let path = with_server(name, |s| s.path.clone()).unwrap_or_default();
    if a.request.path() != path {
        return send_plain(&a, 404, &[], &format!("AI apps connect at {path}.\n"));
    }
    match a.request.method.to_ascii_uppercase().as_str() {
        "POST" => post(name, a),
        "DELETE" => {
            // (an earlier revision's session ended)
            let sid = a.request.header("mcp-session-id").unwrap_or("").trim().to_string();
            let gone = with_server(name, |s| s.sessions.remove(&sid)).flatten();
            match gone {
                Some(x) => {
                    let still = with_server(name, |s| s.sessions.values().any(|o| o.client == x.client)).unwrap_or(false);
                    send_plain(&a, 200, &[], "");
                    if !still {
                        with_server(name, |s| s.clients.retain(|c| c.name != x.client));
                        publish_clients(name);
                        if crate::object::rp_has_handler(name, "ondisconnect") {
                            crate::object::rp_fire_event_then(name, "ondisconnect", &[v_str(&x.client)], |_| {});
                        }
                    }
                }
                None if sid.is_empty() => send_plain(&a, 405, &[("Allow", "POST")], "Send each message as a POST.\n"),
                None => send_json(&a, 404, &[], &error_message(&Json::Null, SESSION_ENDED, "No such session.")),
            }
        }
        // (the earlier revisions' stream of the server's own messages: this
        // server sends none — they allow 405)
        _ => send_plain(&a, 405, &[("Allow", "POST, DELETE")], "AI apps POST their messages here.\n"),
    }
}

fn post(name: &str, a: Arrived) {
    let body: Json = match serde_json::from_slice(&a.request.body) {
        Ok(v) => v,
        Err(e) => return send_json(&a, 400, &[], &error_message(&Json::Null, PARSE_ERROR, &format!("The body isn't JSON ({e})."))),
    };
    if !body.is_object() || body.get("jsonrpc").and_then(Json::as_str) != Some("2.0") {
        return send_json(&a, 400, &[], &error_message(&Json::Null, INVALID_REQUEST, "Send one JSON-RPC 2.0 message (an object) per request."));
    }
    let method = body.get("method").and_then(Json::as_str).unwrap_or("").to_string();
    let id = body.get("id").cloned();
    let header_version = a.request.header("mcp-protocol-version").map(|v| v.trim().to_string());
    let sid = a.request.header("mcp-session-id").map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
    let meta_version = body.pointer("/params/_meta").and_then(|m| m.get(META_VERSION)).and_then(Json::as_str).map(str::to_string);

    // (an earlier revision: initialize starts a session, which the rest name)
    if method == "initialize" {
        return initialize(name, &a, &body);
    }
    // (a session's request, unless it speaks the current revision — which
    // has no sessions: its Mcp-Session-Id is ignored)
    if let Some(sid) = sid.filter(|_| !header_version.as_deref().is_some_and(|v| MODERN.contains(&v))) {
        return legacy(name, a, &body, &sid);
    }
    let modern = match (&header_version, &meta_version) {
        (Some(v), _) => !LEGACY.contains(&v.as_str()),
        (None, Some(_)) => true,
        (None, None) => false,
    };
    if !modern {
        let idj = id.unwrap_or(Json::Null);
        return send_json(&a, 400, &[], &error_message(&idj, SESSION_REFUSED, "Start a session with initialize (then send its Mcp-Session-Id), or speak revision 2026-07-28: an MCP-Protocol-Version header and _meta on every request."));
    }
    modern_request(name, a, &body, header_version, meta_version);
}

/// The value of an Mcp-Name / Mcp-Param header, decoded from its Base64
/// form (`=?base64?…?=`).
fn header_value(v: &str) -> Option<String> {
    match v.strip_prefix("=?base64?").and_then(|r| r.strip_suffix("?=")) {
        Some(b64) => rapidr_value::objects::codec::base64_decode(b64).and_then(|b| String::from_utf8(b).ok()),
        None => Some(v.to_string()),
    }
}

/// A request of the current revision: its headers checked against its
/// body, then answered.
fn modern_request(name: &str, a: Arrived, body: &Json, header_version: Option<String>, meta_version: Option<String>) {
    let id = body.get("id").cloned();
    let idj = id.clone().unwrap_or(Json::Null);
    let method = body.get("method").and_then(Json::as_str).unwrap_or("").to_string();
    let Some(version) = header_version else {
        return send_json(&a, 400, &[], &error_message(&idj, HEADER_MISMATCH, "Header mismatch: the MCP-Protocol-Version header is missing."));
    };
    if !MODERN.contains(&version.as_str()) {
        return send_json(&a, 400, &[], &error_data(&idj, UNSUPPORTED_VERSION, "Unsupported protocol version", json!({"supported": supported(), "requested": version})));
    }
    if meta_version.as_deref() != Some(version.as_str()) {
        return send_json(&a, 400, &[], &error_message(&idj, HEADER_MISMATCH, &format!("Header mismatch: MCP-Protocol-Version {version} doesn't match _meta's {}.", meta_version.unwrap_or_else(|| "(none)".into()))));
    }
    if id.is_some() {
        match a.request.header("mcp-method").map(str::trim) {
            Some(m) if m == method => {}
            Some(m) => return send_json(&a, 400, &[], &error_message(&idj, HEADER_MISMATCH, &format!("Header mismatch: Mcp-Method header value '{m}' does not match body value '{method}'."))),
            None => return send_json(&a, 400, &[], &error_message(&idj, HEADER_MISMATCH, "Header mismatch: the Mcp-Method header is missing.")),
        }
        let named = match method.as_str() {
            "tools/call" | "prompts/get" => body.pointer("/params/name").and_then(Json::as_str),
            "resources/read" => body.pointer("/params/uri").and_then(Json::as_str),
            _ => None,
        };
        if let Some(n) = named {
            match a.request.header("mcp-name").map(|h| header_value(h.trim())) {
                Some(Some(h)) if h == n => {}
                Some(Some(h)) => return send_json(&a, 400, &[], &error_message(&idj, HEADER_MISMATCH, &format!("Header mismatch: Mcp-Name header value '{h}' does not match body value '{n}'."))),
                Some(None) => return send_json(&a, 400, &[], &error_message(&idj, HEADER_MISMATCH, "Header mismatch: the Mcp-Name header's Base64 isn't valid.")),
                None => return send_json(&a, 400, &[], &error_message(&idj, HEADER_MISMATCH, "Header mismatch: the Mcp-Name header is missing.")),
            }
        }
    }
    let (client, version_of) = client_title(body.pointer("/params/_meta").and_then(|m| m.get(META_CLIENT)));
    if shut_out(name, &client) {
        return send_json(&a, 403, &[], &error_message(&idj, SESSION_REFUSED, "Disconnected by the person using the app: they can let AI apps in again by turning the server off and on."));
    }
    // (a notification: nothing to say back)
    let Some(id) = id else {
        return send_plain(&a, 202, &[], "");
    };
    heard(name, &client, &version_of, method == "tools/call");
    match method.as_str() {
        "server/discover" => {
            let (info, caps) = describe(name);
            let mut r = json!({"resultType": "complete", "supportedVersions": supported(), "capabilities": caps, "_meta": {META_SERVER: server_info(&info)}});
            if !info.instructions.trim().is_empty() {
                r["instructions"] = json!(info.instructions);
            }
            send_json(&a, 200, &[], &result_message(&id, r));
        }
        "ping" => send_json(&a, 200, &[], &result_message(&id, json!({"resultType": "complete"}))),
        "tools/list" => send_json(&a, 200, &[], &result_message(&id, with_type(tools_list(name), true))),
        "resources/list" => send_json(&a, 200, &[], &result_message(&id, with_type(resources_list(name), true))),
        "resources/templates/list" => send_json(&a, 200, &[], &result_message(&id, json!({"resultType": "complete", "resourceTemplates": []}))),
        "prompts/list" => send_json(&a, 200, &[], &result_message(&id, json!({"resultType": "complete", "prompts": []}))),
        "logging/setLevel" => send_json(&a, 200, &[], &result_message(&id, json!({"resultType": "complete"}))),
        "tools/call" | "resources/read" => call(name, a, body, id, client, true),
        _ => send_json(&a, 404, &[], &error_message(&id, METHOD_NOT_FOUND, &format!("Method not found: {method}"))),
    }
}

fn with_type(mut r: Json, modern: bool) -> Json {
    if modern {
        r["resultType"] = json!("complete");
    }
    r
}

fn shut_out(name: &str, client: &str) -> bool {
    with_server(name, |s| s.shut_out.contains(&client.to_lowercase())).unwrap_or(false)
}

fn server_info(info: &Info) -> Json {
    let mut i = json!({"name": info.name, "version": info.version});
    if !info.title.trim().is_empty() {
        i["title"] = json!(info.title);
    }
    i
}

/// The server's info and capabilities: tools always; resources when it
/// has some; logging (its messages on a call's stream).
fn describe(name: &str) -> (Info, Json) {
    let info = with_server(name, |s| s.info.clone()).unwrap_or_default();
    let mut caps = json!({"tools": {}, "logging": {}});
    if with_defs(name, |d| !d.resources.is_empty()) {
        caps["resources"] = json!({});
    }
    (info, caps)
}

fn tools_list(name: &str) -> Json {
    let tools: Vec<Json> = with_defs(name, |d| {
        d.tools
            .iter()
            .map(|t| {
                let mut j = json!({"name": t.name, "description": t.description, "inputSchema": t.schema});
                if !t.title.is_empty() {
                    j["title"] = json!(t.title);
                }
                if let Some(a) = &t.annotations {
                    j["annotations"] = a.clone();
                }
                j
            })
            .collect()
    });
    json!({"tools": tools})
}

fn resources_list(name: &str) -> Json {
    let list: Vec<Json> = with_defs(name, |d| d.resources.iter().map(|r| json!({"uri": r.uri, "name": r.name, "description": r.description, "mimeType": r.mime})).collect());
    json!({"resources": list})
}

/// An earlier revision's initialize: a session begins.
fn initialize(name: &str, a: &Arrived, body: &Json) {
    let id = body.get("id").cloned().unwrap_or(Json::Null);
    let asked = body.pointer("/params/protocolVersion").and_then(Json::as_str).unwrap_or("");
    let version = if LEGACY.contains(&asked) { asked.to_string() } else { LEGACY[0].to_string() };
    let (client, client_version) = client_title(body.pointer("/params/clientInfo"));
    if shut_out(name, &client) {
        return send_json(a, 403, &[], &error_message(&id, SESSION_REFUSED, "Disconnected by the person using the app: they can let AI apps in again by turning the server off and on."));
    }
    let Some(sid) = httpserver::random_token().map(|t| t[..32].to_string()) else {
        return send_json(a, 500, &[], &error_message(&id, INTERNAL_ERROR, "The system's secure random source gave nothing: no session could start."));
    };
    with_server(name, |s| s.sessions.insert(sid.clone(), Session { client: client.clone(), protocol: version.clone() }));
    let (info, caps) = describe(name);
    let mut r = json!({"protocolVersion": version, "capabilities": caps, "serverInfo": server_info(&info)});
    if !info.instructions.trim().is_empty() {
        r["instructions"] = json!(info.instructions);
    }
    send_json(a, 200, &[("Mcp-Session-Id", &sid)], &result_message(&id, r));
    heard(name, &client, &client_version, false);
}

/// A request in an earlier revision's session.
fn legacy(name: &str, a: Arrived, body: &Json, sid: &str) {
    let id = body.get("id").cloned();
    let idj = id.clone().unwrap_or(Json::Null);
    let method = body.get("method").and_then(Json::as_str).unwrap_or("").to_string();
    let Some((client, _protocol)) = with_server(name, |s| s.sessions.get(sid).map(|x| (x.client.clone(), x.protocol.clone()))).flatten() else {
        return send_json(&a, 404, &[], &error_message(&idj, SESSION_ENDED, "This session ended (the app was closed, or its server turned off and on): start a new one with initialize."));
    };
    if shut_out(name, &client) {
        return send_json(&a, 403, &[], &error_message(&idj, SESSION_REFUSED, "Disconnected by the person using the app."));
    }
    // (a notification, or the client's answer to a request: nothing to say)
    let Some(id) = id.filter(|_| !method.is_empty()) else {
        return send_plain(&a, 202, &[], "");
    };
    heard(name, &client, "", method == "tools/call");
    match method.as_str() {
        "ping" | "logging/setLevel" => send_json(&a, 200, &[], &result_message(&id, json!({}))),
        "tools/list" => send_json(&a, 200, &[], &result_message(&id, tools_list(name))),
        "resources/list" => send_json(&a, 200, &[], &result_message(&id, resources_list(name))),
        "resources/templates/list" => send_json(&a, 200, &[], &result_message(&id, json!({"resourceTemplates": []}))),
        "prompts/list" => send_json(&a, 200, &[], &result_message(&id, json!({"prompts": []}))),
        "tools/call" | "resources/read" => call(name, a, body, id, client, false),
        _ => send_json(&a, 200, &[], &error_message(&id, METHOD_NOT_FOUND, &format!("Method not found: {method}"))),
    }
}

/// A tools/call or resources/read: checked, then kept for the program.
fn call(name: &str, a: Arrived, body: &Json, id: Json, client: String, modern: bool) {
    let params = body.get("params").cloned().unwrap_or(json!({}));
    let read = body.get("method").and_then(Json::as_str) == Some("resources/read");
    let events = a.request.header("accept").is_some_and(|h| h.to_ascii_lowercase().contains("text/event-stream"));
    let progress = params.pointer("/_meta/progressToken").cloned().filter(|t| t.is_string() || t.is_number());
    let (kind, target, args) = if read {
        let uri = params.get("uri").and_then(Json::as_str).unwrap_or("").to_string();
        if !with_defs(name, |d| d.resources.iter().any(|r| r.uri == uri)) {
            return send_json(&a, 200, &[], &error_data(&id, RESOURCE_NOT_FOUND, "Resource not found", json!({"uri": uri})));
        }
        (Kind::Resource, uri, json!({}))
    } else {
        let tool = params.get("name").and_then(Json::as_str).unwrap_or("").to_string();
        let Some(def) = with_defs(name, |d| d.tools.iter().find(|t| t.name == tool).cloned()) else {
            let known = with_defs(name, |d| d.tools.iter().map(|t| t.name.clone()).collect::<Vec<_>>().join(", "));
            return send_json(&a, 200, &[], &error_message(&id, INVALID_PARAMS, &format!("Unknown tool: {tool}. The tools are: {known}.")));
        };
        let args = params.get("arguments").cloned().unwrap_or(json!({}));
        let mut args = if args.is_null() { json!({}) } else { args };
        coerce_args(&def, &mut args);
        // (a wrong call: told at once what the tool needs, as a tool's
        // error the AI reads and acts on)
        if let Err(why) = check_args(&def, &args) {
            let mut r = json!({"content": [{"type": "text", "text": why}], "isError": true});
            if modern {
                r["resultType"] = json!("complete");
            }
            return send_json(&a, 200, &[], &result_message(&id, r));
        }
        (Kind::Tool, tool, args)
    };
    let key = a.id;
    let waiting = with_server(name, |s| {
        s.calls.insert(key, Call { conn: a.conn, request: a.request, id, kind, name: target, args, client, modern, events, progress, stream: None });
        s.queue.push_back(key);
        s.calls.len()
    })
    .unwrap_or(0);
    rp_comp_set(name, "pendingcount", v_int(waiting as i64));
}

/// The next call to the program, once the last one's handler has run.
fn next_call(name: &str) {
    let next = with_server(name, |s| {
        if s.busy.get() {
            return None;
        }
        while let Some(k) = s.queue.pop_front() {
            if let Some(c) = s.calls.get(&k) {
                s.busy.set(true);
                return Some((k, c.kind, c.name.clone(), c.args.to_string(), c.client.clone(), s.busy.clone(), !s.queue.is_empty()));
            }
        }
        None
    })
    .flatten();
    let Some((key, kind, target, args, client, busy, more)) = next else { return };
    rp_comp_set(name, "call", v_int(key));
    rp_comp_set(name, "tool", v_str(if kind == Kind::Tool { &target } else { "" }));
    rp_comp_set(name, "clientname", v_str(&client));
    rp_comp_set(name, "arguments", v_str(&args));
    let (event, fire_args) = match kind {
        Kind::Tool => ("ontoolcall", vec![v_str(&target), v_str(&args), v_int(key)]),
        Kind::Resource => ("onreadresource", vec![v_str(&target), v_int(key)]),
    };
    if !crate::object::rp_has_handler(name, event) {
        let why = if kind == Kind::Tool { "This program doesn't answer tool calls yet (it has no OnToolCall handler)." } else { "This program doesn't read resources (it has no OnReadResource handler)." };
        reply_text(name, key, why, true);
        busy.set(false);
        if more {
            crate::object::rp_wake();
        }
        return;
    }
    let owner = name.to_string();
    crate::object::rp_fire_event_then(name, event, &fire_args, move |_| {
        busy.set(false);
        // (not answered in the handler: it waits — as a stream when the
        // client takes one, so it can hear how the call goes)
        with_server(&owner, |s| {
            if let Some(c) = s.calls.get_mut(&key) {
                if c.stream.is_none() && c.events {
                    c.stream = httpserver::write_stream_head(&c.conn, &c.request, "");
                }
            }
        });
        crate::object::rp_wake();
    });
}

// ------------------------------------------------------ arguments' check --

fn type_ok(want: &str, v: &Json) -> bool {
    match want {
        "string" => v.is_string(),
        "integer" => v.as_i64().is_some() || v.as_u64().is_some() || v.as_f64().is_some_and(|f| f.fract() == 0.0),
        "number" => v.is_number(),
        "boolean" => v.is_boolean(),
        "array" => v.is_array(),
        "object" => v.is_object(),
        "null" => v.is_null(),
        _ => true,
    }
}

fn type_words(want: &str) -> &str {
    match want {
        "string" => "a string",
        "integer" => "a whole number",
        "number" => "a number",
        "boolean" => "true or false",
        "array" => "a list",
        "object" => "an object",
        other => other,
    }
}

/// What a property is, for the sentence that says a tool needs it.
fn property_words(key: &str, p: &Json) -> String {
    let t = p.get("type").and_then(Json::as_str).map(type_words).unwrap_or("a value");
    let what = p.get("description").and_then(Json::as_str).map(|d| format!(": {d}")).unwrap_or_default();
    let choices = p.get("enum").and_then(Json::as_array).map(|e| format!(", one of {}", e.iter().map(Json::to_string).collect::<Vec<_>>().join(", "))).unwrap_or_default();
    format!("`{key}` ({t}{choices}{what})")
}

/// What a small model sends for a list or an object is often that value as a JSON *string*
/// (`"edits": "[{\"old_text\": …}]"`), a whole number as `"20"`, a flag as `"true"`: the AI
/// app's own tool-call layer didn't parse it. The tool's schema says what it wants, so a
/// string that is exactly that value is that value (found by the outside-AI-app eval,
/// tests/ai-eval/README.md: a small model repeated the refused call to the step limit). A
/// string that isn't is left for `check_args` to refuse in words.
fn coerce_args(def: &ToolDef, args: &mut Json) {
    let empty = Map::new();
    let props = def.schema.get("properties").and_then(Json::as_object).unwrap_or(&empty);
    let Some(obj) = args.as_object_mut() else { return };
    for (key, v) in obj.iter_mut() {
        let Some(want) = props.get(key).and_then(|p| p.get("type")).and_then(Json::as_str) else { continue };
        let Some(text) = v.as_str().map(str::trim) else { continue };
        let parsed = match want {
            "array" | "object" | "integer" | "number" | "boolean" => serde_json::from_str::<Json>(text).ok(),
            _ => None,
        };
        if let Some(p) = parsed.filter(|p| type_ok(want, p)) {
            *v = p;
        }
    }
}

/// A tool's arguments checked against its schema (the top level, and a
/// list's items' type): Err with a sentence an AI can act on.
fn check_args(def: &ToolDef, args: &Json) -> Result<(), String> {
    let Some(obj) = args.as_object() else {
        return Err(format!("The arguments of {} must be a JSON object.", def.name));
    };
    let empty = Map::new();
    let props = def.schema.get("properties").and_then(Json::as_object).unwrap_or(&empty);
    let mut problems = Vec::new();
    for key in def.schema.get("required").and_then(Json::as_array).into_iter().flatten().filter_map(Json::as_str) {
        if obj.get(key).is_none_or(Json::is_null) {
            problems.push(format!("it needs {}", property_words(key, props.get(key).unwrap_or(&Json::Null))));
        }
    }
    for (key, v) in obj {
        let Some(p) = props.get(key) else {
            if def.schema.get("additionalProperties") == Some(&json!(false)) {
                let known: Vec<&str> = props.keys().map(String::as_str).collect();
                problems.push(if known.is_empty() { format!("it takes no arguments (not `{key}`)") } else { format!("it has no argument `{key}` (its arguments: {})", known.join(", ")) });
            }
            continue;
        };
        if v.is_null() {
            continue;
        }
        if let Some(t) = p.get("type").and_then(Json::as_str) {
            if !type_ok(t, v) {
                problems.push(format!("`{key}` must be {}, not {v}", type_words(t)));
                continue;
            }
        }
        if let Some(choices) = p.get("enum").and_then(Json::as_array) {
            if !choices.contains(v) {
                problems.push(format!("`{key}` must be one of {}, not {v}", choices.iter().map(Json::to_string).collect::<Vec<_>>().join(", ")));
            }
        }
        if let (Some(items), Some(list)) = (p.pointer("/items/type").and_then(Json::as_str), v.as_array()) {
            if let Some((i, bad)) = list.iter().enumerate().find(|(_, x)| !type_ok(items, x)) {
                problems.push(format!("`{key}`[{i}] must be {}, not {bad}", type_words(items)));
            }
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(format!("Call {} again: {}.", def.name, problems.join("; ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def(schema: Json) -> ToolDef {
        ToolDef { name: "read_file".into(), description: String::new(), schema, annotations: None, title: String::new() }
    }

    #[test]
    fn arguments_are_checked_with_words_an_ai_can_act_on() {
        let d = def(json!({"type": "object", "properties": {"path": {"type": "string", "description": "the file's path"}, "lines": {"type": "integer"}, "mode": {"type": "string", "enum": ["a", "b"]}, "edits": {"type": "array", "items": {"type": "object"}}}, "required": ["path"], "additionalProperties": false}));
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
        assert!(check_args(&def(json!({"type": "object"})), &json!({"anything": 1})).is_ok(), "no schema to keep to: anything");
    }

    #[test]
    fn a_list_sent_as_json_text_is_the_list() {
        // (a small model's tool call layer: "edits" as a string of JSON, "20" for a number)
        let d = def(json!({"type": "object", "properties": {"path": {"type": "string"}, "edits": {"type": "array", "items": {"type": "object"}}, "wait": {"type": "integer"}, "debug": {"type": "boolean"}, "meta": {"type": "object"}}, "required": ["path"], "additionalProperties": false}));
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
    fn messages_go_as_ascii_meaning_the_same() {
        let m = json!({"text": "héllo ✓ 𝄞"});
        let a = ascii(&m);
        assert!(a.is_ascii());
        assert_eq!(serde_json::from_str::<Json>(&a).unwrap(), m);
    }

    #[test]
    fn base64_names_are_decoded() {
        assert_eq!(header_value("get_weather").as_deref(), Some("get_weather"));
        assert_eq!(header_value("=?base64?SGVsbG8sIOS4lueVjA==?=").as_deref(), Some("Hello, 世界"));
        assert_eq!(header_value("=?base64?!!?="), None);
    }

    #[test]
    fn the_versions_answered() {
        assert_eq!(supported()[0], "2026-07-28");
        assert!(supported().contains(&"2025-03-26"));
    }

    #[test]
    fn images_are_known_by_their_bytes() {
        assert_eq!(image_mime(b"\x89PNG\r\n\x1a\n...."), Some("image/png"));
        assert_eq!(image_mime(&[0xFF, 0xD8, 0xFF]), Some("image/jpeg"));
        assert_eq!(image_mime(b"BM...."), None);
    }
}
