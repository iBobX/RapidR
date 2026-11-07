//! The core of the components that offer a program's tools to AI agents:
//! RMcpServer (MCP's AI apps; RapidR Studio's own server is one) and
//! RAgentTools (a web app's tools for the AI agents of the browser). What
//! the tools are and how their calls go, the same whatever carries them.
//! Two transports use it and neither knows the other:
//!
//! - **http** (the desktop: rapidr-runtime-core's `mcpserver.rs`): MCP's
//!   JSON-RPC over loopback HTTP, with its token, Host and Origin checks;
//! - **browser** (the web: rapidr-runtime-web's `agent_tools_web.rs`): the
//!   page's tools offered to the AI agents of the browser, through the
//!   browser API of the WebMCP draft (`document.modelContext`).
//!
//! Here: the definitions (AddTool, AddAlias, AddPrompt, AddResource), the
//! arguments' check against a tool's schema (a plain sentence an AI can act
//! on), the calls that wait and the one at a time handed to the program
//! (OnToolCall / OnReadResource, then Reply / ReplyImage / ReplyJson), the
//! AI apps heard from (Clients, OnConnect, OnDisconnect, Disconnect), and
//! what leaves: every text with key-shaped words and the Token masked.
//!
//! A transport gives a [`Host`] (the runtime's components, events, clock)
//! and, for each call it hands in, a [`Sink`] (how that call's answer goes
//! back). It answers what it can itself (a list of tools, a refusal) and
//! hands the calls here: [`check_call`], then [`enqueue`], then [`poll`].
//! The program's methods that aren't the transport's own (Start, Stop,
//! NewToken) are [`method`].

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use serde_json::{json, Map, Value as Json};

use crate::{v_int, v_null, v_str, Value};

#[cfg(test)]
mod tests;

/// RAgentTools' type: the one place its name is written (a rename keeps
/// the old one as the registry's alias; the runtimes read it here).
pub const AGENT_TOOLS: &str = "RAGENTTOOLS";

/// The name the browser transport gives every caller: the browser says
/// nothing of who calls (a browser's assistant, an extension, a page's own script).
pub const BROWSER_AGENT: &str = "browser agent";

/// What a runtime gives the core.
pub trait Host: Copy + 'static {
    /// A property of the component, as the program reads it.
    fn get(self, name: &str, prop: &str) -> Value;
    /// Sets one of the component's read-back properties (Call, Tool,
    /// PendingCount, Error …).
    fn set(self, name: &str, prop: &str, v: Value);
    fn has_handler(self, name: &str, event: &str) -> bool;
    /// Fires `event`, then runs `then` once its handler has run (queued
    /// behind it in the interpreter).
    fn fire_then(self, name: &str, event: &str, args: &[Value], then: Box<dyn FnOnce()>);
    /// Asks for another [`poll`] soon (the window loop's turn on the
    /// desktop; a microtask on the web).
    fn wake(self);
    /// Seconds on a clock that only goes forward (any start).
    fn now(self) -> f64;
    /// A file's bytes (ReplyImage's picture).
    fn read_file(self, path: &str) -> Result<Vec<u8>, String> {
        crate::objects::read_file(path)
    }
    /// `text` with what must never reach an AI app masked (the runtimes:
    /// key-shaped words, rapidr_ai::keys::redact). As it is by default.
    fn scrub(self, text: &str) -> String {
        text.to_string()
    }
    /// Every component of the program: (name, type), in creation order
    /// (Expose's tools); none by default.
    fn components(self) -> Vec<(String, String)> {
        Vec::new()
    }
    /// A property set as the program sets it (what follows runs: an edit's
    /// OnChange …): Expose's app_set.
    fn program_set(self, name: &str, prop: &str, v: Value) {
        self.set(name, prop, v);
    }
    /// Asks the person using the program `text` (Allow / Don't): `None`
    /// where this host can't ask (ConfirmChanges then refuses the call).
    fn confirm(self, _text: &str) -> Option<bool> {
        None
    }
}

/// What a call asks for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Tool,
    Resource,
}

/// A tool the program offers.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolDef {
    pub name: String,
    pub title: String,
    pub description: String,
    /// The JSON Schema of its arguments (an object).
    pub schema: Json,
    /// MCP's annotations (readOnlyHint, destructiveHint, openWorldHint,
    /// title …), as the program gave them.
    pub annotations: Option<Json>,
    /// What it returns holds what others wrote (a file's text, a program's
    /// output, a web page): WebMCP's `untrustedContentHint`. MCP has no
    /// such annotation; the http transport doesn't send it.
    pub untrusted: bool,
    /// WebMCP's `consequentialHint` when the program said it; else it
    /// follows from readOnlyHint (a tool that changes something is).
    pub consequential: Option<bool>,
}

impl ToolDef {
    /// Whether the program said the tool only reads.
    pub fn read_only(&self) -> bool {
        self.annotations.as_ref().and_then(|a| a.get("readOnlyHint")).and_then(Json::as_bool).unwrap_or(false)
    }

    /// WebMCP's annotations: readOnlyHint, untrustedContentHint,
    /// consequentialHint (advice to the agent; the program's own policy
    /// still decides).
    pub fn browser_annotations(&self) -> Json {
        let read = self.read_only();
        json!({
            "readOnlyHint": read,
            "untrustedContentHint": self.untrusted,
            "consequentialHint": self.consequential.unwrap_or(!read),
        })
    }

    /// Everything a registration carries, as one text: a tool whose
    /// fingerprint changed is registered again.
    pub fn fingerprint(&self) -> String {
        format!("{}\u{1}{}\u{1}{}\u{1}{}\u{1}{}", self.title, self.description, self.schema, self.browser_annotations(), self.annotations.as_ref().map(Json::to_string).unwrap_or_default())
    }
}

/// A resource the program offers (resources/list; resources/read asks it).
#[derive(Clone, Debug, PartialEq)]
pub struct ResourceDef {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub mime: String,
}

/// A prompt the program offers: its name, title, what it is for, and the
/// text it gives (one user message).
#[derive(Clone, Debug, PartialEq)]
pub struct PromptDef {
    pub name: String,
    pub title: String,
    pub description: String,
    pub text: String,
}

#[derive(Default)]
struct Defs {
    tools: Vec<ToolDef>,
    resources: Vec<ResourceDef>,
    /// Old names still answered, never listed (AddAlias): (alias, tool).
    aliases: Vec<(String, String)>,
    prompts: Vec<PromptDef>,
    /// Bumped by every change (a transport that registers the tools
    /// elsewhere syncs when it moved).
    revision: u64,
    /// Expose's tools (app_components, app_get, app_set): the core answers
    /// them, never OnToolCall.
    builtin: Vec<String>,
}

