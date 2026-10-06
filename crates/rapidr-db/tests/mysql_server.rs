//! RMYSQL against a MySQL server made up here: just enough of MySQL's
//! client/server protocol (a handshake, text queries, result sets, OK and
//! error packets) for the component's calls, on a local TCP port. It
//! answers what a real server would, and records the statements it got:
//! RealConnect (host, port, database), CreateDB / DropDB, Refresh's
//! FLUSH statements, FetchLengths / Length, RowBlob and SaveBlob on
//! binary data.
#![cfg(feature = "mysql")]

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use rapidr_db::Host;
use rapidr_value::{v_int, v_str, Value};

/// What the made-up server knows and was asked.
#[derive(Default)]
struct State {
    databases: Vec<String>,
    /// Every query (but the client's own `SELECT @@…`), in order.
    queries: Vec<String>,
    /// The database each handshake asked for.
    logins: Vec<String>,
}

const BLOB: &[u8] = &[0, 0xFF, b'\'', b'A', 0x80, b'\n'];

fn packet(out: &mut Vec<u8>, seq: &mut u8, payload: &[u8]) {
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes()[..3]);
    out.push(*seq);
    *seq = seq.wrapping_add(1);
    out.extend_from_slice(payload);
}

fn lenenc(out: &mut Vec<u8>, bytes: &[u8]) {
    assert!(bytes.len() < 251);
    out.push(bytes.len() as u8);
    out.extend_from_slice(bytes);
}

fn ok() -> Vec<u8> {
    vec![0, 0, 0, 2, 0, 0, 0]
}

fn eof() -> Vec<u8> {
    vec![0xFE, 0, 0, 2, 0]
}

fn error(code: u16, message: &str) -> Vec<u8> {
    let mut p = vec![0xFF];
    p.extend_from_slice(&code.to_le_bytes());
    p.extend_from_slice(b"#HY000");
    p.extend_from_slice(message.as_bytes());
    p
}

/// A text result set: column names (`blob` columns are BLOBs, others
/// VARCHAR) and rows (None: NULL).
fn result_set(columns: &[&str], rows: &[Vec<Option<&[u8]>>]) -> Vec<Vec<u8>> {
    let mut packets = vec![vec![columns.len() as u8]];
    for name in columns {
        let mut c = Vec::new();
        for part in ["def", "test", "t", "t"] {
            lenenc(&mut c, part.as_bytes());
        }
        lenenc(&mut c, name.as_bytes());
        lenenc(&mut c, name.as_bytes());
        c.push(0x0C);
        let (charset, ty, flags): (u16, u8, u16) = if *name == "blob" { (63, 252, 0x90) } else { (255, 253, 0) };
        c.extend_from_slice(&charset.to_le_bytes());
        c.extend_from_slice(&65535u32.to_le_bytes());
        c.push(ty);
        c.extend_from_slice(&flags.to_le_bytes());
        c.extend_from_slice(&[0, 0, 0]);
        packets.push(c);
    }
    packets.push(eof());
    for row in rows {
        let mut r = Vec::new();
        for cell in row {
            match cell {
                Some(bytes) => lenenc(&mut r, bytes),
                None => r.push(0xFB),
            }
        }
        packets.push(r);
    }
    packets.push(eof());
    packets
}

