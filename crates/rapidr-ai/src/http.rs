//! The transport: requests run in the background — a thread with ureq
//! (the system's TLS) on the desktop, `fetch` on a web page — and what
//! they get back waits in an inbox the caller's loop empties ([`take`]):
//! the answer's status, its body's bytes as they come, its end. A request
//! the provider refuses for a passing reason (429 rate limits, 5xx, 529
//! "overloaded") is tried again by itself, waiting what `retry-after` says
//! (at most 20 seconds), before the error is passed on. So is a request
//! that never reached the provider — the name not found (DNS), no
//! connection, a connection closed before any answer —, once, 2 seconds
//! later ([`NET_RETRY_WAIT`]): a network's passing hiccup no longer ends a
//! question (Robert's evening with DeepSeek, 2026-10-20). A refusal (4xx)
//! is never tried again, nor an answer that stopped coming half-way (the
//! model would be paid twice for it).

use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use crate::wire::HttpRequest;

/// What a request got back, in order: a `Status`, `Data` pieces, `Done`.
#[derive(Clone, Debug, PartialEq)]
pub enum Incoming {
    /// The answer's status, and `retry-after` (seconds) when it said.
    Status { status: u16, retry_after: Option<u64> },
    /// A piece of the answer's body.
    Data(Vec<u8>),
    /// The transfer ended: `Ok`, or why there was no (whole) answer.
    Done(Result<(), String>),
}

#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// Seconds to connect.
    pub connect_timeout: u64,
    /// Seconds the answer may go quiet (a model thinking before it writes).
    pub read_timeout: u64,
    /// How many times a passing refusal is tried again.
    pub retries: u32,
}

impl Default for Options {
    fn default() -> Options {
        Options { connect_timeout: 20, read_timeout: 180, retries: 2 }
    }
}

/// Seconds before a request that never reached the provider is tried again
/// (once).
pub const NET_RETRY_WAIT: u64 = 2;

static INBOX: Mutex<Vec<(u64, Incoming)>> = Mutex::new(Vec::new());
static CANCELLED: Mutex<Option<HashSet<u64>>> = Mutex::new(None);
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
static NOTIFY: Mutex<Option<fn()>> = Mutex::new(None);

/// What came back since the last call, every request's, in order.
pub fn take() -> Vec<(u64, Incoming)> {
    std::mem::take(&mut *INBOX.lock().unwrap_or_else(|e| e.into_inner()))
}

/// How many requests are running.
pub fn active() -> usize {
    ACTIVE.load(Ordering::SeqCst)
}

/// Called after news arrive (the web page's loop has no polling: it
/// delivers them at once). On the desktop it would be called from the
/// request's thread, so it isn't: the caller polls while [`active`].
pub fn set_notify(f: Option<fn()>) {
    *NOTIFY.lock().unwrap_or_else(|e| e.into_inner()) = f;
}

/// Stops request `id`: nothing more of it arrives but its `Done`.
pub fn cancel(id: u64) {
    CANCELLED.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashSet::new).insert(id);
    #[cfg(target_arch = "wasm32")]
    web::abort(id);
}

fn cancelled(id: u64) -> bool {
    CANCELLED.lock().unwrap_or_else(|e| e.into_inner()).as_ref().is_some_and(|c| c.contains(&id))
}

fn push(id: u64, what: Incoming) {
    INBOX.lock().unwrap_or_else(|e| e.into_inner()).push((id, what));
}

fn finished(id: u64) {
    if let Some(c) = CANCELLED.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
        c.remove(&id);
    }
    ACTIVE.fetch_sub(1, Ordering::SeqCst);
}

#[cfg(target_arch = "wasm32")]
fn notify() {
    let f = *NOTIFY.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(f) = f {
        f();
    }
}

/// Whether an error answer is worth waiting for and trying again: a rate
/// limit or the server's trouble, not an account without credit.
pub fn passing(status: u16, body: &str) -> bool {
    let low = body.to_ascii_lowercase();
    matches!(status, 429 | 500 | 502 | 503 | 504 | 529) && !low.contains("insufficient_quota") && !low.contains("insufficient balance") && !low.contains("credit") && !low.contains("quota")
}