/// What the server says it is (ServerName, ServerTitle, ServerVersion,
/// Instructions, as they were at Start).
#[derive(Clone, Default, Debug)]
pub struct Info {
    pub name: String,
    pub title: String,
    pub version: String,
    pub instructions: String,
}

impl Info {
    /// The component's, as set before Start (the defaults: rapidr-app, 1.0).
    pub fn of<H: Host>(host: H, name: &str) -> Info {
        let text = |p: &str| host.get(name, p).to_string_val().trim().to_string();
        let or = |v: String, d: &str| if v.is_empty() { d.to_string() } else { v };
        Info { name: or(text("servername"), "rapidr-app"), title: text("servertitle"), version: or(text("serverversion"), "1.0"), instructions: host.get(name, "instructions").to_string_val() }
    }
}

/// An AI app the server has heard from.
#[derive(Clone, Debug, PartialEq)]
pub struct Client {
    pub name: String,
    pub version: String,
    /// [`Host::now`] when it was last heard.
    pub last: f64,
    pub calls: u64,
}

/// What the program answered.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    /// Reply: text, a failure when `error`.
    Text { text: String, error: bool },
    /// ReplyImage: a picture and, when not empty, a text before it.
    Image { text: String, data: Vec<u8>, mime: &'static str },
    /// ReplyJson: a whole MCP result object, as written.
    Result(Json),
}

impl Outcome {
    /// As MCP's tool result: `{"content": […], "isError"?}` (ReplyJson's
    /// object as it is).
    pub fn mcp_tool_result(&self) -> Json {
        match self {
            Outcome::Text { text, error } => {
                let mut r = json!({"content": [{"type": "text", "text": text}]});
                if *error {
                    r["isError"] = json!(true);
                }
                r
            }
            Outcome::Image { text, data, mime } => {
                let mut content = Vec::new();
                if !text.is_empty() {
                    content.push(json!({"type": "text", "text": text}));
                }
                content.push(json!({"type": "image", "data": crate::objects::codec::base64_encode(data), "mimeType": mime}));
                json!({"content": content})
            }
            Outcome::Result(v) => v.clone(),
        }
    }

    /// As a WebMCP tool's value (what `execute` resolves to): a text as a
    /// string, a failure as `{"error": "…"}` — data, since an exception's
    /// message never reaches the agent —, a picture as `{"text", "image":
    /// "data:…;base64,…"}`, ReplyJson's `structuredContent` (else its
    /// texts). Texts longer than `max` characters (0: no limit) are cut,
    /// saying so.
    pub fn browser_value(&self, max: usize) -> Json {
        let fit = |t: &str| cut(t, max);
        match self {
            Outcome::Text { text, error: false } => json!(fit(text)),
            Outcome::Text { text, error: true } => json!({"error": fit(text)}),
            Outcome::Image { text, data, mime } => {
                let url = format!("data:{mime};base64,{}", crate::objects::codec::base64_encode(data));
                if text.is_empty() {
                    json!({"image": url})
                } else {
                    json!({"text": fit(text), "image": url})
                }
            }
            Outcome::Result(v) => {
                let texts: Vec<&str> = v.get("content").and_then(Json::as_array).into_iter().flatten().filter_map(|c| c.get("text").and_then(Json::as_str)).collect();
                let joined = texts.join("\n");
                if v.get("isError").and_then(Json::as_bool) == Some(true) {
                    return json!({"error": fit(if joined.is_empty() { "The tool failed." } else { &joined })});
                }
                if let Some(s) = v.get("structuredContent").filter(|s| !s.is_null()) {
                    return s.clone();
                }
                let image = v.get("content").and_then(Json::as_array).into_iter().flatten().find(|c| c.get("type").and_then(Json::as_str) == Some("image"));
                match image {
                    Some(img) => {
                        let mime = img.get("mimeType").and_then(Json::as_str).unwrap_or("image/png");
                        let url = format!("data:{mime};base64,{}", img.get("data").and_then(Json::as_str).unwrap_or(""));
                        json!({"text": fit(&joined), "image": url})
                    }
                    None => json!(fit(&joined)),
                }
            }
        }
    }

    /// The same answer with every text passed through `f` (pictures'
    /// bytes, and a JSON result's `data` and `blob` fields, aren't text).
    fn map_text(self, f: &dyn Fn(&str) -> String) -> Outcome {
        fn walk(v: &mut Json, f: &dyn Fn(&str) -> String) {
            match v {
                Json::String(s) => *s = f(s),
                Json::Array(a) => a.iter_mut().for_each(|x| walk(x, f)),
                Json::Object(o) => o.iter_mut().filter(|(k, _)| !matches!(k.as_str(), "data" | "blob")).for_each(|(_, x)| walk(x, f)),
                _ => {}
            }
        }
        match self {
            Outcome::Text { text, error } => Outcome::Text { text: f(&text), error },
            Outcome::Image { text, data, mime } => Outcome::Image { text: f(&text), data, mime },
            Outcome::Result(mut v) => {
                walk(&mut v, f);
                Outcome::Result(v)
            }
        }
    }
}

/// `text` cut to `max` characters (0: as it is), on a character's edge,
/// saying how much was left out.
pub fn cut(text: &str, max: usize) -> String {
    if max == 0 || text.chars().count() <= max {
        return text.to_string();
    }
    let note_room = 80.min(max / 2);
    let keep: String = text.chars().take(max - note_room).collect();
    let left = text.chars().count() - keep.chars().count();
    format!("{keep}\n… ({left} more characters not shown: ask for a smaller part)")
}

/// Why a call ends without the program's answer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Dropped {
    /// The server stopped (Stop, a new Start, the program's end).
    Stopped,
    /// The person disconnected its AI app (Disconnect).
    Disconnected,
    /// The caller gave up (an aborted browser call).
    Cancelled,
}

/// How a call ends.
#[derive(Clone, Debug, PartialEq)]
pub enum Done {
    /// The program's answer: the call's kind, its tool or resource, the
    /// resource's MIME type ("" for a tool), what the program said
    /// (already scrubbed).
    Answer { kind: Kind, target: String, mime: String, outcome: Outcome },
    Dropped(Dropped),
}

/// Where one call's answer goes: a transport's own (an HTTP connection, a
/// browser's promise). Its methods must not call back into the core.
pub trait Sink {
    /// The call is over: its answer goes out. False when the caller can't
    /// hear it any more (it left).
    fn finish(self: Box<Self>, done: Done) -> bool;
    /// A word on how the call goes, while it waits (Progress): false when
    /// the caller doesn't listen. `server`: the server's name.
    fn progress(&mut self, _server: &str, _message: &str, _done: Option<f64>, _total: Option<f64>) -> bool {
        false
    }
    /// Its handler ran and didn't answer: the call waits.
    fn waiting(&mut self) {}
}