fn answer(state: &Mutex<State>, query: &str) -> Vec<Vec<u8>> {
    let mut s = state.lock().unwrap();
    if query == "SELECT @@max_allowed_packet" {
        return result_set(&["@@max_allowed_packet"], &[vec![Some(b"16777216")]]);
    }
    if query.starts_with("SELECT @@") {
        // (@@socket: none — the client stays on TCP)
        return result_set(&["v"], &[vec![Some(b"")]]);
    }
    s.queries.push(query.to_string());
    let name = |q: &str| q.trim_matches('`').replace("``", "`");
    if query == "SHOW DATABASES" {
        let rows: Vec<Vec<Option<&[u8]>>> = s.databases.iter().map(|d| vec![Some(d.as_bytes())]).collect();
        return result_set(&["Database"], &rows);
    }
    if let Some(db) = query.strip_prefix("CREATE DATABASE ") {
        let db = name(db);
        if s.databases.contains(&db) {
            return vec![error(1007, &format!("Can't create database '{db}'; database exists"))];
        }
        s.databases.push(db);
        return vec![ok()];
    }
    if let Some(db) = query.strip_prefix("DROP DATABASE ") {
        let db = name(db);
        if !s.databases.contains(&db) {
            return vec![error(1008, &format!("Can't drop database '{db}'; database doesn't exist"))];
        }
        s.databases.retain(|d| *d != db);
        return vec![ok()];
    }
    if query == "FLUSH HOSTS" {
        return vec![error(1227, "Access denied; you need (at least one of) the RELOAD privilege(s)")];
    }
    if query == "SELECT id, blob FROM pictures" {
        return result_set(&["id", "blob"], &[vec![Some(b"1"), Some(BLOB)], vec![Some(b"22"), None], vec![Some(b"3"), Some(b"text")]]);
    }
    vec![ok()]
}

fn read_packet(stream: &mut TcpStream) -> Option<(u8, Vec<u8>)> {
    let mut head = [0u8; 4];
    stream.read_exact(&mut head).ok()?;
    let len = u32::from_le_bytes([head[0], head[1], head[2], 0]) as usize;
    let mut payload = vec![0; len];
    stream.read_exact(&mut payload).ok()?;
    Some((head[3], payload))
}

fn serve(mut stream: TcpStream, state: Arc<Mutex<State>>) {
    // The handshake: protocol 10, mysql_native_password.
    let caps: u32 = 1 | 2 | 4 | 8 | 0x200 | 0x2000 | 0x8000 | 0x1_0000 | 0x2_0000 | 0x8_0000;
    let mut h = vec![10];
    h.extend_from_slice(b"8.0.36-rapidr-test\0");
    h.extend_from_slice(&7u32.to_le_bytes());
    h.extend_from_slice(b"abcdefgh\0");
    h.extend_from_slice(&(caps as u16).to_le_bytes());
    h.push(255);
    h.extend_from_slice(&2u16.to_le_bytes());
    h.extend_from_slice(&((caps >> 16) as u16).to_le_bytes());
    h.push(21);
    h.extend_from_slice(&[0; 10]);
    h.extend_from_slice(b"ijklmnopqrst\0");
    h.extend_from_slice(b"mysql_native_password\0");
    let mut out = Vec::new();
    let mut seq = 0;
    packet(&mut out, &mut seq, &h);
    if stream.write_all(&out).is_err() {
        return;
    }
    // The client's answer: flags, max packet, charset, 23 zeros, user\0,
    // auth data (length-prefixed), the database\0 when it names one.
    let Some((seq, login)) = read_packet(&mut stream) else { return };
    let flags = u32::from_le_bytes(login[..4].try_into().unwrap());
    let rest = &login[32..];
    let user_end = rest.iter().position(|&b| b == 0).unwrap();
    let rest = &rest[user_end + 1..];
    let rest = &rest[1 + rest[0] as usize..];
    let db = if flags & 8 != 0 { String::from_utf8_lossy(&rest[..rest.iter().position(|&b| b == 0).unwrap()]).into_owned() } else { String::new() };
    state.lock().unwrap().logins.push(db);
    let (mut out, mut seq) = (Vec::new(), seq + 1);
    packet(&mut out, &mut seq, &ok());
    if stream.write_all(&out).is_err() {
        return;
    }
    while let Some((_, command)) = read_packet(&mut stream) {
        let packets = match command.first() {
            Some(1) | None => return,
            Some(3) => answer(&state, &String::from_utf8_lossy(&command[1..])),
            _ => vec![ok()],
        };
        let (mut out, mut seq) = (Vec::new(), 1);
        for p in packets {
            packet(&mut out, &mut seq, &p);
        }
        if stream.write_all(&out).is_err() {
            return;
        }
    }
}

