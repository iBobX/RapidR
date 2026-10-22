//! Network backends — RSocket, RServerSocket, RHTTP.

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use crate::object::{rp_comp_set, rp_fire_event_1, rp_fire_event_2};
use crate::value::{v_int, v_null, v_str, Value};

// ---------------------------------------------------------------------------
// RSocket (TCP client)
// ---------------------------------------------------------------------------

struct SocketState {
    stream: TcpStream,
    reader: BufReader<TcpStream>,
}

thread_local! {
    static SOCKETS: RefCell<HashMap<String, SocketState>> = RefCell::new(HashMap::new());
}

pub fn socket_method(name: &str, method: &str, args: &[Value]) -> Value {
    let name_lower = name.to_lowercase();
    if let Some(v) = qsocket::method(&name_lower, method, args) {
        return v;
    }
    match method {
        "connect" => socket_connect(&name_lower),
        "close" | "disconnect" => socket_close(&name_lower),
        "write" => socket_write(&name_lower, args, false),
        "writeline" => socket_write(&name_lower, args, true),
        "read" => socket_read(&name_lower, args),
        "readline" => socket_readline(&name_lower),
        _ => {
            eprintln!("[WARN] RSocket.{}() not implemented", method);
            v_null()
        }
    }
}

fn socket_connect(name: &str) -> Value {
    let host = crate::object::rp_comp_get(name, "host").to_string_val();
    let port = crate::object::rp_comp_get(name, "port").to_i64() as u16;
    let timeout_ms = crate::object::rp_comp_get(name, "timeout").to_i64().max(1000) as u64;

    let addr = format!("{}:{}", host, port);
    match TcpStream::connect_timeout(
        &addr.parse().unwrap_or_else(|_| {
            format!("127.0.0.1:{}", port).parse().unwrap()
        }),
        Duration::from_millis(timeout_ms),
    ) {
        Ok(stream) => {
            let _ = stream.set_read_timeout(Some(Duration::from_millis(timeout_ms)));
            let _ = stream.set_write_timeout(Some(Duration::from_millis(timeout_ms)));
            let reader = BufReader::new(stream.try_clone().unwrap());
            SOCKETS.with(|s| {
                s.borrow_mut().insert(
                    name.to_string(),
                    SocketState { stream, reader },
                );
            });
            rp_comp_set(name, "connected", v_int(1));
            v_int(1)
        }
        Err(e) => {
            eprintln!("[Socket] Connect error: {}", e);
            rp_comp_set(name, "connected", v_int(0));
            v_int(0)
        }
    }
}

fn socket_close(name: &str) -> Value {
    SOCKETS.with(|s| {
        if let Some(state) = s.borrow_mut().remove(name) {
            let _ = state.stream.shutdown(std::net::Shutdown::Both);
        }
    });
    rp_comp_set(name, "connected", v_int(0));
    v_null()
}

fn socket_write(name: &str, args: &[Value], newline: bool) -> Value {
    let data = args.first().map(|v| v.to_string_val()).unwrap_or_default();
    SOCKETS.with(|s| {
        let mut sockets = s.borrow_mut();
        if let Some(state) = sockets.get_mut(name) {
            let result = if newline {
                writeln!(state.stream, "{}", data)
            } else {
                write!(state.stream, "{}", data)
            };
            let _ = state.stream.flush();
            match result {
                Ok(()) => v_int(1),
                Err(_) => v_int(0),
            }
        } else {
            v_int(0)
        }
    })
}

fn socket_read(name: &str, args: &[Value]) -> Value {
    let max_bytes = args.first().map(|v| v.to_i64()).unwrap_or(4096) as usize;
    SOCKETS.with(|s| {
        let mut sockets = s.borrow_mut();
        if let Some(state) = sockets.get_mut(name) {
            let mut buf = vec![0u8; max_bytes];
            match state.stream.read(&mut buf) {
                Ok(n) => v_str(&String::from_utf8_lossy(&buf[..n])),
                Err(_) => v_str(""),
            }
        } else {
            v_str("")
        }
    })
}