/// A call the transport hands in, checked ([`check_call`]).
#[derive(Clone, Debug)]
pub struct Queued {
    pub kind: Kind,
    /// The tool's own name (an alias resolved), or the resource's URI.
    pub target: String,
    pub args: Json,
    /// Who calls (the AI app's name; [`BROWSER_AGENT`] in a browser).
    pub client: String,
}

/// Why a call is refused before the program sees it.
#[derive(Clone, Debug, PartialEq)]
pub enum Refused {
    /// No such tool: the sentence names the tools there are.
    UnknownTool(String),
    /// No such resource (its URI).
    UnknownResource(String),
    /// Its arguments don't fit the tool's schema: what it needs.
    BadArguments(String),
}

impl Refused {
    /// The sentence the caller reads.
    pub fn sentence(&self) -> String {
        match self {
            Refused::UnknownTool(s) | Refused::BadArguments(s) => s.clone(),
            Refused::UnknownResource(uri) => format!("Resource not found: {uri}"),
        }
    }
}

/// A call the program hasn't answered yet.
struct Call {
    kind: Kind,
    target: String,
    args: Json,
    client: String,
    sink: Option<Box<dyn Sink>>,
}

struct Server {
    info: Info,
    clients: Vec<Client>,
    /// The AI apps the program disconnected (lowercase names): refused
    /// until the server starts again.
    shut_out: Vec<String>,
    calls: HashMap<i64, Call>,
    /// Calls whose OnToolCall / OnReadResource hasn't run yet.
    queue: VecDeque<i64>,
    busy: Rc<Cell<bool>>,
    /// Seconds an AI app may be silent before it counts as gone.
    idle: f64,
}

thread_local! {
    static SERVERS: RefCell<HashMap<String, Server>> = RefCell::new(HashMap::new());
    static DEFS: RefCell<HashMap<String, Defs>> = RefCell::new(HashMap::new());
    /// The calls' numbers: never the same twice in a run.
    static NEXT: Cell<i64> = const { Cell::new(1) };
}

fn key(name: &str) -> String {
    name.to_lowercase()
}

fn with_server<R>(name: &str, f: impl FnOnce(&mut Server) -> R) -> Option<R> {
    SERVERS.with(|s| s.borrow_mut().get_mut(&key(name)).map(f))
}

fn with_defs<R>(name: &str, f: impl FnOnce(&mut Defs) -> R) -> R {
    DEFS.with(|d| f(d.borrow_mut().entry(key(name)).or_default()))
}

fn changed(d: &mut Defs) {
    d.revision += 1;
}

// ------------------------------------------------------------ the server --

/// The server is up (after the transport's own start worked): no client,
/// no call; `idle`: IdleSeconds (0: 15 minutes).
pub fn open<H: Host>(host: H, name: &str, info: Info, idle: f64) {
    close(host, name);
    SERVERS.with(|s| {
        s.borrow_mut().insert(
            key(name),
            Server { info, clients: Vec::new(), shut_out: Vec::new(), calls: HashMap::new(), queue: VecDeque::new(), busy: Rc::new(Cell::new(false)), idle: if idle > 0.0 { idle } else { 15.0 * 60.0 } },
        )
    });
    publish_clients(host, name);
    host.set(name, "pendingcount", v_int(0));
    expose(host, name);
}

/// The server is down: each call still waiting is told
/// ([`Dropped::Stopped`]); Clients, ClientCount, PendingCount go to
/// nothing and Listening to False. False when it wasn't up.
pub fn close<H: Host>(host: H, name: &str) -> bool {
    let Some(mut s) = SERVERS.with(|s| s.borrow_mut().remove(&key(name))) else { return false };
    let mut keys: Vec<i64> = s.calls.keys().copied().collect();
    keys.sort_unstable();
    for k in keys {
        if let Some(sink) = s.calls.remove(&k).and_then(|c| c.sink) {
            sink.finish(Done::Dropped(Dropped::Stopped));
        }
    }
    host.set(name, "listening", v_int(0));
    host.set(name, "clients", v_str(""));
    host.set(name, "clientcount", v_int(0));
    host.set(name, "pendingcount", v_int(0));
    true
}

pub fn is_open(name: &str) -> bool {
    SERVERS.with(|s| s.borrow().contains_key(&key(name)))
}

/// Whether any server is up.
pub fn any_open() -> bool {
    SERVERS.with(|s| !s.borrow().is_empty())
}

/// The servers that are up (their names, lowercase).
pub fn open_names() -> Vec<String> {
    SERVERS.with(|s| s.borrow().keys().cloned().collect())
}

/// What the server said it is at Start.
pub fn info(name: &str) -> Info {
    with_server(name, |s| s.info.clone()).unwrap_or_default()
}

/// The core's turn: AI apps silent too long are gone, the next call goes
/// to the program.
pub fn poll<H: Host>(host: H, name: &str) {
    idle_check(host, name);
    next_call(host, name);
}

// ------------------------------------------------------------- methods --

/// One of RMcpServer's methods that every transport has the same; `None`
/// when `method` is the transport's own (Start, Stop, NewToken) or none.
pub fn method<H: Host>(host: H, name: &str, method: &str, args: &[Value]) -> Option<Value> {
    let text = |i: usize| args.get(i).map(Value::to_string_val).unwrap_or_default();
    let flag = |i: usize| args.get(i).is_some_and(Value::to_bool);
    let call = |i: usize| call_arg(host, name, args, i);
    Some(match method {
        "addtool" => v_int(add_tool(host, name, &text(0), &text(1), &text(2), &text(3)) as i64),
        "removetool" => v_int(remove_tool(name, &text(0)) as i64),
        "cleartools" => {
            with_defs(name, |d| {
                d.tools.clear();
                d.aliases.clear();
                changed(d);
            });
            v_null()
        }
        "addalias" => v_int(add_alias(host, name, &text(0), &text(1)) as i64),
        "addprompt" => v_int(add_prompt(host, name, &text(0), &text(1), &text(2), &text(3)) as i64),
        "clearprompts" => {
            with_defs(name, |d| {
                d.prompts.clear();
                changed(d);
            });
            v_null()
        }
        "toolcount" => v_int(with_defs(name, |d| d.tools.len()) as i64),
        "addresource" => v_int(add_resource(host, name, &text(0), &text(1), &text(2), &text(3)) as i64),
        "clearresources" => {
            with_defs(name, |d| {
                d.resources.clear();
                changed(d);
            });
            v_null()
        }
        "reply" => v_int(reply_text(host, name, call(0), &text(1), flag(2)) as i64),
        "replyimage" => v_int(reply_image(host, name, call(0), &text(1), &text(2)) as i64),
        "replyjson" => v_int(reply_json(host, name, call(0), &text(1)) as i64),
        "progress" => v_int(progress(name, call(0), &text(1), args.get(2).map(Value::to_f64), args.get(3).map(Value::to_f64)) as i64),
        "argument" => v_str(&argument(name, &text(0), call_arg(host, name, args, 1))),
        "isopen" => v_int(is_waiting(name, call(0)) as i64),
        "disconnect" => v_int(disconnect(host, name, &text(0), |_| {}).is_some() as i64),
        _ => return None,
    })
}