/// A server on a free local port.
fn server(databases: &[&str]) -> (u16, Arc<Mutex<State>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let state = Arc::new(Mutex::new(State { databases: databases.iter().map(|d| d.to_string()).collect(), ..State::default() }));
    let shared = state.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let state = shared.clone();
            std::thread::spawn(move || serve(stream, state));
        }
    });
    (port, state)
}

/// The component's properties and events, as a runtime keeps them.
#[derive(Default)]
struct Runtime {
    props: RefCell<HashMap<String, Value>>,
    events: RefCell<Vec<String>>,
}

impl Host for Runtime {
    fn set(&self, _name: &str, prop: &str, value: Value) {
        self.props.borrow_mut().insert(prop.to_string(), value);
    }

    fn fire(&self, _name: &str, event: &str, args: &[Value]) {
        let args: Vec<String> = args.iter().map(|a| a.to_string_val()).collect();
        self.events.borrow_mut().push(format!("{event}({})", args.join(",")));
    }

    fn report(&self, _message: &str) {}
}

impl Runtime {
    fn call(&self, method: &str, args: &[Value]) -> Value {
        rapidr_db::mysql::method(self, "MySQL", method, args)
    }

    fn prop(&self, prop: &str) -> i64 {
        self.props.borrow().get(prop).map_or(-1, |v| v.to_i64())
    }
}

fn real_connect(rt: &Runtime, port: u16, db: &str) {
    let args = [v_str("127.0.0.1"), v_str("rapidr"), v_str("secret"), v_str(db), v_int(port as i64), v_str(""), v_int(0)];
    assert_eq!(rt.call("realconnect", &args).to_string_val(), "", "RealConnect is a SUB");
}

#[test]
fn real_connect_names_the_port_and_the_database() {
    let (port, state) = server(&["mysql", "shop"]);
    let rt = Runtime::default();
    real_connect(&rt, port, "shop");
    assert_eq!((rt.prop("connected"), rt.prop("dbcount")), (1, 2));
    assert_eq!(rt.events.borrow().as_slice(), ["onconnect()"]);
    assert_eq!(rt.call("db", &[v_int(1)]).to_string_val(), "shop");
    assert!(state.lock().unwrap().logins.iter().all(|db| db == "shop"));
    rt.call("close", &[]);

    // A port nothing listens on: not connected, OnError says why.
    let closed = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let rt = Runtime::default();
    real_connect(&rt, closed, "");
    assert_eq!(rt.prop("connected"), 0);
    assert!(rt.events.borrow()[0].starts_with("onerror("), "{:?}", rt.events.borrow());
}

#[test]
fn create_db_and_drop_db() {
    let (port, state) = server(&["mysql"]);
    let rt = Runtime::default();
    real_connect(&rt, port, "");
    assert_eq!(rt.call("createdb", &[v_str("shop")]).to_i64(), 1);
    assert_eq!(rt.prop("dbcount"), 2);
    assert_eq!(rt.call("db", &[v_int(1)]).to_string_val(), "shop");
    // (it's there already: 0, and OnError has the server's message)
    assert_eq!(rt.call("createdb", &[v_str("shop")]).to_i64(), 0);
    assert!(rt.events.borrow().last().unwrap().contains("database exists"));
    assert_eq!(rt.call("dropdb", &[v_str("shop")]).to_i64(), 1);
    assert_eq!(rt.prop("dbcount"), 1);
    assert_eq!(rt.call("dropdb", &[v_str("shop")]).to_i64(), 0);
    assert_eq!(rt.call("createdb", &[v_str("odd`name")]).to_i64(), 1);
    let queries = state.lock().unwrap().queries.clone();
    assert!(queries.contains(&"CREATE DATABASE `shop`".to_string()));
    assert!(queries.contains(&"DROP DATABASE `shop`".to_string()));
    assert!(queries.contains(&"CREATE DATABASE `odd``name`".to_string()));
    assert!(state.lock().unwrap().databases.contains(&"odd`name".to_string()));
}