fn socket_readline(name: &str) -> Value {
    SOCKETS.with(|s| {
        let mut sockets = s.borrow_mut();
        if let Some(state) = sockets.get_mut(name) {
            let mut line = String::new();
            match state.reader.read_line(&mut line) {
                Ok(0) => v_str(""),
                Ok(_) => v_str(line.trim_end_matches('\n').trim_end_matches('\r')),
                Err(_) => v_str(""),
            }
        } else {
            v_str("")
        }
    })
}

// ---------------------------------------------------------------------------
// RapidQ's QSOCKET: numbered sockets, as its manual (Appendix A) has them
// ---------------------------------------------------------------------------

/// RapidQ's QSOCKET API — `Sock% = S.Connect(Server$, Port%)`, `S.Open(Port%)`
/// for a server, `S.Accept(Sock%)`, `S.ReadLine(Sock%)`, `S.Write(Sock%,
/// Msg$, n)`, `S.IsServerReady(Sock%)`, … — told from RapidR's own RSOCKET
/// methods by their arguments (RapidR's take none, or only the data). The
/// calls block as RapidQ's do; the `…Ready` checks don't.
mod qsocket {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream, ToSocketAddrs};

    use crate::object::rp_comp_set;
    use crate::value::{v_int, v_str, Value};

    enum Sock {
        Stream { s: TcpStream, buf: Vec<u8> },
        Server { l: TcpListener, pending: Vec<TcpStream> },
    }

    thread_local! {
        static SOCKS: RefCell<(i64, HashMap<i64, Sock>)> = RefCell::new((3, HashMap::new()));
    }

    fn add(sock: Sock) -> i64 {
        SOCKS.with(|s| {
            let mut s = s.borrow_mut();
            let n = s.0;
            s.0 += 1;
            s.1.insert(n, sock);
            n
        })
    }

    fn with<R>(n: i64, f: impl FnOnce(&mut Sock) -> R) -> Option<R> {
        SOCKS.with(|s| s.borrow_mut().1.get_mut(&n).map(f))
    }

    /// Bytes waiting: what's buffered, or what the peer sent (not waiting).
    fn ready(sock: &mut Sock) -> bool {
        match sock {
            Sock::Stream { s, buf } => {
                if !buf.is_empty() {
                    return true;
                }
                let _ = s.set_nonblocking(true);
                let mut one = [0u8; 1];
                let r = s.peek(&mut one);
                let _ = s.set_nonblocking(false);
                matches!(r, Ok(n) if n > 0)
            }
            Sock::Server { l, pending } => {
                if pending.is_empty() {
                    let _ = l.set_nonblocking(true);
                    if let Ok((c, _)) = l.accept() {
                        let _ = c.set_nonblocking(false);
                        pending.push(c);
                    }
                    let _ = l.set_nonblocking(false);
                }
                !pending.is_empty()
            }
        }
    }

    /// Fills the buffer with what arrives next (blocking); false at the end.
    fn fill(s: &mut TcpStream, buf: &mut Vec<u8>) -> bool {
        let mut chunk = [0u8; 4096];
        match s.read(&mut chunk) {
            Ok(n) if n > 0 => {
                buf.extend_from_slice(&chunk[..n]);
                true
            }
            _ => false,
        }
    }

    fn text(bytes: &[u8]) -> Value {
        v_str(&bytes.iter().map(|&b| char::from(b)).collect::<String>())
    }

    fn bytes(v: &Value) -> Vec<u8> {
        v.to_string_val().chars().map(|c| c as u32 as u8).collect()
    }

    pub fn method(name: &str, method: &str, args: &[Value]) -> Option<Value> {
        let n = |i: usize| args.get(i).map_or(0, Value::to_i64);
        let flag = |b: bool| v_int(b as i64);
        let transferred = |k: usize| rp_comp_set(name, "transferred", v_int(k as i64));
        Some(match (method, args.len()) {
            ("connect", 2) => {
                let addr = format!("{}:{}", args[0].to_string_val(), n(1));
                let stream = addr.to_socket_addrs().ok().and_then(|mut a| a.find_map(|a| TcpStream::connect(a).ok()));
                let id = stream.map_or(-1, |s| add(Sock::Stream { s, buf: Vec::new() }));
                rp_comp_set(name, "mysocket", v_int(id));
                v_int(id)
            }
            // (RapidR's: the address listened on — Open's second argument,
            // else BindAddress; every network interface, "0.0.0.0", as
            // RapidQ's, unless one is given: "127.0.0.1" this computer only)
            ("open", 1 | 2) => {
                let given = match args.get(1) {
                    Some(a) => a.to_string_val(),
                    None => crate::object::rp_comp_get(name, "bindaddress").to_string_val(),
                };
                let host = if given.trim().is_empty() { "0.0.0.0".to_string() } else { given.trim().to_string() };
                let id = TcpListener::bind((host.as_str(), n(0).clamp(0, 65535) as u16)).map_or(-1, |l| add(Sock::Server { l, pending: Vec::new() }));
                rp_comp_set(name, "mysocket", v_int(id));
                v_int(id)
            }
            ("connectionready", 1) => flag(with(n(0), ready).unwrap_or(false)),
            ("isserverready", 1) => flag(with(n(0), ready).unwrap_or(false)),
            ("isclientready", 2) => flag(with(n(1), ready).unwrap_or(false)),
            ("accept", 1) => {
                let client = with(n(0), |s| match s {
                    Sock::Server { l, pending } => {
                        if pending.is_empty() {
                            l.accept().ok().map(|(c, _)| c)
                        } else {
                            Some(pending.remove(0))
                        }
                    }
                    _ => None,
                })
                .flatten();
                v_int(client.map_or(-1, |s| add(Sock::Stream { s, buf: Vec::new() })))
            }
            ("close", 1) => {
                SOCKS.with(|s| s.borrow_mut().1.remove(&n(0)));
                Value::Null
            }
            ("read" | "peek", 2) => {
                let want = n(1).clamp(0, 1 << 24) as usize;
                let peek = method == "peek";
                let got = with(n(0), |s| match s {
                    Sock::Stream { s, buf } => {
                        if buf.is_empty() {
                            fill(s, buf);
                        }
                        let k = want.min(buf.len());
                        if peek { buf[..k].to_vec() } else { buf.drain(..k).collect() }
                    }
                    _ => Vec::new(),
                })
                .unwrap_or_default();
                transferred(got.len());
                text(&got)
            }
            ("readbyte", 1) => {
                let b = with(n(0), |s| match s {
                    Sock::Stream { s, buf } => {
                        if buf.is_empty() {
                            fill(s, buf);
                        }
                        (!buf.is_empty()).then(|| buf.remove(0))
                    }
                    _ => None,
                })
                .flatten();
                v_int(b.map_or(0, i64::from))
            }
            ("readline", 1) => {
                let line = with(n(0), |s| match s {
                    Sock::Stream { s, buf } => loop {
                        if let Some(i) = buf.iter().position(|&b| b == b'\n') {
                            let mut l: Vec<u8> = buf.drain(..=i).collect();
                            l.pop();
                            if l.last() == Some(&b'\r') {
                                l.pop();
                            }
                            break l;
                        }
                        if !fill(s, buf) {
                            break std::mem::take(buf);
                        }
                    },
                    _ => Vec::new(),
                })
                .unwrap_or_default();
                transferred(line.len());
                text(&line)
            }
            ("write", 3) | ("writeline", 2) | ("writebyte", 2) => {
                let mut data = match method {
                    "writebyte" => vec![n(1) as u8],
                    _ => bytes(&args[1]),
                };
                if method == "write" {
                    data.truncate(n(2).max(0) as usize);
                }
                if method == "writeline" {
                    data.extend_from_slice(b"\r\n");
                }
                let sent = with(n(0), |s| match s {
                    Sock::Stream { s, .. } => s.write_all(&data).map(|_| data.len()).unwrap_or(0),
                    _ => 0,
                })
                .unwrap_or(0);
                transferred(sent);
                v_int(sent as i64)
            }
            ("getpeername", 1) => v_str(&with(n(0), |s| match s {
                Sock::Stream { s, .. } => s.peer_addr().map(|a| a.ip().to_string()).unwrap_or_default(),
                _ => String::new(),
            })
            .unwrap_or_default()),
            ("gethostname", 0) => v_str(&host_name()),
            ("gethostip", 0) => v_str(&host_ip()),
            _ => return None,
        })
    }

    fn host_name() -> String {
        std::process::Command::new("hostname").output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default()
    }

    /// The address this machine reaches others from (no packet is sent).
    fn host_ip() -> String {
        std::net::UdpSocket::bind("0.0.0.0:0")
            .and_then(|u| u.connect("8.8.8.8:80").map(|_| u))
            .and_then(|u| u.local_addr())
            .map(|a| a.ip().to_string())
            .unwrap_or_else(|_| "127.0.0.1".to_string())
    }
}

