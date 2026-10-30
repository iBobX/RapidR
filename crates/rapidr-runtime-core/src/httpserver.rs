//! RHttpServer (RapidR's; desktop only): a local HTTP/1.1 server for a
//! program — a web API, a dashboard, an MCP server (Streamable HTTP) —
//! listening on 127.0.0.1 unless told otherwise, on the same connection
//! framing as RServerSocket's Mode "http" (crate::netframe: a request's
//! body arrives whole, by its Content-Length or its chunks).
//!
//! Each connection has a thread of its own that reads its requests and
//! writes their answers; the program sees one request at a time, on its
//! own thread, as OnRequest(Method, Path, Headers, Body) — fired when the
//! window loop (or DOEVENTS) turns, woken by the request's arrival — and
//! answers it then or later: Respond(Status, Headers, Body [, Request]),
//! or a stream of Server-Sent Events (BeginStream, SendEvent, EndStream).
//! The GUI never waits for the network.
//!
//! What the server refuses itself, before the program sees anything:
//! a Host header that isn't the server's own (DNS rebinding: 403), a
//! request a browser marks `Sec-Fetch-Site: cross-site` or whose Origin
//! isn't allowed (a web page of another site: 403), anything but GET and
//! HEAD while no Token is set (401), a missing or wrong `Authorization:
//! Bearer` token when Token is set (401), a
//! request too large (413 / 431) or too slow to arrive (408), more
//! connections than MaxConnections (503), anything that isn't HTTP/1.x
//! (400 / 505). Plain HTTP: no TLS — for 127.0.0.1, where nothing leaves
//! the computer.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use crate::netframe::{self, Framer, Framing, Limits, Piece, Request};
use crate::object::{rp_comp_get, rp_comp_set};
use crate::value::{v_int, v_null, v_str, Value};

/// What a connection's thread is told to write.
pub(crate) enum Out {
    Bytes(Vec<u8>),
    /// The answer is whole: the next request may come, or the connection
    /// closes.
    Done { close: bool },
    /// The connection closed now.
    Close,
}

/// The protections and limits, as they were at Start.
struct Config {
    port: u16,
    /// The address listened on ("127.0.0.1").
    bound: String,
    /// More Host values allowed (lowercase; "*": any).
    hosts: Vec<String>,
    /// More Origins allowed (lowercase, no trailing slash; "*": any).
    origins: Vec<String>,
    token: String,
    limits: Limits,
    timeout: Duration,
    max_connections: usize,
}