#[test]
fn refresh_flushes() {
    let (port, state) = server(&[]);
    let rt = Runtime::default();
    real_connect(&rt, port, "");
    state.lock().unwrap().queries.clear();
    assert_eq!(rt.call("refresh", &[v_int(1 | 2 | 4 | 16)]).to_i64(), 1);
    assert_eq!(state.lock().unwrap().queries, ["FLUSH PRIVILEGES", "FLUSH LOGS", "FLUSH TABLES", "FLUSH STATUS"]);
    assert_eq!(rt.call("refresh", &[v_int(0)]).to_i64(), 1);
    // (the server refuses FLUSH HOSTS here)
    assert_eq!(rt.call("refresh", &[v_int(8)]).to_i64(), 0);
}

#[test]
fn blobs() {
    let (port, _state) = server(&[]);
    let rt = Runtime::default();
    real_connect(&rt, port, "");
    assert_eq!(rt.call("query", &[v_str("SELECT id, blob FROM pictures")]).to_i64(), 1);
    // Before the first row: nothing.
    assert_eq!(rt.call("fetchlengths", &[]).to_i64(), 0);
    assert_eq!(rt.call("rowblob", &[v_int(1), v_int(6)]).to_string_val(), "");

    assert_eq!(rt.call("fetchrow", &[]).to_i64(), 1);
    // Length(i) is what the last FetchLengths found
    assert_eq!(rt.call("length", &[v_int(1)]).to_i64(), 0);
    assert_eq!(rt.call("fetchlengths", &[]).to_i64(), 1);
    assert_eq!((0..3).map(|i| rt.call("length", &[v_int(i)]).to_i64()).collect::<Vec<_>>(), [1, 6, 0]);
    let blob: String = BLOB.iter().map(|&b| char::from(b)).collect();
    assert_eq!(rt.call("rowblob", &[v_int(1), v_int(6)]).to_string_val(), blob);
    assert_eq!(rt.call("rowblob", &[v_int(1), v_int(2)]).to_string_val(), "\0\u{ff}");
    assert_eq!(rt.call("rowblob", &[v_int(1), v_int(99)]).to_string_val(), blob);
    assert_eq!(rt.call("rowblob", &[v_int(1), v_int(0)]).to_string_val(), "");
    assert_eq!(rt.call("rowblob", &[v_int(0), v_int(1)]).to_string_val(), "1");
    assert_eq!(rt.call("rowblob", &[v_int(5), v_int(1)]).to_string_val(), "");

    let dir = std::env::temp_dir().join(format!("rapidr-db-saveblob-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("picture.bin");
    let path = v_str(file.to_str().unwrap());
    rt.call("saveblob", &[v_int(1), path.clone()]);
    assert_eq!(std::fs::read(&file).unwrap(), BLOB);
    // LoadBlob reads it back as text for a quoted literal
    assert_eq!(rt.call("loadblob", std::slice::from_ref(&path)).to_string_val(), "\\0\u{ff}\\'A\u{80}\\n");

    // A NULL: no bytes
    assert_eq!(rt.call("fetchrow", &[]).to_i64(), 1);
    assert_eq!(rt.call("length", &[v_int(0)]).to_i64(), 1, "still the last FetchLengths'");
    assert_eq!(rt.call("fetchlengths", &[]).to_i64(), 1);
    assert_eq!((rt.call("length", &[v_int(0)]).to_i64(), rt.call("length", &[v_int(1)]).to_i64()), (2, 0));
    rt.call("saveblob", &[v_int(1), path.clone()]);
    assert_eq!(std::fs::read(&file).unwrap(), b"");

    assert_eq!(rt.call("fetchrow", &[]).to_i64(), 1);
    assert_eq!(rt.call("rowblob", &[v_int(1), v_int(4)]).to_string_val(), "text");
    assert_eq!(rt.call("row", &[v_int(1)]).to_string_val(), "text");
    // Past the last row: FetchLengths finds nothing, Length keeps its values
    assert_eq!(rt.call("fetchrow", &[]).to_i64(), 0);
    assert_eq!(rt.call("fetchlengths", &[]).to_i64(), 0);
    assert_eq!(rt.call("length", &[v_int(0)]).to_i64(), 2);
    std::fs::remove_dir_all(&dir).unwrap();
}