// ---------------------------------------------------------------------------
// RServerSocket (TCP server) — channel-based event polling
// ---------------------------------------------------------------------------

/// Events generated by the server background threads
enum ServerEvent {
    ClientConnect { addr: String },
    DataReceived { addr: String, data: String },
    ClientDisconnect { addr: String },
}

/// A connected client: its id, the stream the program writes to, its
/// address.
type Clients = Arc<Mutex<Vec<(usize, TcpStream, String)>>>;

struct ServerState {
    /// Shared client list that background threads can write to and broadcast to
    clients: Clients,
    /// Receiver for events from background threads
    events: mpsc::Receiver<ServerEvent>,
    /// Flag to signal shutdown
    running: Arc<Mutex<bool>>,
}

thread_local! {
    static SERVERS: RefCell<HashMap<String, ServerState>> = RefCell::new(HashMap::new());
}

pub fn server_socket_method(name: &str, method: &str, args: &[Value]) -> Value {
    let name_lower = name.to_lowercase();
    match method {
        "start" | "listen" => server_start(&name_lower),
        "stop" => server_stop(&name_lower),
        "broadcast" => server_broadcast(&name_lower, args),
        "accept" => server_accept(&name_lower),
        "poll" => server_poll(&name_lower),
        "sendto" => server_sendto(&name_lower, args, true),
        "send" => server_sendto(&name_lower, args, false),
        "disconnect" => server_disconnect(&name_lower, args),
        _ => {
            eprintln!("[WARN] RServerSocket.{}() not implemented", method);
            v_null()
        }
    }
}