/// A request the server refuses itself: its status, why (the body), and
/// a header to add.
type Refusal = (u16, &'static str, Option<(&'static str, &'static str)>);

fn list(text: &str) -> Vec<String> {
    text.split([',', ' ', ';', '\n'])
        .map(|s| s.trim().trim_end_matches('/').to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Whether `a` and `b` are the same bytes, in a time that doesn't tell
/// where they differ.
fn same_secret(a: &[u8], b: &[u8]) -> bool {
    let mut diff = a.len() ^ b.len();
    for (i, x) in a.iter().enumerate() {
        diff |= usize::from(x ^ b.get(i % b.len().max(1)).copied().unwrap_or(0));
    }
    diff == 0
}

impl Config {
    /// The server's own names, as a Host header has them.
    fn own_hosts(&self) -> Vec<String> {
        let p = self.port;
        let mut names = vec![format!("127.0.0.1:{p}"), format!("localhost:{p}"), format!("[::1]:{p}")];
        let b = self.bound.to_ascii_lowercase();
        if !matches!(b.as_str(), "0.0.0.0" | "::" | "127.0.0.1" | "localhost") {
            names.push(if b.contains(':') { format!("[{b}]:{p}") } else { format!("{b}:{p}") });
        }
        if p == 80 {
            names.extend(names.clone().iter().map(|n| n.rsplit_once(':').map_or(n.clone(), |(h, _)| h.to_string())));
        }
        names
    }

    fn host_ok(&self, req: &Request) -> bool {
        if self.hosts.iter().any(|h| h == "*") {
            return true;
        }
        // (HTTP/1.0 may leave it out; a browser never does)
        let Some(host) = req.header("host") else { return true };
        let host = host.trim().to_ascii_lowercase();
        self.own_hosts().contains(&host) || self.hosts.contains(&host)
    }

    fn origin_ok(&self, req: &Request) -> bool {
        let Some(origin) = req.header("origin") else { return true };
        let origin = origin.trim().trim_end_matches('/').to_ascii_lowercase();
        if self.origins.iter().any(|o| o == "*" || *o == origin) {
            return true;
        }
        self.own_hosts().iter().any(|h| origin == format!("http://{h}"))
    }

    fn token_ok(&self, req: &Request) -> bool {
        if self.token.is_empty() {
            return true;
        }
        // (a browser's CORS preflight carries no credentials: an allowed
        // page's reaches the program, which answers it)
        if req.method == "OPTIONS" && req.header("origin").is_some() && req.header("access-control-request-method").is_some() {
            return true;
        }
        let given = req.header("authorization").unwrap_or("").trim();
        let Some((scheme, value)) = given.split_once(' ') else { return false };
        scheme.eq_ignore_ascii_case("bearer") && same_secret(value.trim().as_bytes(), self.token.as_bytes())
    }

    /// The refusal for `req`, if the server refuses it itself.
    fn check(&self, req: &Request) -> Option<Refusal> {
        if !self.host_ok(req) {
            return Some((403, "This server doesn't answer requests for that host name.", None));
        }
        // (a browser says a request comes from another site's page —
        // a link, an image, a script, a form — even where it sends no
        // Origin)
        if req.header("sec-fetch-site").is_some_and(|s| s.trim().eq_ignore_ascii_case("cross-site")) {
            return Some((403, "Requests from another site's web page aren't allowed here.", None));
        }
        if !self.origin_ok(req) {
            return Some((403, "Requests from that web page aren't allowed here.", None));
        }
        // (without a Token, only reads: anything that could change
        // something needs one)
        if self.token.is_empty() && !matches!(req.method.as_str(), "GET" | "HEAD") {
            return Some((401, "This server has no Token, so it only answers GET and HEAD: set one to accept changes.", None));
        }
        if !self.token_ok(req) {
            return Some((401, "This server needs its token: Authorization: Bearer <token>.", Some(("WWW-Authenticate", "Bearer realm=\"RapidR\""))));
        }
        None
    }
}

/// A request that arrived, for the program (RHttpServer's, RMcpServer's).
pub(crate) struct Arrived {
    pub(crate) id: i64,
    pub(crate) request: Request,
    pub(crate) peer: String,
    pub(crate) conn: mpsc::Sender<Out>,
}

/// A request the program has seen and not finished answering.
struct Open {
    conn: mpsc::Sender<Out>,
    request: Request,
    /// Once BeginStream: whether its events go as chunks (HTTP/1.1).
    streaming: Option<bool>,
}

struct Server {
    running: Arc<AtomicBool>,
    events: mpsc::Receiver<Arrived>,
    queue: VecDeque<Arrived>,
    open: HashMap<i64, Open>,
    /// A request's OnRequest is running: the next waits for it.
    busy: Rc<Cell<bool>>,
    current: i64,
}

thread_local! {
    static SERVERS: RefCell<HashMap<String, Server>> = RefCell::new(HashMap::new());
}

fn with_server<R>(name: &str, f: impl FnOnce(&mut Server) -> R) -> Option<R> {
    SERVERS.with(|s| s.borrow_mut().get_mut(name).map(f))
}

/// RHttpServer `name`'s method.
pub fn method(name: &str, method: &str, args: &[Value]) -> Value {
    let name = name.to_lowercase();
    let text = |i: usize| args.get(i).map(Value::to_string_val).unwrap_or_default();
    match method {
        "start" => start(&name),
        "stop" => {
            stop(&name);
            v_null()
        }
        "respond" => respond(&name, args),
        "beginstream" => begin_stream(&name, &text(0), request_arg(&name, args, 1)),
        "sendevent" => send_event(&name, &text(0), &text(1), request_arg(&name, args, 2)),
        "endstream" => end_stream(&name, request_arg(&name, args, 0)),
        "close" => close(&name, request_arg(&name, args, 0)),
        "newtoken" => v_str(&new_token(&name)),
        "header" => {
            let id = request_arg(&name, args, 1);
            let header = text(0);
            v_str(&with_server(&name, |s| s.open.get(&id).and_then(|o| o.request.header(&header)).unwrap_or("").to_string()).unwrap_or_default())
        }
        _ => {
            eprintln!("[WARN] RHttpServer.{method}() not implemented");
            v_null()
        }
    }
}

/// 32 bytes from the operating system's secure random source, as 64 hex
/// digits; `None` when the system gives none.
pub fn random_token() -> Option<String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).ok()?;
    Some(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// NewToken: Token set to a fresh random one (the system's secure random
/// source), which is returned; "" (and Error saying why) when the system
/// gives none. Like Token, it applies from the next Start.
fn new_token(name: &str) -> String {
    match random_token() {
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

/// A method's Request argument (at `at`), else the request being handled.
fn request_arg(name: &str, args: &[Value], at: usize) -> i64 {
    match args.get(at) {
        Some(v) if !matches!(v, Value::Null) => v.to_i64(),
        _ => with_server(name, |s| s.current).unwrap_or(0),
    }
}

fn prop_text(name: &str, prop: &str) -> String {
    rp_comp_get(name, prop).to_string_val()
}

fn failed(name: &str, why: String) -> Value {
    eprintln!("[HttpServer] {why}");
    rp_comp_set(name, "error", v_str(&why));
    rp_comp_set(name, "listening", v_int(0));
    v_int(0)
}

fn start(name: &str) -> Value {
    stop(name);
    let options = ListenOptions::of(name);
    let l = match listen(&options) {
        Ok(l) => l,
        Err(why) => return failed(name, why),
    };
    SERVERS.with(|s| {
        s.borrow_mut().insert(name.to_string(), Server { running: l.running, events: l.events, queue: VecDeque::new(), open: HashMap::new(), busy: Rc::new(Cell::new(false)), current: 0 });
    });
    rp_comp_set(name, "port", v_int(i64::from(l.port)));
    rp_comp_set(name, "url", v_str(&l.url));
    rp_comp_set(name, "error", v_str(""));
    rp_comp_set(name, "listening", v_int(1));
    v_int(1)
}

/// What a server listens on and refuses (its component's properties at
/// Start).
pub(crate) struct ListenOptions {
    pub(crate) host: String,
    pub(crate) port: i64,
    pub(crate) hosts: String,
    pub(crate) origins: String,
    pub(crate) token: String,
    pub(crate) max_body: i64,
    pub(crate) timeout_ms: i64,
    pub(crate) max_connections: i64,
}

impl ListenOptions {
    /// Component `name`'s: Host, Port, AllowedHosts, AllowedOrigins,
    /// Token, MaxRequestSize, Timeout, MaxConnections.
    pub(crate) fn of(name: &str) -> ListenOptions {
        ListenOptions {
            host: prop_text(name, "host").trim().to_string(),
            port: rp_comp_get(name, "port").to_i64(),
            hosts: prop_text(name, "allowedhosts"),
            origins: prop_text(name, "allowedorigins"),
            token: prop_text(name, "token").trim().to_string(),
            max_body: rp_comp_get(name, "maxrequestsize").to_i64(),
            timeout_ms: rp_comp_get(name, "timeout").to_i64(),
            max_connections: rp_comp_get(name, "maxconnections").to_i64(),
        }
    }
}

/// A server listening: the requests its connections read arrive on
/// `events` (the connections' threads wake the loop for each).
pub(crate) struct Listening {
    pub(crate) running: Arc<AtomicBool>,
    pub(crate) events: mpsc::Receiver<Arrived>,
    pub(crate) port: u16,
    pub(crate) url: String,
}

/// Listens as `o` says: a thread accepts the connections, a thread each
/// reads its requests — refused by the server itself when they don't
/// belong (Config::check) — and writes their answers.
pub(crate) fn listen(o: &ListenOptions) -> Result<Listening, String> {
    let host = if o.host.is_empty() { "127.0.0.1".to_string() } else { o.host.clone() };
    let port = o.port;
    if !(0..=65535).contains(&port) {
        return Err(format!("Port {port} isn't a port number (1 to 65535, or 0 for any free one)"));
    }
    let listener = TcpListener::bind((host.as_str(), port as u16)).map_err(|e| format!("can't listen on {host}:{port}: {e}"))?;
    let port = listener.local_addr().map(|a| a.port()).unwrap_or(port as u16);
    let _ = listener.set_nonblocking(true);
    let cfg = Arc::new(Config {
        port,
        bound: host.clone(),
        hosts: list(&o.hosts),
        origins: list(&o.origins),
        token: o.token.clone(),
        limits: Limits { max_head: 64 * 1024, max_body: if o.max_body > 0 { o.max_body as usize } else { Limits::default().max_body } },
        timeout: Duration::from_millis(if o.timeout_ms > 0 { o.timeout_ms as u64 } else { 30_000 }),
        max_connections: if o.max_connections > 0 { o.max_connections as usize } else { 32 },
    });
    let running = Arc::new(AtomicBool::new(true));
    let (tx, rx) = mpsc::channel::<Arrived>();
    let ids = Arc::new(AtomicI64::new(1));
    let active = Arc::new(AtomicUsize::new(0));
    {
        let running = running.clone();
        std::thread::spawn(move || accept_loop(listener, cfg, tx, ids, running, active));
    }
    let shown = if host.contains(':') { format!("[{host}]") } else if host == "0.0.0.0" { "127.0.0.1".to_string() } else { host };
    Ok(Listening { running, events: rx, port, url: format!("http://{shown}:{port}/") })
}

fn stop(name: &str) {
    if let Some(s) = SERVERS.with(|s| s.borrow_mut().remove(name)) {
        s.running.store(false, Ordering::Relaxed);
        // (each open request's connection closed: its thread's channel ends)
        for (_, o) in s.open {
            let _ = o.conn.send(Out::Close);
        }
        rp_comp_set(name, "listening", v_int(0));
    }
}

/// Whether a server is listening (the window loop then hears requests).
pub fn busy() -> bool {
    SERVERS.with(|s| !s.borrow().is_empty())
}

/// The requests that arrived, given to the program one at a time:
/// OnRequest fired for the next once the last one's handler has run.
/// (The window loop's turn and DOEVENTS call it.)
pub fn poll() -> bool {
    let names: Vec<String> = SERVERS.with(|s| s.borrow().keys().cloned().collect());
    for name in &names {
        poll_one(name);
    }
    !names.is_empty()
}

fn poll_one(name: &str) {
    let next = with_server(name, |s| {
        while let Ok(a) = s.events.try_recv() {
            s.queue.push_back(a);
        }
        if s.busy.get() {
            return None;
        }
        let a = s.queue.pop_front()?;
        s.current = a.id;
        s.busy.set(true);
        s.open.insert(a.id, Open { conn: a.conn, request: a.request.clone(), streaming: None });
        Some((a.id, a.request, a.peer, s.busy.clone(), !s.queue.is_empty()))
    })
    .flatten();
    let Some((id, req, peer, busy, more)) = next else { return };
    rp_comp_set(name, "request", v_int(id));
    rp_comp_set(name, "method", v_str(&req.method));
    rp_comp_set(name, "path", v_str(req.path()));
    rp_comp_set(name, "query", v_str(req.query()));
    rp_comp_set(name, "peer", v_str(&peer));
    if !crate::object::rp_has_handler(name, "onrequest") {
        respond_to(name, id, 404, Vec::new(), "Not found: this program doesn't answer requests (it has no OnRequest handler).\n");
        busy.set(false);
        if more {
            crate::object::rp_wake();
        }
        return;
    }
    let args = [v_str(&req.method), v_str(req.path()), v_str(&req.header_text()), v_str(&req.body_text())];
    crate::object::rp_fire_event_then(name, "onrequest", &args, move |_| {
        busy.set(false);
        // (the next request waiting: the loop turns again for it)
        crate::object::rp_wake();
    });
}

/// The open request `id` taken (one being streamed only when `streaming`).
fn take_open(name: &str, id: i64, streaming: bool) -> Option<Open> {
    with_server(name, |s| {
        let ok = s.open.get(&id).is_some_and(|o| o.streaming.is_some() == streaming);
        if ok { s.open.remove(&id) } else { None }
    })
    .flatten()
}

fn respond(name: &str, args: &[Value]) -> Value {
    let status = args.first().map(Value::to_i64).filter(|s| (100..=599).contains(s)).unwrap_or(200) as u16;
    let headers = netframe::header_lines(&args.get(1).map(Value::to_string_val).unwrap_or_default());
    let body = args.get(2).map(Value::to_string_val).unwrap_or_default();
    let id = request_arg(name, args, 3);
    v_int(respond_to(name, id, status, headers, &body) as i64)
}

fn respond_to(name: &str, id: i64, status: u16, headers: Vec<(String, String)>, body: &str) -> bool {
    let Some(open) = take_open(name, id, false) else { return false };
    write_response(&open.conn, &open.request, status, headers, body.as_bytes())
}

/// Writes a whole answer to `request` on its connection `conn`: its
/// status, headers (a text body gets a Content-Type) and body. False when
/// the client went away.
pub(crate) fn write_response(conn: &mpsc::Sender<Out>, request: &Request, status: u16, mut headers: Vec<(String, String)>, body: &[u8]) -> bool {
    let bodiless = status == 204 || status == 304 || (100..200).contains(&status);
    if !bodiless && !body.is_empty() && !headers.iter().any(|(n, _)| n.eq_ignore_ascii_case("content-type")) {
        headers.push(("Content-Type".into(), "text/plain; charset=utf-8".into()));
    }
    let close = !request.keep_alive();
    let mut bytes = netframe::response_head(status, &headers, (!bodiless).then_some(body.len()), false, close).into_bytes();
    if !bodiless && !request.method.eq_ignore_ascii_case("HEAD") {
        bytes.extend_from_slice(body);
    }
    conn.send(Out::Bytes(bytes)).is_ok() && conn.send(Out::Done { close }).is_ok()
}

/// Begins answering `request` with a stream of Server-Sent Events (200,
/// `text/event-stream`, not cached, not buffered by a proxy, and
/// `headers`): Some(chunked) when it began — HTTP/1.1's events go as
/// chunks, HTTP/1.0's until the connection closes.
pub(crate) fn write_stream_head(conn: &mpsc::Sender<Out>, request: &Request, headers: &str) -> Option<bool> {
    let mut headers = netframe::header_lines(headers);
    for (n, v) in [("Content-Type", "text/event-stream"), ("Cache-Control", "no-cache"), ("X-Accel-Buffering", "no")] {
        if !headers.iter().any(|(h, _)| h.eq_ignore_ascii_case(n)) {
            headers.push((n.into(), v.into()));
        }
    }
    let chunked = request.version.eq_ignore_ascii_case("HTTP/1.1");
    let close = !chunked || !request.keep_alive();
    let head = netframe::response_head(200, &headers, None, chunked, close);
    conn.send(Out::Bytes(head.into_bytes())).is_ok().then_some(chunked)
}

/// Sends one event on a stream [`write_stream_head`] began. False when the
/// client went away.
pub(crate) fn write_event(conn: &mpsc::Sender<Out>, chunked: bool, data: &str, event: &str) -> bool {
    let text = netframe::sse_event(data, event);
    let bytes = if chunked { netframe::chunk(text.as_bytes()) } else { text.into_bytes() };
    conn.send(Out::Bytes(bytes)).is_ok()
}

/// Ends a stream [`write_stream_head`] began.
pub(crate) fn write_stream_end(conn: &mpsc::Sender<Out>, request: &Request, chunked: bool) -> bool {
    let close = !chunked || !request.keep_alive();
    (!chunked || conn.send(Out::Bytes(b"0\r\n\r\n".to_vec())).is_ok()) && conn.send(Out::Done { close }).is_ok()
}

/// Closes a request's connection now, without an answer.
pub(crate) fn close_conn(conn: &mpsc::Sender<Out>) -> bool {
    conn.send(Out::Close).is_ok()
}

fn begin_stream(name: &str, headers: &str, id: i64) -> Value {
    let Some(mut open) = take_open(name, id, false) else { return v_int(0) };
    let Some(chunked) = write_stream_head(&open.conn, &open.request, headers) else { return v_int(0) };
    open.streaming = Some(chunked);
    with_server(name, |s| s.open.insert(id, open));
    v_int(1)
}

fn send_event(name: &str, data: &str, event: &str, id: i64) -> Value {
    let sent = with_server(name, |s| {
        let Some(open) = s.open.get(&id) else { return false };
        let Some(chunked) = open.streaming else { return false };
        let ok = write_event(&open.conn, chunked, data, event);
        // (the client went away: the stream is over)
        if !ok {
            s.open.remove(&id);
        }
        ok
    });
    v_int(sent.unwrap_or(false) as i64)
}

fn end_stream(name: &str, id: i64) -> Value {
    let Some(open) = take_open(name, id, true) else { return v_int(0) };
    v_int(write_stream_end(&open.conn, &open.request, open.streaming == Some(true)) as i64)
}

fn close(name: &str, id: i64) -> Value {
    let open = with_server(name, |s| s.open.remove(&id)).flatten();
    v_int(open.is_some_and(|o| close_conn(&o.conn)) as i64)
}

// ------------------------------------------------- the connections' side --

fn accept_loop(listener: TcpListener, cfg: Arc<Config>, events: mpsc::Sender<Arrived>, ids: Arc<AtomicI64>, running: Arc<AtomicBool>, active: Arc<AtomicUsize>) {
    while running.load(Ordering::Relaxed) {
        let (mut stream, peer) = match listener.accept() {
            Ok(c) => c,
            Err(_) => {
                std::thread::sleep(Duration::from_millis(15));
                continue;
            }
        };
        let _ = stream.set_nonblocking(false);
        if active.load(Ordering::Relaxed) >= cfg.max_connections {
            let _ = stream.set_write_timeout(Some(Duration::from_secs(1)));
            refuse(&mut stream, (503, "Too many connections: try again soon.", None));
            continue;
        }
        active.fetch_add(1, Ordering::Relaxed);
        let (cfg, events, ids, running, active) = (cfg.clone(), events.clone(), ids.clone(), running.clone(), active.clone());
        std::thread::spawn(move || {
            serve(stream, peer.to_string(), &cfg, &events, &ids, &running);
            active.fetch_sub(1, Ordering::Relaxed);
        });
    }
}

/// The refusal written, and the connection closed — after what the client
/// still sends is read for a moment, so it sees the answer.
fn refuse(stream: &mut TcpStream, (status, why, extra): Refusal) {
    let body = format!("{why}\n");
    let mut headers = vec![("Content-Type".to_string(), "text/plain; charset=utf-8".to_string())];
    if let Some((n, v)) = extra {
        headers.push((n.into(), v.into()));
    }
    let head = netframe::response_head(status, &headers, Some(body.len()), false, true);
    let _ = stream.write_all(format!("{head}{body}").as_bytes());
    let _ = stream.flush();
    let _ = stream.shutdown(Shutdown::Write);
    let until = Instant::now() + Duration::from_millis(500);
    let _ = stream.set_read_timeout(Some(Duration::from_millis(100)));
    let mut sink = [0u8; 8192];
    while Instant::now() < until {
        match stream.read(&mut sink) {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
            Err(_) => break,
        }
    }
}

/// One connection, until it closes: its requests read whole, refused or
/// handed to the program, and each answer written as the program gives
/// it, before the next request is taken.
fn serve(mut stream: TcpStream, peer: String, cfg: &Config, events: &mpsc::Sender<Arrived>, ids: &AtomicI64, running: &AtomicBool) {
    let _ = stream.set_read_timeout(Some(Duration::from_millis(100)));
    let _ = stream.set_write_timeout(Some(cfg.timeout));
    let _ = stream.set_nodelay(true);
    let mut framer = Framer::new(Framing::Http(cfg.limits));
    let mut ready: VecDeque<Request> = VecDeque::new();
    let mut buf = vec![0u8; 64 * 1024];
    let mut idle = Instant::now();
    let mut begun: Option<Instant> = None;
    while running.load(Ordering::Relaxed) {
        if let Some(req) = ready.pop_front() {
            if let Some(r) = cfg.check(&req) {
                refuse(&mut stream, r);
                return;
            }
            let keep = req.keep_alive();
            let (tx, rx) = mpsc::channel();
            let id = ids.fetch_add(1, Ordering::Relaxed);
            if events.send(Arrived { id, request: req, peer: peer.clone(), conn: tx }).is_err() {
                return;
            }
            crate::object::rp_wake();
            // (the answer, as the program gives it)
            loop {
                match rx.recv_timeout(Duration::from_millis(200)) {
                    Ok(Out::Bytes(b)) => {
                        if stream.write_all(&b).is_err() {
                            return;
                        }
                    }
                    Ok(Out::Done { close }) => {
                        let _ = stream.flush();
                        if close || !keep {
                            let _ = stream.shutdown(Shutdown::Both);
                            return;
                        }
                        break;
                    }
                    Ok(Out::Close) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                        let _ = stream.shutdown(Shutdown::Both);
                        return;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        if !running.load(Ordering::Relaxed) {
                            return;
                        }
                    }
                }
            }
            idle = Instant::now();
            continue;
        }
        match stream.read(&mut buf) {
            Ok(0) => return,
            Ok(n) => {
                begun.get_or_insert_with(Instant::now);
                for p in framer.feed(&buf[..n]) {
                    match p {
                        Piece::Request(r) => ready.push_back(r),
                        Piece::Continue(head) => {
                            // (refused before its body is invited)
                            if let Some(r) = cfg.check(&head) {
                                refuse(&mut stream, r);
                                return;
                            }
                            if stream.write_all(b"HTTP/1.1 100 Continue\r\n\r\n").is_err() {
                                return;
                            }
                        }
                        Piece::Refuse(status, why) => {
                            let why: &'static str = match status {
                                413 => "The request is larger than this server takes (MaxRequestSize).",
                                431 => "The request's headers are too large.",
                                505 => "Only HTTP/1.0 and HTTP/1.1 are spoken here.",
                                501 => "Only chunked transfer is understood.",
                                _ => {
                                    eprintln!("[HttpServer] refused a request from {peer}: {why}");
                                    "Not a request this server understands."
                                }
                            };
                            refuse(&mut stream, (status, why, None));
                            return;
                        }
                        Piece::Text(_) => {}
                    }
                }
                if !framer.in_progress() {
                    begun = None;
                    idle = Instant::now();
                }
            }
            Err(e) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut | std::io::ErrorKind::Interrupted) => {
                if let Some(t) = begun {
                    if t.elapsed() > cfg.timeout {
                        refuse(&mut stream, (408, "The request took too long to arrive.", None));
                        return;
                    }
                } else if idle.elapsed() > cfg.timeout {
                    return;
                }
            }
            Err(_) => return,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(token: &str, origins: &str) -> Config {
        Config { port: 47501, bound: "127.0.0.1".into(), hosts: Vec::new(), origins: list(origins), token: token.into(), limits: Limits::default(), timeout: Duration::from_secs(5), max_connections: 4 }
    }

    fn req(headers: &[(&str, &str)]) -> Request {
        Request { method: "POST".into(), target: "/mcp".into(), version: "HTTP/1.1".into(), headers: headers.iter().map(|(n, v)| (n.to_string(), v.to_string())).collect(), body: Vec::new() }
    }

    #[test]
    fn the_server_refuses_foreign_hosts_origins_and_missing_tokens() {
        let c = cfg("s3cret", "https://ok.example");
        let status = |h: &[(&str, &str)]| c.check(&req(h)).map(|r| r.0);
        assert_eq!(status(&[("Host", "127.0.0.1:47501"), ("Authorization", "Bearer s3cret")]), None);
        assert_eq!(status(&[("Host", "localhost:47501"), ("Authorization", "bearer s3cret")]), None, "the scheme in any case");
        assert_eq!(status(&[("Host", "127.0.0.1:47501")]), Some(401), "no token");
        assert_eq!(status(&[("Host", "127.0.0.1:47501"), ("Authorization", "Bearer s3cre")]), Some(401), "a wrong one");
        assert_eq!(status(&[("Host", "evil.example:47501"), ("Authorization", "Bearer s3cret")]), Some(403), "DNS rebinding");
        assert_eq!(status(&[("Host", "127.0.0.1:47501"), ("Origin", "https://evil.example"), ("Authorization", "Bearer s3cret")]), Some(403));
        assert_eq!(status(&[("Host", "127.0.0.1:47501"), ("Origin", "https://ok.example/"), ("Authorization", "Bearer s3cret")]), None, "an allowed page");
        assert_eq!(status(&[("Host", "127.0.0.1:47501"), ("Origin", "http://localhost:47501"), ("Authorization", "Bearer s3cret")]), None, "the server's own page");
        // (a preflight from an allowed page reaches the program without the token)
        let mut pre = req(&[("Host", "127.0.0.1:47501"), ("Origin", "https://ok.example"), ("Access-Control-Request-Method", "POST")]);
        pre.method = "OPTIONS".into();
        assert_eq!(c.check(&pre).map(|r| r.0), None);
        // (a link, an image or a script of another site's page: no Origin,
        // but the browser says cross-site)
        assert_eq!(status(&[("Host", "127.0.0.1:47501"), ("Sec-Fetch-Site", "cross-site"), ("Authorization", "Bearer s3cret")]), Some(403));
        assert_eq!(status(&[("Host", "127.0.0.1:47501"), ("Sec-Fetch-Site", "same-origin"), ("Authorization", "Bearer s3cret")]), None);
    }

    /// Without a Token only reads are answered: GET and HEAD; a POST (or
    /// PUT, DELETE …) is refused with 401, and a cross-site GET with 403.
    #[test]
    fn without_a_token_only_reads_pass() {
        let open = cfg("", "");
        let with = |method: &str, h: &[(&str, &str)]| {
            let mut r = req(h);
            r.method = method.into();
            open.check(&r).map(|r| r.0)
        };
        let host = [("Host", "127.0.0.1:47501")];
        assert_eq!(with("GET", &host), None, "a GET is answered");
        assert_eq!(with("HEAD", &host), None);
        assert_eq!(with("POST", &host), Some(401), "a POST without a Token set is refused");
        assert_eq!(with("DELETE", &host), Some(401));
        assert_eq!(with("OPTIONS", &[("Host", "127.0.0.1:47501"), ("Origin", "http://localhost:47501"), ("Access-Control-Request-Method", "POST")]), Some(401));
        assert_eq!(with("GET", &[("Host", "127.0.0.1:47501"), ("Sec-Fetch-Site", "cross-site")]), Some(403), "a cross-site GET is refused");
        assert!(open.check(&req(&[("Host", "127.0.0.1:47501")])).is_some_and(|r| r.1.contains("no Token")), "it says why in plain words");
    }

    #[test]
    fn a_new_token_is_long_and_never_the_same() {
        let (a, b) = (random_token().expect("the system's random source"), random_token().unwrap());
        assert_eq!(a.len(), 64);
        assert!(a.bytes().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }

    #[test]
    fn secrets_compare_whole() {
        assert!(same_secret(b"abc", b"abc"));
        assert!(!same_secret(b"abc", b"abd"));
        assert!(!same_secret(b"ab", b"abc"));
        assert!(!same_secret(b"", b"a"));
        assert!(same_secret(b"", b""));
    }
}