/// How long to wait before try `attempt` (from 1): `retry-after`, else
/// 2 s, 6 s …, never more than 20 s.
pub fn backoff(attempt: u32, retry_after: Option<u64>) -> u64 {
    retry_after.unwrap_or(2 * 3u64.pow(attempt.saturating_sub(1))).clamp(1, 20)
}

/// Starts request `id` in the background; its news come through [`take`].
pub fn start(id: u64, req: HttpRequest, opts: Options) -> Result<(), String> {
    ACTIVE.fetch_add(1, Ordering::SeqCst);
    #[cfg(not(target_arch = "wasm32"))]
    {
        let spawned = std::thread::Builder::new().name(format!("rapidr-ai-{id}")).spawn(move || {
            native::send(id, &req, opts, &mut |what| push(id, what));
            finished(id);
        });
        if let Err(e) = spawned {
            finished(id);
            return Err(format!("can't start the request: {e}"));
        }
        Ok(())
    }
    #[cfg(target_arch = "wasm32")]
    {
        web::start(id, req, opts)
    }
}

/// (the desktop, `rapidr ai`) A request run to its end here: its status
/// and its whole body.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_blocking(req: &HttpRequest, opts: Options, mut on_data: impl FnMut(&[u8])) -> Result<(u16, Option<u64>), String> {
    let mut status = (0, None);
    let mut result = Ok(());
    native::send(u64::MAX, req, opts, &mut |what| match what {
        Incoming::Status { status: s, retry_after } => status = (s, retry_after),
        Incoming::Data(d) => on_data(&d),
        Incoming::Done(r) => result = r,
    });
    result.map(|()| status)
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::io::Read;
    use std::time::Duration;

    use super::{backoff, cancelled, passing, Incoming, Options, NET_RETRY_WAIT};
    use crate::wire::HttpRequest;

    /// (the tests' walks) `RAPIDR_AI_TEST_DNS_FAIL_ONCE` set: the first name
    /// the process looks up isn't found, as a network's DNS hiccup — what
    /// the retry is for, seen without unplugging anything.
    static DNS_FAILED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

    fn agent(opts: Options) -> ureq::Agent {
        // (no redirects: a provider's API never needs one, and a redirect
        // would take the key's header — x-api-key, which ureq keeps across
        // hosts — to wherever it points, SEC-27; a 3xx is an answer, said
        // in plain words, wire::explain_error)
        let mut builder = ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(opts.connect_timeout)).timeout_read(Duration::from_secs(opts.read_timeout)).redirects(0).user_agent(concat!("RapidR/", env!("CARGO_PKG_VERSION")));
        if rapidr_testmode::var_os("RAPIDR_AI_TEST_DNS_FAIL_ONCE").is_some() && !DNS_FAILED.load(std::sync::atomic::Ordering::SeqCst) {
            builder = builder.resolver(|netloc: &str| {
                if !DNS_FAILED.swap(true, std::sync::atomic::Ordering::SeqCst) {
                    return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "the name wasn't found (a test's DNS failure)"));
                }
                std::net::ToSocketAddrs::to_socket_addrs(netloc).map(Iterator::collect)
            });
        }
        match ureq::native_tls::TlsConnector::new() {
            Ok(tls) => builder.tls_connector(std::sync::Arc::new(tls)).build(),
            Err(_) => builder.build(),
        }
    }

    /// What ureq said, without the URL it repeats.
    fn words(t: &ureq::Transport) -> String {
        let kind = match t.kind() {
            ureq::ErrorKind::Dns => "the name wasn't found (DNS)".to_string(),
            ureq::ErrorKind::ConnectionFailed => "the connection failed".to_string(),
            ureq::ErrorKind::TooManyRedirects => "too many redirects".to_string(),
            ureq::ErrorKind::InvalidUrl | ureq::ErrorKind::UnknownScheme => "the address isn't valid".to_string(),
            _ => String::new(),
        };
        let detail = t.message().map(str::to_string).or_else(|| t.source_error_text()).unwrap_or_default();
        match (kind.is_empty(), detail.is_empty()) {
            (false, false) => format!("{kind} ({detail})"),
            (false, true) => kind,
            (true, false) => detail,
            (true, true) => "no answer".into(),
        }
    }

    /// Whether the request never reached the provider, for a passing
    /// reason worth one more try: the name not found, no connection (none
    /// in time either), a connection closed before any answer. Never an
    /// answer that didn't come in time (the provider has the question:
    /// asked again, it would be paid twice).
    fn unreached(t: &ureq::Transport) -> bool {
        match t.kind() {
            ureq::ErrorKind::Dns | ureq::ErrorKind::ConnectionFailed => true,
            ureq::ErrorKind::Io => !timed_out(t),
            _ => false,
        }
    }

    /// Waits `seconds`, or less when the request is stopped meanwhile;
    /// whether it is still wanted.
    fn wait_unless_stopped(id: u64, seconds: u64) -> bool {
        for _ in 0..seconds * 10 {
            if cancelled(id) {
                return false;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        !cancelled(id)
    }

    /// Whether the transport gave up waiting (to connect, or for the answer).
    fn timed_out(t: &ureq::Transport) -> bool {
        let io = std::error::Error::source(t).and_then(|e| e.downcast_ref::<std::io::Error>()).is_some_and(|e| matches!(e.kind(), std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock));
        io || t.message().is_some_and(|m| m.contains("timed out"))
    }

    trait SourceText {
        fn source_error_text(&self) -> Option<String>;
    }

    impl SourceText for ureq::Transport {
        fn source_error_text(&self) -> Option<String> {
            std::error::Error::source(self).map(|e| e.to_string())
        }
    }

    pub fn send(id: u64, req: &HttpRequest, opts: Options, on: &mut dyn FnMut(Incoming)) {
        let mut attempt = 0;
        // (one more try for a request that never reached the provider)
        let mut unreached_tries = 0;
        loop {
            attempt += 1;
            let agent = agent(opts);
            let mut r = agent.request(req.method, &req.url);
            for (k, v) in &req.headers {
                r = r.set(k, v);
            }
            let res = match &req.body {
                Some(b) => r.send_string(b),
                None => r.call(),
            };
            match res {
                Ok(resp) => {
                    on(Incoming::Status { status: resp.status(), retry_after: None });
                    let mut reader = resp.into_reader();
                    let mut buf = vec![0u8; 16 * 1024];
                    loop {
                        if cancelled(id) {
                            on(Incoming::Done(Err("stopped".into())));
                            return;
                        }
                        match reader.read(&mut buf) {
                            Ok(0) => break,
                            Ok(n) => on(Incoming::Data(buf[..n].to_vec())),
                            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                            Err(e) => {
                                let what = if matches!(e.kind(), std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock) { format!("the answer stopped coming for {} seconds", opts.read_timeout) } else { e.to_string() };
                                on(Incoming::Done(Err(what)));
                                return;
                            }
                        }
                    }
                    on(Incoming::Done(Ok(())));
                    return;
                }
                Err(ureq::Error::Status(code, resp)) => {
                    let retry_after = resp.header("retry-after").and_then(|v| v.trim().parse::<u64>().ok());
                    let body = resp.into_string().unwrap_or_default();
                    if attempt <= opts.retries && passing(code, &body) && !cancelled(id) {
                        std::thread::sleep(Duration::from_secs(backoff(attempt, retry_after)));
                        if !cancelled(id) {
                            continue;
                        }
                    }
                    on(Incoming::Status { status: code, retry_after });
                    on(Incoming::Data(body.into_bytes()));
                    on(Incoming::Done(Ok(())));
                    return;
                }
                Err(ureq::Error::Transport(t)) => {
                    if unreached(&t) && unreached_tries == 0 && !cancelled(id) {
                        unreached_tries += 1;
                        if wait_unless_stopped(id, NET_RETRY_WAIT) {
                            continue;
                        }
                        on(Incoming::Done(Err("stopped".into())));
                        return;
                    }
                    let what = if !timed_out(&t) {
                        words(&t)
                    } else if t.kind() == ureq::ErrorKind::ConnectionFailed {
                        format!("no connection in {} seconds", opts.connect_timeout)
                    } else {
                        format!("no answer came in {} seconds", opts.read_timeout)
                    };
                    on(Incoming::Done(Err(what)));
                    return;
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    use std::cell::RefCell;
    use std::collections::HashMap;

    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{AbortController, Headers, RequestInit, RequestMode, Response};

    use super::{backoff, cancelled, finished, notify, passing, push, Incoming, Options, NET_RETRY_WAIT};
    use crate::wire::HttpRequest;

    thread_local! {
        static CONTROLLERS: RefCell<HashMap<u64, AbortController>> = RefCell::new(HashMap::new());
    }

    pub fn abort(id: u64) {
        if let Some(c) = CONTROLLERS.with(|c| c.borrow_mut().remove(&id)) {
            c.abort();
        }
    }

    fn js_words(e: &JsValue) -> String {
        let m = js_sys::Reflect::get(e, &"message".into()).ok().and_then(|m| m.as_string()).or_else(|| e.as_string()).unwrap_or_else(|| "no answer".into());
        // (a browser says only "Failed to fetch": the network, CORS, or the
        // page's Content-Security-Policy)
        if m.contains("Failed to fetch") || m.contains("NetworkError") || m.contains("Load failed") {
            "the browser couldn't connect (no network, or the server or this page's security policy doesn't allow it)".into()
        } else {
            m
        }
    }

    async fn sleep(seconds: u64) {
        let p = js_sys::Promise::new(&mut |resolve, _| {
            if let Some(w) = web_sys::window() {
                let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, (seconds * 1000) as i32);
            }
        });
        let _ = JsFuture::from(p).await;
    }

    /// `p`'s value, or `Err(())` when `seconds` went by first (a browser's
    /// fetch has no timeout of its own: a server that never answers would
    /// keep the request — and the button that waits for it — forever).
    async fn within(p: js_sys::Promise, seconds: u64) -> Result<Result<JsValue, JsValue>, ()> {
        let marker = js_sys::Object::new();
        let m = marker.clone();
        let mut handle = None;
        let timer = js_sys::Promise::new(&mut |resolve, _| {
            if let Some(w) = web_sys::window() {
                handle = w.set_timeout_with_callback_and_timeout_and_arguments_1(&resolve, (seconds * 1000) as i32, &m).ok();
            }
        });
        let race = js_sys::Promise::race(&js_sys::Array::of2(&p, &timer));
        let r = JsFuture::from(race).await;
        // (the timer of a race won goes: a stream's many pieces leave none behind)
        if let (Some(h), Some(w)) = (handle, web_sys::window()) {
            w.clear_timeout_with_handle(h);
        }
        match r {
            Ok(v) if v == JsValue::from(marker) => Err(()),
            r => Ok(r),
        }
    }

    pub fn start(id: u64, req: HttpRequest, opts: Options) -> Result<(), String> {
        let ctrl = AbortController::new().map_err(|e| js_words(&e))?;
        CONTROLLERS.with(|c| c.borrow_mut().insert(id, ctrl.clone()));
        wasm_bindgen_futures::spawn_local(async move {
            let result = fetch(id, &req, opts, &ctrl).await;
            CONTROLLERS.with(|c| c.borrow_mut().remove(&id));
            let result = if cancelled(id) { Err("stopped".to_string()) } else { result };
            push(id, Incoming::Done(result));
            finished(id);
            notify();
        });
        Ok(())
    }

    async fn fetch(id: u64, req: &HttpRequest, opts: Options, ctrl: &AbortController) -> Result<(), String> {
        let window = web_sys::window().ok_or("no browser window")?;
        let mut attempt = 0;
        // (one more try for a request that never reached the provider: the
        // browser says only that it couldn't fetch)
        let mut unreached_tries = 0;
        loop {
            attempt += 1;
            let init = RequestInit::new();
            init.set_method(req.method);
            init.set_mode(RequestMode::Cors);
            let headers = Headers::new().map_err(|e| js_words(&e))?;
            for (k, v) in &req.headers {
                headers.set(k, v).map_err(|e| js_words(&e))?;
            }
            init.set_headers(&headers);
            if let Some(b) = &req.body {
                init.set_body(&JsValue::from_str(b));
            }
            init.set_signal(Some(&ctrl.signal()));
            let resp = match within(window.fetch_with_str_and_init(&req.url, &init), opts.connect_timeout + opts.read_timeout).await {
                Ok(Err(e)) if unreached_tries == 0 && !cancelled(id) => {
                    unreached_tries += 1;
                    sleep(NET_RETRY_WAIT).await;
                    if cancelled(id) {
                        return Err(js_words(&e));
                    }
                    continue;
                }
                Ok(r) => r.map_err(|e| js_words(&e))?,
                Err(()) => {
                    ctrl.abort();
                    return Err(format!("no answer came in {} seconds", opts.connect_timeout + opts.read_timeout));
                }
            };
            let resp: Response = resp.dyn_into().map_err(|_| "not an answer".to_string())?;
            let status = resp.status();
            let retry_after = resp.headers().get("retry-after").ok().flatten().and_then(|v| v.trim().parse::<u64>().ok());
            if status >= 400 {
                let text = match resp.text() {
                    Ok(p) => within(p, opts.read_timeout).await.ok().and_then(Result::ok).and_then(|t| t.as_string()).unwrap_or_default(),
                    Err(_) => String::new(),
                };
                if attempt <= opts.retries && passing(status, &text) && !cancelled(id) {
                    sleep(backoff(attempt, retry_after)).await;
                    if !cancelled(id) {
                        continue;
                    }
                }
                push(id, Incoming::Status { status, retry_after });
                push(id, Incoming::Data(text.into_bytes()));
                return Ok(());
            }
            push(id, Incoming::Status { status, retry_after });
            notify();
            let Some(body) = resp.body() else { return Ok(()) };
            let reader: web_sys::ReadableStreamDefaultReader = body.get_reader().unchecked_into();
            loop {
                let chunk = match within(reader.read(), opts.read_timeout).await {
                    Ok(r) => r.map_err(|e| js_words(&e))?,
                    Err(()) => {
                        ctrl.abort();
                        return Err(format!("the answer stopped coming for {} seconds", opts.read_timeout));
                    }
                };
                if js_sys::Reflect::get(&chunk, &"done".into()).ok().and_then(|d| d.as_bool()).unwrap_or(true) {
                    return Ok(());
                }
                if let Ok(v) = js_sys::Reflect::get(&chunk, &"value".into()) {
                    push(id, Incoming::Data(js_sys::Uint8Array::new(&v).to_vec()));
                    notify();
                }
            }
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;

    /// A one-shot HTTP server on 127.0.0.1: answers each connection with
    /// the next of `answers` (status line + headers + body, written in
    /// pieces), and keeps the requests it read.
    pub fn serve(answers: Vec<String>) -> (String, std::thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let handle = std::thread::spawn(move || {
            let mut seen = Vec::new();
            for answer in answers {
                let (mut s, _) = listener.accept().unwrap();
                let mut r = BufReader::new(s.try_clone().unwrap());
                let mut head = String::new();
                let mut len = 0usize;
                loop {
                    let mut line = String::new();
                    r.read_line(&mut line).unwrap();
                    if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                        len = v.trim().parse().unwrap();
                    }
                    head.push_str(&line);
                    if line == "\r\n" || line.is_empty() {
                        break;
                    }
                }
                let mut body = vec![0u8; len];
                std::io::Read::read_exact(&mut r, &mut body).unwrap();
                seen.push(head + &String::from_utf8_lossy(&body));
                for piece in answer.as_bytes().chunks(13) {
                    s.write_all(piece).unwrap();
                    s.flush().unwrap();
                }
            }
            seen
        });
        (url, handle)
    }

    fn ok(body: &str, content_type: &str) -> String {
        format!("HTTP/1.1 200 OK\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len())
    }

    #[test]
    fn a_request_in_the_background() {
        let (url, server) = serve(vec![ok("data: {\"x\":1}\n\n", "text/event-stream")]);
        let req = HttpRequest { method: "POST", url: format!("{url}/v1/chat/completions"), headers: vec![("authorization".into(), "Bearer k".into())], body: Some("{\"a\":1}".into()) };
        start(7, req, Options::default()).unwrap();
        let mut got = Vec::new();
        for _ in 0..500 {
            got.extend(take().into_iter().filter(|(id, _)| *id == 7).map(|(_, w)| w));
            if matches!(got.last(), Some(Incoming::Done(_))) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(got.first(), Some(&Incoming::Status { status: 200, retry_after: None }));
        let body: Vec<u8> = got.iter().filter_map(|w| if let Incoming::Data(d) = w { Some(d.clone()) } else { None }).flatten().collect();
        assert_eq!(body, b"data: {\"x\":1}\n\n");
        assert_eq!(got.last(), Some(&Incoming::Done(Ok(()))));
        let seen = server.join().unwrap();
        assert!(seen[0].starts_with("POST /v1/chat/completions") && seen[0].to_ascii_lowercase().contains("authorization: bearer k") && seen[0].ends_with("{\"a\":1}"), "{}", seen[0]);
    }

    #[test]
    fn a_passing_refusal_is_tried_again() {
        let busy = "HTTP/1.1 529 Overloaded\r\nretry-after: 1\r\ncontent-length: 11\r\nconnection: close\r\n\r\noverloaded!".to_string();
        let (url, server) = serve(vec![busy, ok("{}", "application/json")]);
        let req = HttpRequest { method: "GET", url: format!("{url}/v1/models"), headers: vec![], body: None };
        let mut body = Vec::new();
        let (status, _) = run_blocking(&req, Options::default(), |d| body.extend_from_slice(d)).unwrap();
        assert_eq!((status, body.as_slice()), (200, b"{}".as_slice()));
        assert_eq!(server.join().unwrap().len(), 2);
        // (no credit: passed on at once)
        let poor = "HTTP/1.1 429 Too Many\r\ncontent-length: 34\r\nconnection: close\r\n\r\n{\"error\":\"insufficient_quota\"}    ".to_string();
        let (url, server) = serve(vec![poor]);
        let req = HttpRequest { method: "GET", url: format!("{url}/x"), headers: vec![], body: None };
        assert_eq!(run_blocking(&req, Options::default(), |_| {}).unwrap().0, 429);
        server.join().unwrap();
        // (nobody there)
        let gone = HttpRequest { method: "GET", url: "http://127.0.0.1:9/x".into(), headers: vec![], body: None };
        assert!(run_blocking(&gone, Options { retries: 0, connect_timeout: 3, ..Options::default() }, |_| {}).is_err());
        assert_eq!(backoff(1, None), 2);
        assert_eq!(backoff(2, None), 6);
        assert_eq!(backoff(1, Some(90)), 20);
    }

    /// Robert's evening with DeepSeek (2026-10-20): "Couldn't reach
    /// api.deepseek.com: the name wasn't found (DNS)" on one question, and
    /// he had to send it again. A request that never reached the provider
    /// is tried once more, 2 seconds later; a refusal never is.
    #[test]
    fn a_request_that_never_reached_the_provider_is_tried_once_more() {
        // (the connection closed before any answer: the second try answers)
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (first, _) = listener.accept().unwrap();
            drop(first);
            let (mut s, _) = listener.accept().unwrap();
            let mut r = BufReader::new(s.try_clone().unwrap());
            loop {
                let mut line = String::new();
                r.read_line(&mut line).unwrap();
                if line == "\r\n" || line.is_empty() {
                    break;
                }
            }
            s.write_all(ok("{}", "application/json").as_bytes()).unwrap();
        });
        let started = std::time::Instant::now();
        let req = HttpRequest { method: "GET", url: format!("{url}/v1/models"), headers: vec![], body: None };
        let mut body = Vec::new();
        let (status, _) = run_blocking(&req, Options { retries: 0, ..Options::default() }, |d| body.extend_from_slice(d)).unwrap();
        assert_eq!((status, body.as_slice()), (200, b"{}".as_slice()));
        assert!(started.elapsed() >= std::time::Duration::from_secs(NET_RETRY_WAIT), "it waited before trying again");
        server.join().unwrap();
        // (a name that is never found: tried twice, then the error in words)
        let started = std::time::Instant::now();
        let nowhere = HttpRequest { method: "GET", url: "http://no-such-host.invalid/v1/models".into(), headers: vec![], body: None };
        let e = run_blocking(&nowhere, Options { retries: 0, connect_timeout: 3, ..Options::default() }, |_| {}).unwrap_err();
        assert!(e.contains("DNS") && started.elapsed() >= std::time::Duration::from_secs(NET_RETRY_WAIT), "{e}");
        // (a refusal: passed on at once, never tried again)
        let refused = "HTTP/1.1 401 Unauthorized\r\ncontent-length: 2\r\nconnection: close\r\n\r\n{}".to_string();
        let (url, server) = serve(vec![refused]);
        let req = HttpRequest { method: "GET", url: format!("{url}/x"), headers: vec![], body: None };
        assert_eq!(run_blocking(&req, Options::default(), |_| {}).unwrap().0, 401);
        assert_eq!(server.join().unwrap().len(), 1);
    }
}