/// How long a write to a client may wait for it to read (a client that
/// never reads doesn't hold the program forever).
const WRITE_WAIT: Duration = Duration::from_secs(10);

fn server_start(name: &str) -> Value {
    let host = crate::object::rp_comp_get(name, "host").to_string_val();
    let host = if host.trim().is_empty() { "0.0.0.0".to_string() } else { host.trim().to_string() };
    let port = crate::object::rp_comp_get(name, "port").to_i64().clamp(0, 65535) as u16;
    let framing = crate::netframe::Framing::by_name(&crate::object::rp_comp_get(name, "mode").to_string_val(), crate::netframe::Limits::default());

    // (one already listening under this name: stopped first)
    server_stop(name);
    let listener = match TcpListener::bind((host.as_str(), port)) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[ServerSocket] Bind error: {}", e);
            return v_int(0);
        }
    };
    // (Port 0: the port the system picked)
    if let Ok(a) = listener.local_addr() {
        rp_comp_set(name, "port", v_int(i64::from(a.port())));
    }
    let _ = listener.set_nonblocking(true);

    let (tx, rx) = mpsc::channel::<ServerEvent>();
    let running = Arc::new(Mutex::new(true));
    let clients: Clients = Arc::new(Mutex::new(Vec::new()));
    let next_id = Arc::new(Mutex::new(1usize));

    // Spawn accept thread
    let accept_running = Arc::clone(&running);
    let accept_clients = Arc::clone(&clients);
    let accept_tx = tx.clone();
    let accept_next_id = Arc::clone(&next_id);

    std::thread::spawn(move || {
        while *accept_running.lock().unwrap() {
            match listener.accept() {
                Ok((stream, addr)) => {
                    let addr_str = addr.to_string();
                    let _ = stream.set_nonblocking(false);
                    let _ = stream.set_read_timeout(Some(Duration::from_millis(100)));
                    let _ = stream.set_write_timeout(Some(WRITE_WAIT));

                    let mut id_lock = accept_next_id.lock().unwrap();
                    let client_id = *id_lock;
                    *id_lock += 1;
                    drop(id_lock);

                    // Clone stream for reading thread
                    let read_stream = match stream.try_clone() {
                        Ok(s) => s,
                        Err(_) => continue,
                    };

                    // Store in shared clients list
                    accept_clients.lock().unwrap().push((client_id, stream, addr_str.clone()));

                    let _ = accept_tx.send(ServerEvent::ClientConnect { addr: addr_str.clone() });

                    // Spawn per-client reader thread
                    let reader_tx = accept_tx.clone();
                    let reader_running = Arc::clone(&accept_running);
                    let reader_clients = Arc::clone(&accept_clients);
                    std::thread::spawn(move || {
                        read_client(read_stream, framing, &reader_running, |data| {
                            let _ = reader_tx.send(ServerEvent::DataReceived { addr: addr_str.clone(), data });
                        });
                        reader_clients.lock().unwrap().retain(|(id, _, _)| *id != client_id);
                        let _ = reader_tx.send(ServerEvent::ClientDisconnect { addr: addr_str.clone() });
                    });
                }
                Err(_) => {
                    std::thread::sleep(Duration::from_millis(20));
                }
            }
        }
    });

    SERVERS.with(|s| {
        s.borrow_mut().insert(name.to_string(), ServerState { clients, events: rx, running });
    });
    v_int(1)
}