/// A method's Call argument (at `at`), else the call being handled.
fn call_arg<H: Host>(host: H, name: &str, args: &[Value], at: usize) -> i64 {
    match args.get(at) {
        Some(v) if !matches!(v, Value::Null) && !v.to_string_val().is_empty() => v.to_i64(),
        _ => host.get(name, "call").to_i64(),
    }
}

fn name_ok(t: &str) -> bool {
    !t.is_empty() && t.len() <= 128 && t.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

/// AddTool: a tool named `tool` (letters, digits, `_`, `-`, `.`), what it
/// does, its arguments (a SUB's parameter list or a JSON Schema; "" —
/// none) and how it acts: "read" (it only reads), "change", "delete",
/// "network", and "untrusted" / "consequential" (said only to a browser's
/// agents) — words joined by spaces or commas —, or annotations as JSON.
/// Added again: replaced, in its place. False (Error says why) when the
/// name or the schema isn't right.
pub fn add_tool<H: Host>(host: H, name: &str, tool: &str, description: &str, schema: &str, how: &str) -> bool {
    let tool = tool.trim();
    let fail = |why: String| {
        host.set(name, "error", v_str(&why));
        false
    };
    if !name_ok(tool) {
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
    let (annotations, untrusted, consequential) = match tool_hints(how) {
        Ok(h) => h,
        Err(why) => return fail(format!("AddTool {tool}: {why}")),
    };
    let title = annotations.as_ref().and_then(|a| a.get("title")).and_then(Json::as_str).unwrap_or("").to_string();
    let def = ToolDef { name: tool.to_string(), title, description: description.to_string(), schema, annotations, untrusted, consequential };
    with_defs(name, |d| {
        match d.tools.iter_mut().find(|t| t.name == def.name) {
            Some(t) => *t = def,
            None => d.tools.push(def),
        }
        changed(d);
    });
    true
}

/// AddTool's Kind: MCP's annotations, and the browser's two hints of its
/// own (untrustedContentHint, consequentialHint), which MCP's don't carry.
#[allow(clippy::type_complexity)]
fn tool_hints(how: &str) -> Result<(Option<Json>, bool, Option<bool>), String> {
    let how = how.trim();
    if how.starts_with('{') {
        let mut a = match serde_json::from_str::<Json>(how) {
            Ok(v) if v.is_object() => v,
            _ => return Err("its annotations aren't a JSON object".into()),
        };
        let o = a.as_object_mut().expect("an object");
        let untrusted = o.remove("untrustedContentHint").and_then(|v| v.as_bool()).unwrap_or(false);
        let consequential = o.remove("consequentialHint").and_then(|v| v.as_bool());
        return Ok((Some(a), untrusted, consequential));
    }
    let mut base: Option<Json> = None;
    let (mut untrusted, mut consequential) = (false, None);
    for word in how.split(|c: char| c == ',' || c.is_whitespace()).filter(|w| !w.is_empty()) {
        let set = |b: &mut Option<Json>, v: Json| -> Result<(), String> {
            if b.is_some() {
                return Err(format!("\"{how}\" says how it acts twice (one of read, change, delete, network)"));
            }
            *b = Some(v);
            Ok(())
        };
        match word.to_ascii_lowercase().as_str() {
            "read" => set(&mut base, json!({"readOnlyHint": true, "openWorldHint": false}))?,
            "change" => set(&mut base, json!({"readOnlyHint": false, "destructiveHint": false, "openWorldHint": false}))?,
            "delete" | "destructive" => set(&mut base, json!({"readOnlyHint": false, "destructiveHint": true, "openWorldHint": false}))?,
            "network" => set(&mut base, json!({"readOnlyHint": false, "openWorldHint": true}))?,
            "untrusted" => untrusted = true,
            "consequential" => consequential = Some(true),
            other => return Err(format!("\"{other}\" isn't read, change, delete, network, untrusted, consequential or annotations in JSON")),
        }
    }
    Ok((base, untrusted, consequential))
}

fn remove_tool(name: &str, tool: &str) -> bool {
    with_defs(name, |d| {
        let n = d.tools.len();
        d.tools.retain(|x| !x.name.eq_ignore_ascii_case(tool));
        let gone = d.tools.len() != n;
        if gone {
            changed(d);
        }
        gone
    })
}

/// A tool's arguments written as a SUB's parameters — `Text AS STRING,
/// [Count AS INTEGER]`, `[ ]` around the optional ones — as the JSON
/// Schema AI apps read. STRING; INTEGER, LONG, SHORT, BYTE, WORD, DWORD;
/// SINGLE, DOUBLE, NUMBER; BOOLEAN; LIST; OBJECT.
pub fn params_schema(text: &str) -> Result<Json, String> {
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
            "INTEGER" | "LONG" | "SHORT" | "BYTE" | "WORD" | "DWORD" | "INT64" | "UINT64" => "integer",
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

/// AddAlias: calls of `alias` (an old name a connected AI app may still
/// use) are calls of `tool`, which the program gets under the tool's own
/// name; the alias is never listed. False (Error says why) when either
/// isn't a tool name.
pub fn add_alias<H: Host>(host: H, name: &str, alias: &str, tool: &str) -> bool {
    let (alias, tool) = (alias.trim(), tool.trim());
    if !name_ok(alias) || !name_ok(tool) || alias == tool {
        host.set(name, "error", v_str(&format!("AddAlias: \"{alias}\" for \"{tool}\" isn't an old name for a tool")));
        return false;
    }
    with_defs(name, |d| {
        d.aliases.retain(|(a, _)| a != alias);
        d.aliases.push((alias.to_string(), tool.to_string()));
        changed(d);
    });
    true
}

/// AddPrompt: a ready-made instruction the AI app's person can pick (MCP's
/// prompts) — its name (letters, digits, _ - .), title, what it is for, and
/// its text, given as one user message. Added again: replaced.
pub fn add_prompt<H: Host>(host: H, name: &str, prompt: &str, title: &str, description: &str, text: &str) -> bool {
    let prompt = prompt.trim();
    if !name_ok(prompt) {
        host.set(name, "error", v_str(&format!("AddPrompt: \"{prompt}\" isn't a prompt name (1 to 128 letters, digits, _ - .)")));
        return false;
    }
    if text.trim().is_empty() {
        host.set(name, "error", v_str(&format!("AddPrompt {prompt}: its text is empty")));
        return false;
    }
    let def = PromptDef { name: prompt.to_string(), title: title.trim().to_string(), description: description.to_string(), text: text.to_string() };
    with_defs(name, |d| {
        match d.prompts.iter_mut().find(|p| p.name == def.name) {
            Some(p) => *p = def,
            None => d.prompts.push(def),
        }
        changed(d);
    });
    true
}

/// AddResource: something AI apps can read, by its URI.
pub fn add_resource<H: Host>(host: H, name: &str, uri: &str, title: &str, description: &str, mime: &str) -> bool {
    let uri = uri.trim();
    if uri.is_empty() || !uri.contains(':') {
        host.set(name, "error", v_str(&format!("AddResource: \"{uri}\" isn't a URI (scheme:…, as file:///notes.txt or app://status)")));
        return false;
    }
    let def = ResourceDef { uri: uri.to_string(), name: if title.trim().is_empty() { uri.to_string() } else { title.trim().to_string() }, description: description.to_string(), mime: if mime.trim().is_empty() { "text/plain".into() } else { mime.trim().to_string() } };
    with_defs(name, |d| {
        match d.resources.iter_mut().find(|r| r.uri == def.uri) {
            Some(r) => *r = def,
            None => d.resources.push(def),
        }
        changed(d);
    });
    true
}

/// The tools offered, in the order they were added.
pub fn tools(name: &str) -> Vec<ToolDef> {
    with_defs(name, |d| d.tools.clone())
}

/// One tool by its name.
pub fn tool(name: &str, tool: &str) -> Option<ToolDef> {
    with_defs(name, |d| d.tools.iter().find(|t| t.name == tool).cloned())
}

pub fn resources(name: &str) -> Vec<ResourceDef> {
    with_defs(name, |d| d.resources.clone())
}

pub fn prompts(name: &str) -> Vec<PromptDef> {
    with_defs(name, |d| d.prompts.clone())
}

/// How many times the definitions changed (a transport that registers the
/// tools elsewhere syncs when it moved).
pub fn revision(name: &str) -> u64 {
    with_defs(name, |d| d.revision)
}

/// Forgets every definition of `name` (a test's server).
pub fn forget(name: &str) {
    DEFS.with(|d| d.borrow_mut().remove(&key(name)));
}

// --------------------------------------------------------------- calls --

/// A call checked before the program sees it: a resource that exists, or
/// a tool (an old name its tool's) whose arguments fit its schema — a list
/// or a number sent as text is that value. Ok: the tool's own name and the
/// arguments as the program gets them.
pub fn check_call(name: &str, kind: Kind, asked: &str, args: Json) -> Result<(String, Json), Refused> {
    if kind == Kind::Resource {
        return if with_defs(name, |d| d.resources.iter().any(|r| r.uri == asked)) { Ok((asked.to_string(), json!({}))) } else { Err(Refused::UnknownResource(asked.to_string())) };
    }
    // (an old name: its tool's call, under the tool's own name)
    let tool = with_defs(name, |d| d.aliases.iter().find(|(a, _)| a.eq_ignore_ascii_case(asked)).map(|(_, t)| t.clone())).unwrap_or_else(|| asked.to_string());
    let Some(def) = self::tool(name, &tool) else {
        let known = with_defs(name, |d| d.tools.iter().map(|t| t.name.clone()).collect::<Vec<_>>().join(", "));
        return Err(Refused::UnknownTool(format!("Unknown tool: {tool}. The tools are: {known}.")));
    };
    let mut args = if args.is_null() { json!({}) } else { args };
    coerce_args(&def, &mut args);
    // (a wrong call: told at once what the tool needs, as a tool's error
    // the AI reads and acts on)
    check_args(&def, &args).map_err(Refused::BadArguments)?;
    Ok((tool, args))
}

/// A checked call kept for the program, its answer to go to `sink`: its
/// number (the program's CallId). PendingCount follows. The program sees
/// it at the next [`poll`].
pub fn enqueue<H: Host>(host: H, name: &str, call: Queued, sink: Box<dyn Sink>) -> Option<i64> {
    let id = NEXT.with(|n| {
        let id = n.get();
        n.set(id + 1);
        id
    });
    let waiting = with_server(name, |s| {
        s.calls.insert(id, Call { kind: call.kind, target: call.target, args: call.args, client: call.client, sink: Some(sink) });
        s.queue.push_back(id);
        s.calls.len()
    })?;
    host.set(name, "pendingcount", v_int(waiting as i64));
    Some(id)
}

/// The calls waiting (the program's answer, or their turn): number, kind,
/// tool or resource, client — oldest first.
pub fn waiting(name: &str) -> Vec<(i64, Kind, String, String)> {
    let mut v: Vec<(i64, Kind, String, String)> = with_server(name, |s| s.calls.iter().map(|(k, c)| (*k, c.kind, c.target.clone(), c.client.clone())).collect()).unwrap_or_default();
    v.sort_by_key(|c| c.0);
    v
}

/// IsOpen: whether call `id` still waits for its answer.
pub fn is_waiting(name: &str, id: i64) -> bool {
    with_server(name, |s| s.calls.contains_key(&id)).unwrap_or(false)
}

/// The caller of call `id` gave up (a browser's aborted call): it goes
/// without an answer ([`Dropped::Cancelled`] to its sink), the program's
/// OnToolCancel (when it has one) hears of it. False when it wasn't
/// waiting.
pub fn cancel<H: Host>(host: H, name: &str, id: i64) -> bool {
    let Some(c) = with_server(name, |s| {
        s.queue.retain(|k| *k != id);
        s.calls.remove(&id)
    })
    .flatten() else {
        return false;
    };
    if let Some(sink) = c.sink {
        sink.finish(Done::Dropped(Dropped::Cancelled));
    }
    publish_pending(host, name);
    if host.has_handler(name, "ontoolcancel") {
        host.fire_then(name, "ontoolcancel", &[v_int(id)], Box::new(|| {}));
    }
    true
}

fn publish_pending<H: Host>(host: H, name: &str) {
    let left = with_server(name, |s| s.calls.len()).unwrap_or(0);
    host.set(name, "pendingcount", v_int(left as i64));
}

/// The next call to the program, once the last one's handler has run.
fn next_call<H: Host>(host: H, name: &str) {
    let next = with_server(name, |s| {
        if s.busy.get() {
            return None;
        }
        while let Some(k) = s.queue.pop_front() {
            if let Some(c) = s.calls.get(&k) {
                s.busy.set(true);
                return Some((k, c.kind, c.target.clone(), c.args.to_string(), c.client.clone(), s.busy.clone(), !s.queue.is_empty()));
            }
        }
        None
    })
    .flatten();
    let Some((id, kind, target, args, client, busy, more)) = next else { return };
    host.set(name, "call", v_int(id));
    host.set(name, "tool", v_str(if kind == Kind::Tool { &target } else { "" }));
    host.set(name, "clientname", v_str(&client));
    host.set(name, "arguments", v_str(&args));
    // (ConfirmChanges: a tool that may change something asks the person
    // first — the app's own question, never the agent's)
    let def = if kind == Kind::Tool { tool(name, &target) } else { None };
    if let Some(d) = def.as_ref().filter(|d| !d.read_only() && host.get(name, "confirmchanges").to_bool()) {
        let ask = crate::i18n::tr_ui("An AI agent wants to use \"{tool}\" in this app: {what}\n\nAllow it?").replace("{tool}", &d.name).replace("{what}", d.description.trim_end_matches('.'));
        let refused = match host.confirm(&ask) {
            Some(true) => None,
            Some(false) => Some("The person using the app didn't allow it. Ask them what they want instead."),
            None => Some("This app asks its user before an AI agent changes something, and it can't ask here."),
        };
        if let Some(why) = refused {
            reply_text(host, name, id, why, true);
            busy.set(false);
            if more {
                host.wake();
            }
            return;
        }
    }
    if kind == Kind::Tool && with_defs(name, |d| d.builtin.contains(&target)) {
        let (text, failed) = exposed_call(host, &target, &args);
        reply_text(host, name, id, &text, failed);
        busy.set(false);
        if more {
            host.wake();
        }
        return;
    }
    let (event, fire_args) = match kind {
        Kind::Tool => ("ontoolcall", vec![v_str(&target), v_str(&args), v_int(id)]),
        Kind::Resource => ("onreadresource", vec![v_str(&target), v_int(id)]),
    };
    if !host.has_handler(name, event) {
        let why = if kind == Kind::Tool { "This program doesn't answer tool calls yet (it has no OnToolCall handler)." } else { "This program doesn't read resources (it has no OnReadResource handler)." };
        reply_text(host, name, id, why, true);
        busy.set(false);
        if more {
            host.wake();
        }
        return;
    }
    let owner = name.to_string();
    host.fire_then(
        name,
        event,
        &fire_args,
        Box::new(move || {
            busy.set(false);
            // (not answered in the handler: it waits)
            let sink = with_server(&owner, |s| s.calls.get_mut(&id).and_then(|c| c.sink.take())).flatten();
            if let Some(mut sink) = sink {
                sink.waiting();
                with_server(&owner, |s| match s.calls.get_mut(&id) {
                    Some(c) => c.sink = Some(sink),
                    None => drop(sink),
                });
            }
            host.wake();
        }),
    );
}

/// The program's answer to call `id`, scrubbed, to its sink.
fn answer<H: Host>(host: H, name: &str, id: i64, outcome: Outcome) -> bool {
    let Some(c) = with_server(name, |s| {
        s.queue.retain(|k| *k != id);
        s.calls.remove(&id)
    })
    .flatten() else {
        return false;
    };
    let mime = if c.kind == Kind::Resource { with_defs(name, |d| d.resources.iter().find(|r| r.uri == c.target).map(|r| r.mime.clone())).unwrap_or_else(|| "text/plain".into()) } else { String::new() };
    let outcome = scrubbed(host, name, outcome);
    let ok = c.sink.is_some_and(|sink| sink.finish(Done::Answer { kind: c.kind, target: c.target, mime, outcome }));
    publish_pending(host, name);
    ok
}

/// What leaves, with key-shaped words ([`Host::scrub`]) and the server's
/// own Token masked.
fn scrubbed<H: Host>(host: H, name: &str, outcome: Outcome) -> Outcome {
    let token = host.get(name, "token").to_string_val();
    let token = token.trim().to_string();
    outcome.map_text(&move |t: &str| {
        let t = host.scrub(t);
        if token.len() >= 16 && t.contains(&token) {
            t.replace(&token, "(the token, not shown)")
        } else {
            t
        }
    })
}

/// Reply: a call answered with text (a tool's result, marked as an error
/// when `failed`; a resource's contents).
fn reply_text<H: Host>(host: H, name: &str, id: i64, text: &str, failed: bool) -> bool {
    answer(host, name, id, Outcome::Text { text: text.to_string(), error: failed })
}

/// The MIME type of an image's bytes, by its signature.
pub fn image_mime(b: &[u8]) -> Option<&'static str> {
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
fn reply_image<H: Host>(host: H, name: &str, id: i64, file: &str, text: &str) -> bool {
    if !is_waiting(name, id) {
        return false;
    }
    let bytes = match host.read_file(file) {
        Ok(b) => b,
        Err(e) => return reply_text(host, name, id, &format!("The picture couldn't be read ({e})."), true),
    };
    let (data, mime) = match image_mime(&bytes) {
        Some(m) => (bytes, m),
        None if bytes.starts_with(b"BM") => match crate::objects::codec::decode_bmp(&bytes).and_then(|img| crate::objects::codec::encode_png(&img)) {
            Ok(png) => (png, "image/png"),
            Err(e) => return reply_text(host, name, id, &format!("The picture couldn't be read ({e})."), true),
        },
        None => return reply_text(host, name, id, "The picture isn't a PNG, JPEG, GIF, WebP or BMP image.", true),
    };
    answer(host, name, id, Outcome::Image { text: text.to_string(), data, mime })
}

/// ReplyJson: a call answered with a whole result object, as written.
fn reply_json<H: Host>(host: H, name: &str, id: i64, text: &str) -> bool {
    match serde_json::from_str::<Json>(text) {
        Ok(v) if v.is_object() => answer(host, name, id, Outcome::Result(v)),
        _ => {
            host.set(name, "error", v_str("ReplyJson: the result must be a JSON object ({\"content\": […]})"));
            false
        }
    }
}

/// Progress: a word on how a call goes, while it waits. False when its
/// caller doesn't listen, or left.
fn progress(name: &str, id: i64, message: &str, done: Option<f64>, total: Option<f64>) -> bool {
    let Some((mut sink, server)) = with_server(name, |s| {
        let server = s.info.name.clone();
        s.calls.get_mut(&id).and_then(|c| c.sink.take()).map(|k| (k, server))
    })
    .flatten() else {
        return false;
    };
    let ok = sink.progress(&server, message, done, total);
    with_server(name, |s| match s.calls.get_mut(&id) {
        Some(c) => c.sink = Some(sink),
        None => drop(sink),
    });
    ok
}

/// Argument: one of a call's arguments as text — a string as it is, a
/// number or true/false as written, a list or an object as JSON, "" when
/// it wasn't given. `a.b` reaches inside an object.
pub fn argument(name: &str, key: &str, id: i64) -> String {
    with_server(name, |s| {
        let c = s.calls.get(&id)?;
        let mut v = &c.args;
        for part in key.split('.') {
            // (an argument's name in any case: a model sends `path` for a
            // tool whose schema says `Path`, and BASIC names don't care)
            v = match v.get(part) {
                Some(found) => found,
                None => v.as_object()?.iter().find(|(k, _)| same_name(k, part))?.1,
            };
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

// -------------------------------------------------------------- Expose --

/// Expose: what of the app's own components the agents may use — ""
/// nothing (the default), "read" (app_components, app_get: what each one
/// shows and its properties), "write" (app_set too: a property set, asked
/// first under ConfirmChanges). Only the windows' components (the visual
/// ones), never a property that holds a credential. Read at Start.
pub fn expose<H: Host>(host: H, name: &str) {
    let level = host.get(name, "expose").to_string_val().trim().to_ascii_lowercase();
    let level = match level.as_str() {
        "1" | "-1" | "true" | "read" => "read",
        "write" | "all" => "write",
        _ => "",
    };
    let had: Vec<String> = with_defs(name, |d| std::mem::take(&mut d.builtin));
    if !had.is_empty() {
        with_defs(name, |d| {
            d.tools.retain(|t| !had.contains(&t.name));
            changed(d);
        });
    }
    if level.is_empty() {
        return;
    }
    let comp = |n: &str| json!({"type": "string", "description": format!("the component's name, as app_components lists it{n}")});
    let mut defs = vec![
        ("app_components", "The app's windows and what is on them: each component's name, type, what it shows (its caption or text) and whether it shows and is enabled. Call it first. Example: {}.", json!({"type": "object", "properties": {}, "additionalProperties": false}), "read untrusted"),
        ("app_get", "One property of one of the app's components (Text, Caption, Checked, ItemIndex …). Example: {\"name\": \"Edit1\", \"property\": \"Text\"}.", json!({"type": "object", "properties": {"name": comp(""), "property": {"type": "string", "description": "the property, e.g. Text"}}, "required": ["name", "property"], "additionalProperties": false}), "read untrusted"),
    ];
    if level == "write" {
        defs.push(("app_set", "Sets one property of one of the app's components, as the program would (the person using the app is asked first). Example: {\"name\": \"Edit1\", \"property\": \"Text\", \"value\": \"Hello\"}.", json!({"type": "object", "properties": {"name": comp(""), "property": {"type": "string"}, "value": {"type": "string", "description": "the new value, as text"}}, "required": ["name", "property", "value"], "additionalProperties": false}), "change"));
    }
    for (tool, description, schema, how) in defs {
        let (annotations, untrusted, consequential) = tool_hints(how).unwrap_or((None, false, None));
        let def = ToolDef { name: tool.into(), title: String::new(), description: description.into(), schema, annotations, untrusted, consequential };
        with_defs(name, |d| {
            d.tools.retain(|t| t.name != tool);
            d.tools.push(def);
            d.builtin.push(tool.to_string());
            changed(d);
        });
    }
}

/// An Expose tool's call, answered by the core: (text, failed).
fn exposed_call<H: Host>(host: H, tool: &str, args: &str) -> (String, bool) {
    let args: Json = serde_json::from_str(args).unwrap_or(json!({}));
    let arg = |k: &str| args.get(k).map(|v| v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string())).unwrap_or_default();
    // (the windows' components only: forms and what is on them)
    let shown: Vec<(String, &'static rapidr_lang::Component)> = host.components().into_iter().filter_map(|(n, t)| rapidr_lang::component(&t).filter(|c| c.visual).map(|c| (n, c))).collect();
    let find = |want: &str| shown.iter().find(|(n, _)| n.eq_ignore_ascii_case(want));
    let prop_of = |c: &'static rapidr_lang::Component, p: &str| c.properties.iter().find(|x| x.name.eq_ignore_ascii_case(p));
    match tool {
        "app_components" => {
            if shown.is_empty() {
                return ("The app has no windows yet.".into(), false);
            }
            let lines: Vec<String> = shown
                .iter()
                .map(|(n, c)| {
                    let says = ["caption", "text"].iter().find(|p| prop_of(c, p).is_some()).map(|p| host.get(n, p).to_string_val()).unwrap_or_default();
                    let says: String = says.chars().take(80).collect();
                    let state = format!("{}{}", if host.get(n, "visible").to_bool() { "shows" } else { "hidden" }, if prop_of(c, "enabled").is_some() && !matches!(host.get(n, "enabled"), Value::Null) && !host.get(n, "enabled").to_bool() { ", disabled" } else { "" });
                    format!("{n} ({}){} — {state}", c.display, if says.is_empty() { String::new() } else { format!(": \"{says}\"") })
                })
                .collect();
            (lines.join("\n"), false)
        }
        "app_get" | "app_set" => {
            let (who, what) = (arg("name"), arg("property"));
            let Some((n, c)) = find(&who) else {
                return (format!("No component named {who}: app_components lists them."), true);
            };
            let Some(p) = prop_of(c, &what) else {
                return (format!("{} has no property {what}.", c.display), true);
            };
            if p.secret {
                return (format!("{}.{} holds a credential: it is never given to an AI agent.", c.display, p.name), true);
            }
            if tool == "app_get" {
                return (host.get(n, p.name).to_string_val(), false);
            }
            if p.access == rapidr_lang::Access::Read {
                return (format!("{}.{} can be read, not set.", c.display, p.name), true);
            }
            host.program_set(n, &p.name.to_ascii_lowercase(), v_str(&arg("value")));
            (format!("{n}.{} is now {}", p.name, host.get(n, p.name).to_string_val()), false)
        }
        _ => (format!("Unknown tool: {tool}."), true),
    }
}

// ------------------------------------------------------------- clients --

/// The AI apps heard from, as they are now.
pub fn clients(name: &str) -> Vec<Client> {
    with_server(name, |s| s.clients.clone()).unwrap_or_default()
}

/// The program's Clients / ClientCount, as they are now.
fn publish_clients<H: Host>(host: H, name: &str) {
    let now = host.now();
    let lines = with_server(name, |s| s.clients.iter().map(|c| format!("{}\t{}\t{}\t{}", c.name, c.version, (now - c.last).max(0.0) as u64, c.calls)).collect::<Vec<_>>()).unwrap_or_default();
    host.set(name, "clientcount", v_int(lines.len() as i64));
    host.set(name, "clients", v_str(&lines.join("\n")));
}

/// A request from `client`: it is in (OnConnect the first time); `call`:
/// it calls a tool.
pub fn heard<H: Host>(host: H, name: &str, client: &str, version: &str, call: bool) {
    let now = host.now();
    let new = with_server(name, |s| match s.clients.iter_mut().find(|c| c.name == client) {
        Some(c) => {
            c.last = now;
            c.calls += u64::from(call);
            false
        }
        None => {
            s.clients.push(Client { name: client.to_string(), version: version.to_string(), last: now, calls: u64::from(call) });
            true
        }
    })
    .unwrap_or(false);
    publish_clients(host, name);
    if new && host.has_handler(name, "onconnect") {
        host.fire_then(name, "onconnect", &[v_str(client)], Box::new(|| {}));
    }
}

/// Whether the person disconnected `client` (refused until the next Start).
pub fn shut_out(name: &str, client: &str) -> bool {
    with_server(name, |s| s.shut_out.contains(&client.to_lowercase())).unwrap_or(false)
}

/// `client` ended its session itself (and has no other): it is gone,
/// OnDisconnect.
pub fn client_gone<H: Host>(host: H, name: &str, client: &str) {
    with_server(name, |s| s.clients.retain(|c| c.name != client));
    publish_clients(host, name);
    if host.has_handler(name, "ondisconnect") {
        host.fire_then(name, "ondisconnect", &[v_str(client)], Box::new(|| {}));
    }
}

/// Clients quiet for IdleSeconds (and with no call waiting) are gone:
/// OnDisconnect.
fn idle_check<H: Host>(host: H, name: &str) {
    let now = host.now();
    let gone = with_server(name, |s| {
        let pending: Vec<String> = s.calls.values().map(|c| c.client.clone()).collect();
        let idle = s.idle;
        let (gone, kept): (Vec<Client>, Vec<Client>) = std::mem::take(&mut s.clients).into_iter().partition(|c| now - c.last > idle && !pending.contains(&c.name));
        s.clients = kept;
        gone.into_iter().map(|c| c.name).collect::<Vec<_>>()
    })
    .unwrap_or_default();
    if gone.is_empty() {
        return;
    }
    publish_clients(host, name);
    for c in gone {
        if host.has_handler(name, "ondisconnect") {
            host.fire_then(name, "ondisconnect", &[v_str(&c)], Box::new(|| {}));
        }
    }
}

/// Disconnect: the AI app named (every one, when "") is out until the
/// server starts again — its calls waiting are told
/// ([`Dropped::Disconnected`]), its next requests refused. `out` hears the
/// names shut out before anything is fired (a transport's own sessions
/// end). The names that were connected, or `None` when the server isn't up.
pub fn disconnect<H: Host>(host: H, name: &str, who: &str, out: impl FnOnce(&[String])) -> Option<Vec<String>> {
    let who = who.trim().to_lowercase();
    let (names, all, dropped) = with_server(name, |s| {
        let names: Vec<String> = s.clients.iter().map(|c| c.name.clone()).filter(|n| who.is_empty() || n.to_lowercase() == who).collect();
        let mut all = names.clone();
        if !who.is_empty() && !all.iter().any(|n| n.to_lowercase() == who) {
            all.push(who.clone());
        }
        for n in &all {
            let low = n.to_lowercase();
            if !s.shut_out.contains(&low) {
                s.shut_out.push(low);
            }
        }
        s.clients.retain(|c| !all.iter().any(|n| n.eq_ignore_ascii_case(&c.name)));
        let mut ids: Vec<i64> = s.calls.iter().filter(|(_, c)| all.iter().any(|n| n.eq_ignore_ascii_case(&c.client))).map(|(k, _)| *k).collect();
        ids.sort_unstable();
        let dropped: Vec<Box<dyn Sink>> = ids.iter().filter_map(|id| s.calls.remove(id).and_then(|c| c.sink)).collect();
        s.queue.retain(|id| s.calls.contains_key(id));
        (names, all, dropped)
    })?;
    out(&all);
    for sink in dropped {
        sink.finish(Done::Dropped(Dropped::Disconnected));
    }
    publish_pending(host, name);
    publish_clients(host, name);
    for n in &names {
        if host.has_handler(name, "ondisconnect") {
            host.fire_then(name, "ondisconnect", &[v_str(n)], Box::new(|| {}));
        }
    }
    Some(names)
}

// --------------------------------------------------- arguments' check --

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

/// Two argument names that are the same but for case and underscores
/// (`Path` and `path`, `ToX` and `to_x`: an older tool list's spelling).
fn same_name(a: &str, b: &str) -> bool {
    let bare = |s: &str| s.chars().filter(|c| *c != '_').map(|c| c.to_ascii_lowercase()).collect::<String>();
    a.eq_ignore_ascii_case(b) || bare(a) == bare(b)
}

/// What a small model sends for a list or an object is often that value as a JSON *string*
/// (`"edits": "[{\"old_text\": …}]"`), a whole number as `"20"`, a flag as `"true"`: the AI
/// app's own tool-call layer didn't parse it. The tool's schema says what it wants, so a
/// string that is exactly that value is that value (found by the outside-AI-app eval,
/// tests/ai-eval/README.md: a small model repeated the refused call to the step limit). A
/// string that isn't is left for `check_args` to refuse in words.
pub fn coerce_args(def: &ToolDef, args: &mut Json) {
    let empty = Map::new();
    let props = def.schema.get("properties").and_then(Json::as_object).unwrap_or(&empty);
    let Some(obj) = args.as_object_mut() else { return };
    // (an argument named in another case — `Path` for `path`, as an older
    // tool list wrote it — is that argument)
    let sent: Vec<String> = obj.keys().cloned().collect();
    for key in sent {
        if props.contains_key(&key) {
            continue;
        }
        if let Some(spelled) = props.keys().find(|p| same_name(p, &key)).filter(|p| !obj.contains_key(*p)) {
            if let Some(v) = obj.remove(&key) {
                obj.insert(spelled.clone(), v);
            }
        }
    }
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
pub fn check_args(def: &ToolDef, args: &Json) -> Result<(), String> {
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
