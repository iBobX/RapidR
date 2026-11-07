//! RMcpServer on the desktop: its **http** transport. A program's tools
//! served to AI apps over MCP's Streamable HTTP transport, on 127.0.0.1
//! with a token — RapidR Studio's own server is one (ide/mcp.inc), and any
//! program can have one.
//!
//! What the tools are and how their calls go (the definitions, the
//! arguments' check, the calls handed to the program one at a time, the
//! clients, the answers) is the core's, [`rapidr_value::agent_tools`], which the
//! web's browser transport shares (rapidr-runtime-web's `agent_tools_web.rs`).
//! This file is only the wire: MCP's JSON-RPC messages, its sessions, the
//! HTTP connections the answers go back on.
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

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::OnceLock;
use std::time::Instant;

use rapidr_value::agent_tools::{self as mcp, Done, Dropped, Kind, Outcome, Queued, Refused};
use serde_json::{json, Value as Json};

use crate::httpserver::{self, Arrived, ListenOptions, Out};
use crate::netframe::Request;
use crate::object::{rp_comp_get, rp_comp_set};
use crate::value::{v_int, v_null, v_str, Value};

/// The revision served statelessly (newest first).
pub const MODERN: &[&str] = &["2026-07-28"];
/// The initialize-based revisions served with a session (newest first).
/// 2024-11-05 too: some AI apps still ask only for it, and answered with a
/// newer revision they misread the reply (Goose, Robert 2026-11-03).
pub const LEGACY: &[&str] = &["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

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

/// The runtime as the core sees it.
#[derive(Clone, Copy)]
struct Desk;

impl mcp::Host for Desk {
    fn get(self, name: &str, prop: &str) -> Value {
        rp_comp_get(name, prop)
    }
    fn set(self, name: &str, prop: &str, v: Value) {
        rp_comp_set(name, prop, v);
    }
    fn has_handler(self, name: &str, event: &str) -> bool {
        crate::object::rp_has_handler(name, event)
    }
    fn fire_then(self, name: &str, event: &str, args: &[Value], then: Box<dyn FnOnce()>) {
        crate::object::rp_fire_event_then(name, event, args, move |_| then());
    }
    fn wake(self) {
        crate::object::rp_wake();
    }
    fn now(self) -> f64 {
        static START: OnceLock<Instant> = OnceLock::new();
        START.get_or_init(Instant::now).elapsed().as_secs_f64()
    }
    fn read_file(self, path: &str) -> Result<Vec<u8>, String> {
        std::fs::read(path).map_err(|e| e.to_string())
    }
    // (a key a program's tool read never reaches an AI app)
    fn scrub(self, text: &str) -> String {
        rapidr_ai::keys::redact(text)
    }
}

/// An earlier revision's session (initialize's Mcp-Session-Id).
struct Session {
    client: String,
    protocol: String,
}

/// The http transport's own part of a server: its listener, the
/// connections' messages, its address's path, the earlier revisions'
/// sessions.
struct Http {
    running: httpserver::Accepting,
    events: mpsc::Receiver<Arrived>,
    path: String,
    sessions: HashMap<String, Session>,
}

thread_local! {
    static SERVERS: RefCell<HashMap<String, Http>> = RefCell::new(HashMap::new());
}

fn with_http<R>(name: &str, f: impl FnOnce(&mut Http) -> R) -> Option<R> {
    SERVERS.with(|s| s.borrow_mut().get_mut(name).map(f))
}

fn prop_text(name: &str, prop: &str) -> String {
    rp_comp_get(name, prop).to_string_val()
}

/// RMcpServer `name`'s method: the transport's own (Start, Stop, NewToken,
/// Disconnect's sessions), else the core's.
pub fn method(name: &str, method: &str, args: &[Value]) -> Value {
    let name = name.to_lowercase();
    match method {
        "start" => start(&name),
        "stop" => {
            stop(&name);
            v_null()
        }
        "newtoken" => v_str(&new_token(&name)),
        "disconnect" => {
            let who = args.first().map(Value::to_string_val).unwrap_or_default();
            // (its sessions end with it)
            let out = mcp::disconnect(Desk, &name, &who, |out| {
                with_http(&name, |h| h.sessions.retain(|_, x| !out.iter().any(|n| n.eq_ignore_ascii_case(&x.client))));
            });
            v_int(out.is_some() as i64)
        }
        _ => mcp::method(Desk, &name, method, args).unwrap_or_else(|| {
            eprintln!("[WARN] RMcpServer.{method}() not implemented");
            v_null()
        }),
    }
}

/// RMcpServer's properties the transport answers: Available (always, on
/// the desktop), Transport ("http").
pub fn get(prop: &str) -> Option<Value> {
    match prop {
        "available" => Some(v_int(1)),
        "transport" => Some(v_str("http")),
        _ => None,
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
    let url = format!("{}{}", l.url.trim_end_matches('/'), path);
    SERVERS.with(|s| s.borrow_mut().insert(name.to_string(), Http { running: l.running, events: l.events, path, sessions: HashMap::new() }));
    mcp::open(Desk, name, mcp::Info::of(Desk, name), idle);
    rp_comp_set(name, "port", v_int(i64::from(l.port)));
    rp_comp_set(name, "url", v_str(&url));
    rp_comp_set(name, "error", v_str(""));
    rp_comp_set(name, "listening", v_int(1));
    v_int(1)
}

fn stop(name: &str) {
    if let Some(mut h) = SERVERS.with(|s| s.borrow_mut().remove(name)) {
        h.running.stop();
    }
    // (each call still waiting: told, then closed)
    mcp::close(Desk, name);
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
        let Some(a) = with_http(name, |s| s.events.try_recv().ok()).flatten() else { break };
        arrived(name, a);
    }
    mcp::poll(Desk, name);
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

/// Where one call's answer goes: its connection, as JSON or as the last
/// event of its stream.
struct Reply {
    conn: mpsc::Sender<Out>,
    request: Request,
    /// The JSON-RPC id.
    id: Json,
    modern: bool,
    /// The client takes an event stream as the answer.
    events: bool,
    progress: Option<Json>,
    /// Once its answer is a stream: whether its events go as chunks.
    stream: Option<bool>,
}

impl Reply {
    fn send(&self, message: &Json) -> bool {
        let text = ascii(message);
        match self.stream {
            Some(chunked) => httpserver::write_event(&self.conn, chunked, &text, "message") && httpserver::write_stream_end(&self.conn, &self.request, chunked),
            None => httpserver::write_response(&self.conn, &self.request, 200, vec![("Content-Type".into(), "application/json".into())], text.as_bytes()),
        }
    }
}

impl mcp::Sink for Reply {
    fn finish(self: Box<Self>, done: Done) -> bool {
        let message = match done {
            Done::Answer { kind: Kind::Resource, outcome: Outcome::Text { text, error: true }, .. } => error_message(&self.id, INTERNAL_ERROR, &text),
            Done::Answer { kind, target, mime, outcome } => {
                let mut result = match (kind, outcome) {
                    (Kind::Resource, Outcome::Text { text, .. }) => json!({"contents": [{"uri": target, "mimeType": mime, "text": text}]}),
                    (_, other) => other.mcp_tool_result(),
                };
                if self.modern {
                    if let Some(o) = result.as_object_mut() {
                        o.entry("resultType").or_insert(json!("complete"));
                    }
                }
                result_message(&self.id, result)
            }
            Done::Dropped(Dropped::Stopped) => error_message(&self.id, INTERNAL_ERROR, "The server stopped before this call was answered."),
            Done::Dropped(Dropped::Disconnected) => error_message(&self.id, SESSION_REFUSED, "Disconnected by the person using the app."),
            Done::Dropped(Dropped::Cancelled) => error_message(&self.id, INTERNAL_ERROR, "The call was cancelled."),
        };
        self.send(&message)
    }

    /// As a progress notification (when the client asked for them) or a
    /// log message, on the call's event stream. False when the client
    /// takes no stream or left.
    fn progress(&mut self, server: &str, message: &str, done: Option<f64>, total: Option<f64>) -> bool {
        if self.stream.is_none() {
            if !self.events {
                return false;
            }
            match httpserver::write_stream_head(&self.conn, &self.request, "") {
                Some(chunked) => self.stream = Some(chunked),
                None => return false,
            }
        }
        let note = match &self.progress {
            Some(token) => {
                let mut p = json!({"progressToken": token, "progress": done.unwrap_or(0.0), "message": message});
                if let Some(t) = total.filter(|t| *t > 0.0) {
                    p["total"] = json!(t);
                }
                json!({"jsonrpc": "2.0", "method": "notifications/progress", "params": p})
            }
            None => json!({"jsonrpc": "2.0", "method": "notifications/message", "params": {"level": "info", "logger": server, "data": message}}),
        };
        httpserver::write_event(&self.conn, self.stream.unwrap_or(true), &ascii(&note), "message")
    }

    /// Not answered in its handler: it waits — as a stream when the client
    /// takes one, so it can hear how the call goes.
    fn waiting(&mut self) {
        if self.stream.is_none() && self.events {
            self.stream = httpserver::write_stream_head(&self.conn, &self.request, "");
        }
    }
}

// -------------------------------------------------------------- clients --

fn client_title(info: Option<&Json>) -> (String, String) {
    let field = |k: &str| info.and_then(|i| i.get(k)).and_then(Json::as_str).map(str::trim).filter(|t| !t.is_empty()).map(|t| t.chars().take(80).collect::<String>());
    let name = field("title").or_else(|| field("name")).unwrap_or_else(|| "an AI app".into());
    (name, field("version").unwrap_or_default())
}

// ------------------------------------------------------------- requests --

/// A request that arrived, answered — or, a call, kept for the program.
fn arrived(name: &str, a: Arrived) {
    let path = with_http(name, |s| s.path.clone()).unwrap_or_default();
    if a.request.path() != path {
        return send_plain(&a, 404, &[], &format!("AI apps connect at {path}.\n"));
    }
    match a.request.method.to_ascii_uppercase().as_str() {
        "POST" => post(name, a),
        "DELETE" => {
            // (an earlier revision's session ended)
            let sid = a.request.header("mcp-session-id").unwrap_or("").trim().to_string();
            let gone = with_http(name, |s| s.sessions.remove(&sid)).flatten();
            match gone {
                Some(x) => {
                    let still = with_http(name, |s| s.sessions.values().any(|o| o.client == x.client)).unwrap_or(false);
                    send_plain(&a, 200, &[], "");
                    if !still {
                        mcp::client_gone(Desk, name, &x.client);
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
    // (what the server is, asked whatever revision the header names — an
    // app asks it first, with an earlier revision's header, and quit when
    // refused (Goose, Robert 2026-11-03): the versions it can then pick)
    // (a client the person disconnected is refused here too, as on every
    // other path; one let in is heard: the app shows it)
    if method == "server/discover" && !modern {
        if let Some(id) = &id {
            let meta = body.pointer("/params/_meta").and_then(|m| m.get(META_CLIENT));
            let (client, version_of) = client_title(meta.or_else(|| body.pointer("/params/clientInfo")));
            if mcp::shut_out(name, &client) {
                return send_json(&a, 403, &[], &error_message(id, SESSION_REFUSED, "Disconnected by the person using the app: they can let AI apps in again by turning the server off and on."));
            }
            mcp::heard(Desk, name, &client, &version_of, false);
            return discover(name, &a, id);
        }
    }
    if !modern {
        let idj = id.unwrap_or(Json::Null);
        return send_json(&a, 400, &[], &error_message(&idj, SESSION_REFUSED, "Start a session with initialize (then send its Mcp-Session-Id), or speak revision 2026-07-28: an MCP-Protocol-Version header and _meta on every request."));
    }
    modern_request(name, a, &body, header_version, meta_version);
}

/// server/discover's answer: the revisions served, the capabilities, who
/// the server is, and how long it may be kept (`ttlMs`, `cacheScope`:
/// required — without them Goose's library read the answer as another
/// kind and quit, Robert 2026-11-04). Not kept (0), and only for this
/// client: it is behind the token.
fn discover(name: &str, a: &Arrived, id: &Json) {
    let (info, caps) = describe(name);
    let mut r = json!({"resultType": "complete", "supportedVersions": supported(), "capabilities": caps, "ttlMs": 0, "cacheScope": "private", "_meta": {META_SERVER: server_info(&info)}});
    if !info.instructions.trim().is_empty() {
        r["instructions"] = json!(info.instructions);
    }
    send_json(a, 200, &[], &result_message(id, r));
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
    if mcp::shut_out(name, &client) {
        return send_json(&a, 403, &[], &error_message(&idj, SESSION_REFUSED, "Disconnected by the person using the app: they can let AI apps in again by turning the server off and on."));
    }
    // (a notification: nothing to say back)
    let Some(id) = id else {
        return send_plain(&a, 202, &[], "");
    };
    mcp::heard(Desk, name, &client, &version_of, method == "tools/call");
    match method.as_str() {
        "server/discover" => discover(name, &a, &id),
        "ping" => send_json(&a, 200, &[], &result_message(&id, json!({"resultType": "complete"}))),
        "tools/list" => send_json(&a, 200, &[], &result_message(&id, with_type(tools_list(name), true))),
        "resources/list" => send_json(&a, 200, &[], &result_message(&id, with_type(resources_list(name), true))),
        "resources/templates/list" => send_json(&a, 200, &[], &result_message(&id, json!({"resultType": "complete", "resourceTemplates": []}))),
        "prompts/list" => send_json(&a, 200, &[], &result_message(&id, with_type(prompts_list(name), true))),
        "prompts/get" => match prompts_get(name, body) {
            Ok(r) => send_json(&a, 200, &[], &result_message(&id, with_type(r, true))),
            Err(why) => send_json(&a, 200, &[], &error_message(&id, INVALID_PARAMS, &why)),
        },
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

fn server_info(info: &mcp::Info) -> Json {
    let mut i = json!({"name": info.name, "version": info.version});
    if !info.title.trim().is_empty() {
        i["title"] = json!(info.title);
    }
    i
}

/// The server's info and capabilities: tools always; resources when it
/// has some; logging (its messages on a call's stream).
fn describe(name: &str) -> (mcp::Info, Json) {
    let info = mcp::info(name);
    let mut caps = json!({"tools": {}, "logging": {}});
    if !mcp::resources(name).is_empty() {
        caps["resources"] = json!({});
    }
    if !mcp::prompts(name).is_empty() {
        caps["prompts"] = json!({});
    }
    (info, caps)
}

fn tools_list(name: &str) -> Json {
    let tools: Vec<Json> = mcp::tools(name)
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
        .collect();
    json!({"tools": tools})
}

fn resources_list(name: &str) -> Json {
    let list: Vec<Json> = mcp::resources(name).iter().map(|r| json!({"uri": r.uri, "name": r.name, "description": r.description, "mimeType": r.mime})).collect();
    json!({"resources": list})
}

fn prompts_list(name: &str) -> Json {
    let list: Vec<Json> = mcp::prompts(name)
        .iter()
        .map(|p| {
            let mut j = json!({"name": p.name, "description": p.description, "arguments": []});
            if !p.title.is_empty() {
                j["title"] = json!(p.title);
            }
            j
        })
        .collect();
    json!({"prompts": list})
}

/// prompts/get: the prompt's text as one user message, or why not.
fn prompts_get(name: &str, body: &Json) -> Result<Json, String> {
    let wanted = body.pointer("/params/name").and_then(Json::as_str).unwrap_or("");
    let prompts = mcp::prompts(name);
    match prompts.iter().find(|p| p.name == wanted) {
        Some(p) => Ok(json!({"description": p.description, "messages": [{"role": "user", "content": {"type": "text", "text": p.text}}]})),
        None => Err(format!("Unknown prompt: {wanted}. The prompts are: {}.", prompts.iter().map(|p| p.name.clone()).collect::<Vec<_>>().join(", "))),
    }
}

/// An earlier revision's initialize: a session begins.
fn initialize(name: &str, a: &Arrived, body: &Json) {
    let id = body.get("id").cloned().unwrap_or(Json::Null);
    let asked = body.pointer("/params/protocolVersion").and_then(Json::as_str).unwrap_or("");
    let version = if LEGACY.contains(&asked) { asked.to_string() } else { LEGACY[0].to_string() };
    let (client, client_version) = client_title(body.pointer("/params/clientInfo"));
    if mcp::shut_out(name, &client) {
        return send_json(a, 403, &[], &error_message(&id, SESSION_REFUSED, "Disconnected by the person using the app: they can let AI apps in again by turning the server off and on."));
    }
    let Some(sid) = httpserver::random_token().map(|t| t[..32].to_string()) else {
        return send_json(a, 500, &[], &error_message(&id, INTERNAL_ERROR, "The system's secure random source gave nothing: no session could start."));
    };
    with_http(name, |s| s.sessions.insert(sid.clone(), Session { client: client.clone(), protocol: version.clone() }));
    let (info, caps) = describe(name);
    let mut r = json!({"protocolVersion": version, "capabilities": caps, "serverInfo": server_info(&info)});
    if !info.instructions.trim().is_empty() {
        r["instructions"] = json!(info.instructions);
    }
    send_json(a, 200, &[("Mcp-Session-Id", &sid)], &result_message(&id, r));
    mcp::heard(Desk, name, &client, &client_version, false);
}

/// A request in an earlier revision's session.
fn legacy(name: &str, a: Arrived, body: &Json, sid: &str) {
    let id = body.get("id").cloned();
    let idj = id.clone().unwrap_or(Json::Null);
    let method = body.get("method").and_then(Json::as_str).unwrap_or("").to_string();
    let Some((client, _protocol)) = with_http(name, |s| s.sessions.get(sid).map(|x| (x.client.clone(), x.protocol.clone()))).flatten() else {
        return send_json(&a, 404, &[], &error_message(&idj, SESSION_ENDED, "This session ended (the app was closed, or its server turned off and on): start a new one with initialize."));
    };
    if mcp::shut_out(name, &client) {
        return send_json(&a, 403, &[], &error_message(&idj, SESSION_REFUSED, "Disconnected by the person using the app."));
    }
    // (a notification, or the client's answer to a request: nothing to say)
    let Some(id) = id.filter(|_| !method.is_empty()) else {
        return send_plain(&a, 202, &[], "");
    };
    mcp::heard(Desk, name, &client, "", method == "tools/call");
    match method.as_str() {
        "ping" | "logging/setLevel" => send_json(&a, 200, &[], &result_message(&id, json!({}))),
        "tools/list" => send_json(&a, 200, &[], &result_message(&id, tools_list(name))),
        "resources/list" => send_json(&a, 200, &[], &result_message(&id, resources_list(name))),
        "resources/templates/list" => send_json(&a, 200, &[], &result_message(&id, json!({"resourceTemplates": []}))),
        "prompts/list" => send_json(&a, 200, &[], &result_message(&id, prompts_list(name))),
        "prompts/get" => match prompts_get(name, body) {
            Ok(r) => send_json(&a, 200, &[], &result_message(&id, r)),
            Err(why) => send_json(&a, 200, &[], &error_message(&id, INVALID_PARAMS, &why)),
        },
        "tools/call" | "resources/read" => call(name, a, body, id, client, false),
        _ => send_json(&a, 200, &[], &error_message(&id, METHOD_NOT_FOUND, &format!("Method not found: {method}"))),
    }
}

/// A tools/call or resources/read: checked (the core's), then kept for the
/// program with the connection its answer goes back on.
fn call(name: &str, a: Arrived, body: &Json, id: Json, client: String, modern: bool) {
    let params = body.get("params").cloned().unwrap_or(json!({}));
    let read = body.get("method").and_then(Json::as_str) == Some("resources/read");
    let events = a.request.header("accept").is_some_and(|h| h.to_ascii_lowercase().contains("text/event-stream"));
    let progress = params.pointer("/_meta/progressToken").cloned().filter(|t| t.is_string() || t.is_number());
    let (kind, asked, args) = if read {
        (Kind::Resource, params.get("uri").and_then(Json::as_str).unwrap_or("").to_string(), json!({}))
    } else {
        (Kind::Tool, params.get("name").and_then(Json::as_str).unwrap_or("").to_string(), params.get("arguments").cloned().unwrap_or(json!({})))
    };
    let (target, args) = match mcp::check_call(name, kind, &asked, args) {
        Ok(checked) => checked,
        Err(Refused::UnknownResource(uri)) => return send_json(&a, 200, &[], &error_data(&id, RESOURCE_NOT_FOUND, "Resource not found", json!({"uri": uri}))),
        Err(Refused::UnknownTool(why)) => return send_json(&a, 200, &[], &error_message(&id, INVALID_PARAMS, &why)),
        // (a wrong call: told at once what the tool needs, as a tool's
        // error the AI reads and acts on)
        Err(Refused::BadArguments(why)) => {
            let mut r = json!({"content": [{"type": "text", "text": why}], "isError": true});
            if modern {
                r["resultType"] = json!("complete");
            }
            return send_json(&a, 200, &[], &result_message(&id, r));
        }
    };
    let reply = Reply { conn: a.conn, request: a.request, id, modern, events, progress, stream: None };
    mcp::enqueue(Desk, name, Queued { kind, target, args, client }, Box::new(reply));
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(supported().contains(&"2024-11-05"));
    }

    /// A server named `name` for the tests: nothing listens; its requests
    /// are handed to `post` as they would arrive.
    fn test_server(name: &str) {
        let (_tx, rx) = mpsc::channel();
        SERVERS.with(|s| s.borrow_mut().insert(name.to_string(), Http { running: httpserver::Accepting::none(), events: rx, path: "/mcp".into(), sessions: HashMap::new() }));
        mcp::open(Desk, name, mcp::Info { name: "rapidr-test".into(), version: "1.0".into(), ..mcp::Info::default() }, 900.0);
    }

    /// A POST of `body` with `headers` to server `name`: its status, its
    /// JSON answer, its head.
    fn posted(name: &str, headers: &[(&str, &str)], body: &Json) -> (u16, Json, String) {
        let (tx, rx) = mpsc::channel();
        let mut all = vec![("Content-Type".to_string(), "application/json".to_string())];
        all.extend(headers.iter().map(|(n, v)| (n.to_string(), v.to_string())));
        let request = Request { method: "POST".into(), target: "/mcp".into(), version: "HTTP/1.1".into(), headers: all, body: body.to_string().into_bytes() };
        post(name, Arrived { id: 1, request, peer: "127.0.0.1".into(), conn: tx });
        read_out(&rx)
    }

    fn read_out(rx: &mpsc::Receiver<Out>) -> (u16, Json, String) {
        let mut bytes = Vec::new();
        while let Ok(out) = rx.try_recv() {
            if let Out::Bytes(b) = out {
                bytes.extend(b);
            }
        }
        let text = String::from_utf8(bytes).unwrap();
        let (head, answer) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
        let status = head.split(' ').nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        (status, serde_json::from_str(answer).unwrap_or(Json::Null), head.to_string())
    }

    fn clients_of(name: &str) -> Vec<String> {
        mcp::clients(name).iter().map(|c| c.name.clone()).collect()
    }

    /// server/discover asked with an earlier revision's header, or with
    /// none: the whole answer (ttlMs and cacheScope too), the client heard;
    /// a client the person disconnected is refused there as everywhere.
    #[test]
    fn discover_on_the_earlier_revisions_path() {
        let name = "mcp-test-discover";
        test_server(name);
        let ask = |client: &str| json!({"jsonrpc": "2.0", "id": 7, "method": "server/discover", "params": {"_meta": {META_CLIENT: {"name": client, "version": "1.2"}}}});
        for headers in [&[("MCP-Protocol-Version", "2025-06-18")][..], &[("MCP-Protocol-Version", "2025-11-25")][..], &[][..]] {
            let (status, answer, _) = posted(name, headers, &ask("goose"));
            assert_eq!(status, 200, "{headers:?}: {answer}");
            let r = &answer["result"];
            assert_eq!(answer["id"], json!(7));
            assert_eq!(r["supportedVersions"][0], json!("2026-07-28"), "{answer}");
            assert_eq!(r["ttlMs"], json!(0), "{answer}");
            assert_eq!(r["cacheScope"], json!("private"), "{answer}");
            assert_eq!(r["_meta"][META_SERVER]["name"], json!("rapidr-test"), "{answer}");
        }
        assert_eq!(clients_of(name), vec!["goose".to_string()], "heard: the app shows it");
        // (no _meta: the earlier revisions' clientInfo names it, or none)
        let (status, _, _) = posted(name, &[], &json!({"jsonrpc": "2.0", "id": 8, "method": "server/discover"}));
        assert_eq!(status, 200);
        assert!(clients_of(name).contains(&"an AI app".to_string()));
        // (disconnected by the person: refused, not heard again)
        assert_eq!(method(name, "disconnect", &[v_str("goose")]).to_i64(), 1);
        let (status, answer, _) = posted(name, &[("MCP-Protocol-Version", "2025-06-18")], &ask("Goose"));
        assert_eq!(status, 403, "{answer}");
        assert_eq!(answer["error"]["code"], json!(SESSION_REFUSED));
        assert!(answer["error"]["message"].as_str().unwrap_or("").contains("Disconnected"), "{answer}");
        assert!(!clients_of(name).iter().any(|c| c.eq_ignore_ascii_case("goose")), "a refused client isn't heard");
        stop(name);
    }

    /// An earlier revision's initialize at 2024-11-05: answered with that
    /// revision and a session; the session's requests work; Disconnect
    /// ends its sessions (the next request: no such session).
    #[test]
    fn a_session_at_2024_11_05() {
        let name = "mcp-test-2024";
        test_server(name);
        let (status, answer, head) = posted(name, &[], &json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "old-app", "version": "0.1"}}}));
        assert_eq!(status, 200, "{answer}");
        assert_eq!(answer["result"]["protocolVersion"], json!("2024-11-05"), "{answer}");
        assert_eq!(answer["result"]["serverInfo"]["name"], json!("rapidr-test"));
        let sid = head.lines().find_map(|l| l.strip_prefix("Mcp-Session-Id: ")).map(str::trim).unwrap_or("").to_string();
        assert_eq!(sid.len(), 32, "{head}");
        let (status, answer, _) = posted(name, &[("Mcp-Session-Id", &sid), ("MCP-Protocol-Version", "2024-11-05")], &json!({"jsonrpc": "2.0", "id": 2, "method": "ping"}));
        assert_eq!(status, 200, "{answer}");
        assert_eq!(answer["result"], json!({}));
        assert_eq!(clients_of(name), vec!["old-app".to_string()]);
        assert_eq!(method(name, "disconnect", &[v_str("old-app")]).to_i64(), 1);
        let (status, answer, _) = posted(name, &[("Mcp-Session-Id", &sid)], &json!({"jsonrpc": "2.0", "id": 3, "method": "ping"}));
        assert_eq!(status, 404, "{answer}");
        assert_eq!(answer["error"]["code"], json!(SESSION_ENDED));
        stop(name);
    }

    /// An old name (AddAlias) is its tool's call, never listed; prompts
    /// (AddPrompt) are listed and given, the capability said — on the
    /// current revision as on the earlier ones. A call kept for the
    /// program is answered on its connection by Reply (the core's), as
    /// MCP's result with resultType; Stop tells a waiting call.
    #[test]
    fn aliases_prompts_and_a_call_answered() {
        let name = "mcp-test-aliases";
        test_server(name);
        assert_eq!(method(name, "addtool", &[v_str("check_code"), v_str("Compiles."), v_str(""), v_str("read")]).to_i64(), 1);
        assert_eq!(method(name, "addalias", &[v_str("get_problems"), v_str("check_code")]).to_i64(), 1);
        assert_eq!(method(name, "addalias", &[v_str("check_code"), v_str("check_code")]).to_i64(), 0, "a tool isn't its own alias");
        assert_eq!(method(name, "addprompt", &[v_str("rapidr-way"), v_str("Work the RapidR way"), v_str("How to work on this project"), v_str("1. studio_status\n2. find_example")]).to_i64(), 1);
        assert_eq!(method(name, "addprompt", &[v_str("bad name"), v_str(""), v_str(""), v_str("x")]).to_i64(), 0);
        let modern = |method: &str, id: i64, params: Json, named: Option<&str>| {
            let mut p = params;
            p["_meta"] = json!({META_VERSION: MODERN[0], META_CLIENT: {"name": "alias-test", "version": "1"}});
            let mut headers = vec![("MCP-Protocol-Version", MODERN[0]), ("Mcp-Method", method)];
            if let Some(n) = named {
                headers.push(("Mcp-Name", n));
            }
            let (tx, rx) = mpsc::channel();
            let mut all = vec![("Content-Type".to_string(), "application/json".to_string())];
            all.extend(headers.iter().map(|(n, v)| (n.to_string(), v.to_string())));
            let request = Request { method: "POST".into(), target: "/mcp".into(), version: "HTTP/1.1".into(), headers: all, body: json!({"jsonrpc": "2.0", "id": id, "method": method, "params": p}).to_string().into_bytes() };
            post(name, Arrived { id: 1, request, peer: "127.0.0.1".into(), conn: tx });
            let out = read_out(&rx);
            (out, rx)
        };
        let ((status, answer, _), _) = modern("tools/list", 1, json!({}), None);
        assert_eq!(status, 200, "{answer}");
        let names: Vec<&str> = answer["result"]["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert_eq!(names, ["check_code"], "the alias isn't listed");
        assert_eq!(answer["result"]["tools"][0]["annotations"], json!({"readOnlyHint": true, "openWorldHint": false}));
        let ((_, answer, _), _) = modern("server/discover", 2, json!({}), None);
        assert_eq!(answer["result"]["capabilities"]["prompts"], json!({}), "{answer}");
        let ((_, answer, _), _) = modern("prompts/list", 3, json!({}), None);
        assert_eq!(answer["result"]["prompts"][0]["name"], json!("rapidr-way"), "{answer}");
        assert_eq!(answer["result"]["prompts"][0]["title"], json!("Work the RapidR way"));
        let ((_, answer, _), _) = modern("prompts/get", 4, json!({"name": "rapidr-way"}), Some("rapidr-way"));
        assert_eq!(answer["result"]["messages"][0]["content"]["text"], json!("1. studio_status\n2. find_example"), "{answer}");
        assert_eq!(answer["result"]["messages"][0]["role"], json!("user"));
        let ((_, answer, _), _) = modern("prompts/get", 5, json!({"name": "nope"}), Some("nope"));
        assert_eq!(answer["error"]["code"], json!(INVALID_PARAMS), "{answer}");
        let ((_, answer, _), _) = modern("tools/call", 9, json!({"name": "nope", "arguments": {}}), Some("nope"));
        assert_eq!(answer["error"]["code"], json!(INVALID_PARAMS), "{answer}");
        assert!(answer["error"]["message"].as_str().unwrap().starts_with("Unknown tool: nope. The tools are: check_code."), "{answer}");
        // (the old name's call: kept for the program as the tool's own)
        let ((status, answer, _), rx) = modern("tools/call", 6, json!({"name": "get_problems", "arguments": {}}), Some("get_problems"));
        assert_eq!(status, 0, "kept for the program, not answered yet: {answer}");
        let kept = mcp::waiting(name);
        assert_eq!(kept.iter().map(|c| c.2.as_str()).collect::<Vec<_>>(), ["check_code"]);
        assert_eq!(rp_comp_get(name, "pendingcount").to_i64(), 1);
        // (answered: MCP's result, resultType on the current revision)
        assert_eq!(method(name, "reply", &[v_int(kept[0].0), v_str("No problems.")]).to_i64(), 1);
        let (status, answer, _) = read_out(&rx);
        assert_eq!(status, 200, "{answer}");
        assert_eq!(answer, json!({"jsonrpc": "2.0", "id": 6, "result": {"content": [{"type": "text", "text": "No problems."}], "resultType": "complete"}}));
        assert_eq!(rp_comp_get(name, "pendingcount").to_i64(), 0);
        // (an earlier revision lists them too)
        let (_, answer, head) = posted(name, &[], &json!({"jsonrpc": "2.0", "id": 7, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "old", "version": "1"}}}));
        assert_eq!(answer["result"]["capabilities"]["prompts"], json!({}), "{answer}");
        let sid = head.lines().find_map(|l| l.strip_prefix("Mcp-Session-Id: ")).map(str::trim).unwrap_or("").to_string();
        let (_, answer, _) = posted(name, &[("Mcp-Session-Id", &sid)], &json!({"jsonrpc": "2.0", "id": 8, "method": "prompts/get", "params": {"name": "rapidr-way"}}));
        assert!(answer["result"]["messages"][0]["content"]["text"].as_str().unwrap_or("").contains("studio_status"), "{answer}");
        // (a call still waiting when the server stops: told so)
        let ((status, _, _), rx) = modern("tools/call", 10, json!({"name": "check_code", "arguments": {}}), Some("check_code"));
        assert_eq!(status, 0);
        stop(name);
        let (status, answer, _) = read_out(&rx);
        assert_eq!(status, 200, "{answer}");
        assert_eq!(answer["error"]["message"], json!("The server stopped before this call was answered."));
        mcp::forget(name);
    }
}