/// A client's connection read until it closes (or the server stops): what
/// arrives cut as `framing` says (crate::netframe — nothing that arrived is
/// lost however slowly it comes), each piece given to `deliver` as the
/// program gets it. An HTTP request (Mode "http") arrives whole: its head,
/// then its body by its Content-Length or its chunks; a client waiting for
/// `100 Continue` is told to go on; one the limits refuse is answered with
/// the status and closed.
fn read_client(mut stream: TcpStream, framing: crate::netframe::Framing, running: &Mutex<bool>, mut deliver: impl FnMut(String)) {
    use crate::netframe::{Framer, Piece};
    let mut framer = Framer::new(framing);
    let mut buf = vec![0u8; 16 * 1024];
    loop {
        if !*running.lock().unwrap() {
            return;
        }
        let pieces = match stream.read(&mut buf) {
            Ok(0) => {
                for p in framer.finish() {
                    if let Piece::Text(t) = p {
                        deliver(t);
                    }
                }
                return;
            }
            Ok(n) => framer.feed(&buf[..n]),
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut || e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return,
        };
        for p in pieces {
            match p {
                Piece::Text(t) => deliver(t),
                Piece::Request(r) => deliver(r.whole_text()),
                Piece::Continue(_) => {
                    let _ = stream.write_all(b"HTTP/1.1 100 Continue\r\n\r\n");
                }
                Piece::Refuse(status, why) => {
                    let body = format!("{why}\n");
                    let head = crate::netframe::response_head(status, &[("Content-Type".into(), "text/plain; charset=utf-8".into())], Some(body.len()), false, true);
                    let _ = stream.write_all(format!("{head}{body}").as_bytes());
                    let _ = stream.shutdown(std::net::Shutdown::Both);
                    return;
                }
            }
        }
    }
}

/// Drain pending events and fire handlers on the calling thread.
fn server_poll(name: &str) -> Value {
    let mut events_to_fire: Vec<ServerEvent> = Vec::new();
    SERVERS.with(|s| {
        let servers = s.borrow();
        if let Some(state) = servers.get(name) {
            // Non-blocking drain
            while let Ok(evt) = state.events.try_recv() {
                events_to_fire.push(evt);
            }
            // Update client count
            let count = state.clients.lock().unwrap().len() as i64;
            rp_comp_set(name, "clientcount", v_int(count));
        }
    });

    for evt in events_to_fire {
        match evt {
            ServerEvent::ClientConnect { addr } => {
                rp_fire_event_1(name, "onclientconnect", v_str(&addr));
            }
            ServerEvent::DataReceived { addr, data } => {
                rp_fire_event_2(name, "ondatareceived", v_str(&addr), v_str(&data));
            }
            ServerEvent::ClientDisconnect { addr } => {
                rp_fire_event_1(name, "onclientdisconnect", v_str(&addr));
            }
        }
    }
    v_null()
}

fn server_stop(name: &str) -> Value {
    SERVERS.with(|s| {
        if let Some(state) = s.borrow_mut().remove(name) {
            *state.running.lock().unwrap() = false;
            // (the clients' connections closed now, not when their readers
            // next look)
            for (_, stream, _) in state.clients.lock().unwrap().iter() {
                let _ = stream.shutdown(std::net::Shutdown::Both);
            }
        }
    });
    v_null()
}

fn server_broadcast(name: &str, args: &[Value]) -> Value {
    let data = args.first().map(|v| v.to_string_val()).unwrap_or_default();
    SERVERS.with(|s| {
        let servers = s.borrow();
        if let Some(state) = servers.get(name) {
            let mut clients = state.clients.lock().unwrap();
            clients.retain_mut(|(_, stream, _)| {
                writeln!(stream, "{}", data).is_ok()
            });
        }
    });
    v_null()
}

/// SendTo(Address, Data): Data and a line end to the client at Address;
/// Send(Address, Data) (RapidR's): Data as it is, nothing added — an HTTP
/// response, bytes. 1 when it was written.
fn server_sendto(name: &str, args: &[Value], line: bool) -> Value {
    let target = args.first().map(|v| v.to_string_val()).unwrap_or_default();
    let data = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
    let sent = SERVERS.with(|s| {
        let servers = s.borrow();
        let Some(state) = servers.get(name) else { return false };
        let mut clients = state.clients.lock().unwrap();
        let Some((_, stream, _)) = clients.iter_mut().find(|(_, _, addr)| addr == &target) else { return false };
        if line { writeln!(stream, "{}", data).is_ok() } else { stream.write_all(data.as_bytes()).is_ok() }
    });
    v_int(sent as i64)
}

/// Disconnect(Address) (RapidR's): the client at Address's connection
/// closed (OnClientDisconnect follows). 1 when there was one.
fn server_disconnect(name: &str, args: &[Value]) -> Value {
    let target = args.first().map(|v| v.to_string_val()).unwrap_or_default();
    let closed = SERVERS.with(|s| {
        let servers = s.borrow();
        let Some(state) = servers.get(name) else { return false };
        let clients = state.clients.lock().unwrap();
        clients.iter().find(|(_, _, addr)| addr == &target).map(|(_, stream, _)| stream.shutdown(std::net::Shutdown::Both).is_ok()).unwrap_or(false)
    });
    v_int(closed as i64)
}

fn server_accept(name: &str) -> Value {
    // Legacy manual accept - just poll for events
    server_poll(name)
}

// ---------------------------------------------------------------------------
// RHTTP (HTTP client using ureq)
// ---------------------------------------------------------------------------

pub fn http_method(name: &str, method: &str, args: &[Value]) -> Value {
    let name_lower = name.to_lowercase();
    match method {
        "get" => http_get(&name_lower, args),
        "post" => http_post(&name_lower, args),
        _ => {
            eprintln!("[WARN] RHTTP.{}() not implemented", method);
            v_null()
        }
    }
}

/// An agent for one request. Its TLS is the operating system's, with the
/// system's certificates (Cargo.toml): Security.framework on macOS, SChannel
/// on Windows, OpenSSL 3 on Linux.
pub(crate) fn http_agent(timeout: Duration) -> ureq::Agent {
    let builder = ureq::AgentBuilder::new().timeout(timeout);
    let builder = match ureq::native_tls::TlsConnector::new() {
        Ok(tls) => builder.tls_connector(std::sync::Arc::new(tls)),
        Err(e) => {
            eprintln!("[HTTP] TLS unavailable: {e}");
            builder
        }
    };
    builder.build()
}

fn http_get(name: &str, args: &[Value]) -> Value {
    let url = args.first().map(|v| v.to_string_val()).unwrap_or_default();
    let timeout_ms = crate::object::rp_comp_get(name, "timeout").to_i64().max(1000) as u64;

    let agent = http_agent(Duration::from_millis(timeout_ms));

    match agent.get(&url).call() {
        Ok(response) => {
            let status = response.status();
            let body = response.into_string().unwrap_or_default();
            rp_comp_set(name, "statuscode", v_int(status as i64));
            rp_comp_set(name, "responsetext", v_str(&body));
            rp_comp_set(name, "url", v_str(&url));
            v_str(&body)
        }
        Err(e) => {
            eprintln!("[HTTP] GET error: {}", e);
            rp_comp_set(name, "statuscode", v_int(0));
            rp_comp_set(name, "responsetext", v_str(""));
            v_str("")
        }
    }
}

fn http_post(name: &str, args: &[Value]) -> Value {
    let url = args.first().map(|v| v.to_string_val()).unwrap_or_default();
    let body = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
    let timeout_ms = crate::object::rp_comp_get(name, "timeout").to_i64().max(1000) as u64;

    let agent = http_agent(Duration::from_millis(timeout_ms));

    match agent
        .post(&url)
        .set("Content-Type", "application/x-www-form-urlencoded")
        .send_string(&body)
    {
        Ok(response) => {
            let status = response.status();
            let resp_body = response.into_string().unwrap_or_default();
            rp_comp_set(name, "statuscode", v_int(status as i64));
            rp_comp_set(name, "responsetext", v_str(&resp_body));
            rp_comp_set(name, "url", v_str(&url));
            v_str(&resp_body)
        }
        Err(e) => {
            eprintln!("[HTTP] POST error: {}", e);
            rp_comp_set(name, "statuscode", v_int(0));
            rp_comp_set(name, "responsetext", v_str(""));
            v_str("")
        }
    }
}

#[cfg(test)]
mod qsocket_tests {
    use crate::value::{v_int, v_str, Value};

    fn call(m: &str, args: &[Value]) -> Value {
        super::qsocket::method("s", m, args).expect(m)
    }

    #[test]
    fn rapidq_sockets_talk_over_loopback() {
        let port = 47000 + (std::process::id() % 900) as i64;
        // (this computer only: Open's address, RapidR's)
        let srv = call("open", &[v_int(port), v_str("127.0.0.1")]).to_i64();
        assert!(srv > 0);
        let c = call("connect", &[v_str("127.0.0.1"), v_int(port)]).to_i64();
        assert!(c > 0);
        let a = call("accept", &[v_int(srv)]).to_i64();
        assert!(a > 0);
        assert_eq!(call("writeline", &[v_int(c), v_str("hi")]).to_i64(), 4);
        assert_eq!(call("readline", &[v_int(a)]).to_string_val(), "hi");
        call("write", &[v_int(a), v_str("pong!xyz"), v_int(5)]);
        assert_eq!(call("read", &[v_int(c), v_int(10)]).to_string_val(), "pong!");
        assert_eq!(call("isserverready", &[v_int(c)]).to_i64(), 0);
        for s in [c, a, srv] {
            call("close", &[v_int(s)]);
        }
        assert_eq!(call("connect", &[v_str("127.0.0.1"), v_int(1)]).to_i64(), -1);
    }

    /// RServerSocket's reader (Mode "http"): a POST of 1 MB, sent in pieces
    /// with pauses longer than a read's wait, arrives as one whole request;
    /// the client asking for 100 Continue is told to go on.
    #[test]
    fn a_slow_post_arrives_whole() {
        use std::io::{Read, Write};
        use std::sync::Mutex;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let body = "0123456789abcdef".repeat(65536);
        let wire = format!("POST /echo HTTP/1.1\r\nHost: {addr}\r\nExpect: 100-continue\r\nContent-Length: {}\r\n\r\n", body.len());
        let client = std::thread::spawn(move || {
            let mut c = std::net::TcpStream::connect(addr).unwrap();
            c.write_all(wire.as_bytes()).unwrap();
            let mut cont = [0u8; 25];
            c.read_exact(&mut cont).unwrap();
            assert_eq!(&cont, b"HTTP/1.1 100 Continue\r\n\r\n");
            for (i, part) in body.as_bytes().chunks(300_000).enumerate() {
                if i > 0 {
                    std::thread::sleep(std::time::Duration::from_millis(150));
                }
                c.write_all(part).unwrap();
            }
            c
        });
        let (stream, _) = listener.accept().unwrap();
        let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(100)));
        let running = Mutex::new(true);
        let mut got = Vec::new();
        let framing = crate::netframe::Framing::Http(crate::netframe::Limits::default());
        std::thread::scope(|s| {
            let reader = s.spawn(|| super::read_client(stream, framing, &running, |d| got.push(d)));
            // (the client closes once it has sent it all)
            drop(client.join().unwrap());
            reader.join().unwrap();
        });
        assert_eq!(got.len(), 1, "one request");
        let (head, received) = got[0].split_once("\r\n\r\n").unwrap();
        assert!(head.starts_with("POST /echo HTTP/1.1\r\n"), "{head}");
        assert_eq!(received.len(), 1_048_576);
        assert_eq!(received, "0123456789abcdef".repeat(65536));
    }
}
