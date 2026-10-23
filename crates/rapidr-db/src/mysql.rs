//! RMYSQL (RapidQ's QMYSQL) on the `mysql` crate: native builds and the
//! interpreter (a browser can't open MySQL's TCP connection; the web
//! runtime says so).
//!
//! `DB.Query(sql)` without values goes as text, as it always has; a query
//! that returns rows (SELECT, SHOW, WITH …, EXPLAIN — whatever MySQL
//! answers with columns) gives the component its rows, the last result set
//! that had some. With values (after the SQL, or queued with AddParam) the
//! SQL is a prepared statement and the values are sent apart from it,
//! bound to its `?` placeholders; the rows then come in MySQL's binary
//! protocol, and are written as the text protocol writes them.
//!
//! Binary data (RapidQ's blobs): a cell keeps the bytes the server sent
//! (RowBlob gives them one character per byte, SaveBlob writes them to a
//! file; FetchLengths then Length(i) counts them), and LoadBlob reads a
//! file as EscapeString's text for a quoted SQL literal.
//!
//! As in RapidQ, the calls that change the server — CreateDB, DropDB,
//! Refresh, SelectDB — return 1 when they worked and 0 when they didn't
//! (the C client's functions return the other way round).
//!
//! Encrypted connections (RapidR's; REQUEST-001 item 7): `SSLMode` says
//! how — MySQL's own client's modes —, on the operating system's TLS
//! (native-tls: Security.framework, SChannel, OpenSSL 3), and `Encrypted`
//! says whether this connection is:
//!
//! | SSLMode | The connection |
//! |---|---|
//! | `Preferred` (the default) | encrypted when the server can, else plain, as before; the server's certificate isn't checked |
//! | `Disabled` | never encrypted |
//! | `Required` | encrypted, or no connection; the certificate isn't checked |
//! | `VerifyCA` | encrypted, the certificate signed by `SSLCA` (a .pem or .der file) or the system's trusted authorities |
//! | `VerifyFull` | as VerifyCA, and the certificate names the host connected to |
//!
//! A Unix socket or a named pipe never carries TLS (the server's own
//! machine): Required and stricter modes refuse them, and Preferred to
//! 127.0.0.1 may end on the server's socket (the crate's `prefer_socket`;
//! Encrypted says so — the server is asked). RealConnect's CLIENT_SSL flag
//! (2048), which RapidQ's client took, asks for Required.
//!
//! `InTransaction` and `Autocommit` (RapidR's; Database Studio's REQUEST-003
//! backlog) say what the server said about the session, never a query of
//! their own: the connection asks MySQL's session tracker
//! (`session_track_transaction_info = STATE`, MySQL 5.7+ / MariaDB 10.3+)
//! to put the transaction's state in its answer to every statement that
//! changes it ([`Tx`]). The `mysql` crate keeps the status flags of the
//! last answer (SERVER_STATUS_IN_TRANS) to itself, and an answer with rows
//! ends without the tracker's news: a query of several statements whose
//! last gives rows can leave it unsaid — InTransaction is then -1 (can't
//! tell) until a statement's answer says it again.
//!
//! `DB.QueryAsync(sql [, values …])` runs it in the background on the
//! connection's worker thread ([`crate::background`]), the rows streaming
//! into the frame under the component's name; `Cancel` asks the server to
//! stop it (`KILL QUERY`, from a second connection of the pool). A
//! background query's cells are text: binary data (RowBlob, SaveBlob)
//! wants Query.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use mysql::consts::{CapabilityFlags, ColumnFlags, ColumnType};
use mysql::prelude::{Protocol, Queryable};
use mysql::{Column, QueryResult, Value as MyValue};
use rapidr_value::objects::codec::bytes_to_string;
use rapidr_value::{v_int, v_null, v_str, Value};

use crate::background::{self, Chunker, End, Shared};
use crate::{cursor_method, param_method, params, publish, sql_and_params, Host, ResultSet};

struct Db {
    /// The connection, shared with a background query while it runs (it
    /// holds the lock then: a query meanwhile is refused, never waited for).
    conn: Arc<Mutex<mysql::PooledConn>>,
    /// The server's id for it (Cancel's KILL QUERY).
    id: u32,
    /// The connection's worker thread (background queries), made by the
    /// first QueryAsync; it ends when the component closes.
    worker: Option<std::sync::mpsc::Sender<Job>>,
    results: ResultSet,
    databases: Vec<String>,
    /// What the last FetchLengths found (Length(i)): the byte lengths of
    /// that row's cells.
    lengths: Vec<i64>,
    /// The session's transaction and autocommit as the server said them
    /// (InTransaction, Autocommit), shared with a background query.
    tx: Arc<Mutex<Tx>>,
    /// What a call on the program's thread just found about them, to set
    /// once the component isn't borrowed.
    observed: Option<Tx>,
    /// The component's name as the program wrote it (SelectDB sets its
    /// properties).
    name: String,
}

/// The session's transaction as MySQL's session tracker said it
/// (InTransaction, Autocommit) — in the answer to a statement that changed
/// it, the server's own word (never a query of ours, never the SQL read).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Tx {
    /// The server tracks the transaction's state for this session (MySQL
    /// 5.7+, MariaDB 10.3+: `session_track_transaction_info`).
    tracked: bool,
    /// A transaction open, or none; `None`: can't tell (an answer that
    /// said it went unseen, or the server doesn't track it).
    open: Option<bool>,
    /// Autocommit on (MySQL's `autocommit`, reported when it changes).
    autocommit: Option<bool>,
}

/// What a query's answers were: how many statements answered without rows
/// (an OK, which carries the tracker's news), how many with rows (whose
/// end carries none), whether the last answered with rows, and the
/// server's error code if one failed.
#[derive(Clone, Copy, Debug, Default)]
struct Answers {
    oks: usize,
    rows: usize,
    last_rows: bool,
    error: Option<u16>,
}

impl Answers {
    fn set(&mut self, has_rows: bool) {
        if has_rows {
            self.rows += 1;
        } else {
            self.oks += 1;
        }
        self.last_rows = has_rows;
    }

    fn failed(&mut self, e: &mysql::Error) {
        self.error = Some(match e {
            mysql::Error::MySqlError(e) => e.code,
            _ => 0,
        });
    }
}

/// MySQL's deadlock error: the server rolled the transaction back.
const ER_LOCK_DEADLOCK: u16 = 1213;

impl Tx {
    /// After a query on `conn` whose answers were `a`: what the last answer
    /// said (its tracker's news), else what can still be known.
    fn after(&mut self, conn: &mysql::PooledConn, a: &Answers) {
        self.heard(tracker_news(conn), a);
    }

    /// [`Tx::after`] with the last answer's news (the transaction's state,
    /// autocommit's new value) in hand.
    fn heard(&mut self, (state, autocommit): (Option<bool>, Option<bool>), a: &Answers) {
        if autocommit.is_some() {
            self.autocommit = autocommit;
        }
        if !self.tracked {
            self.open = None;
            return;
        }
        // (the last answer said it: as it is now)
        if let Some(open) = state {
            self.open = Some(open);
            return;
        }
        // (answers before the last one carried news we never saw: several
        // statements in one query)
        let unseen = if a.last_rows || a.error.is_some() { a.oks } else { a.oks.saturating_sub(1) };
        if unseen > 0 {
            self.open = None;
        } else if a.error == Some(ER_LOCK_DEADLOCK) {
            self.open = Some(false);
        } else if a.rows > 0 && self.autocommit == Some(false) && self.open == Some(false) {
            // (with autocommit off, reading a table begins a transaction,
            // and an answer with rows can't say so)
            self.open = None;
        }
    }

    /// InTransaction: 1 open, 0 none, -1 can't tell.
    fn value(&self) -> i64 {
        match self.open {
            Some(true) => 1,
            Some(false) => 0,
            None => -1,
        }
    }

    /// Autocommit: 1 on, 0 off; -1 not known.
    fn autocommit_value(&self) -> i64 {
        self.autocommit.map_or(-1, i64::from)
    }
}

/// The tracker's news in the connection's last answer: the transaction's
/// state (its first character `T` explicit, `I` implicit, `_` none) and
/// autocommit's new value.
fn tracker_news(conn: &mysql::PooledConn) -> (Option<bool>, Option<bool>) {
    use mysql::session_state_change::SessionStateChange;
    let (mut state, mut autocommit) = (None, None);
    for info in conn.session_state_changes().unwrap_or_default() {
        match info.decode() {
            Ok(SessionStateChange::TransactionState(t)) => state = t.as_bytes().first().map(|c| *c != b'_'),
            Ok(SessionStateChange::SystemVariables(vars)) => {
                for v in vars.iter().filter(|v| v.name_str().eq_ignore_ascii_case("autocommit")) {
                    autocommit = Some(matches!(v.value_str().to_ascii_uppercase().as_str(), "ON" | "1"));
                }
            }
            _ => {}
        }
    }
    (state, autocommit)
}

/// Asks the server to say the transaction's state and autocommit in its
/// answers (a fresh session: none open — unless autocommit is off, when
/// connecting's own reads may have begun one).
fn start_tracking(conn: &mut mysql::PooledConn) -> Tx {
    let tracked = conn.query_drop("SET SESSION session_track_transaction_info = 'STATE'").is_ok();
    // (read cell by cell: a typed row that doesn't fit would panic)
    let row: Option<mysql::Row> = conn.query_first("SELECT @@autocommit, @@session_track_system_variables").ok().flatten();
    let found = row.and_then(|r| Some((r.get_opt::<i64, usize>(0)?.ok()?, r.get_opt::<String, usize>(1).and_then(Result::ok).unwrap_or_default())));
    let autocommit = found.as_ref().map(|(a, _)| *a != 0);
    // (autocommit's changes reported: the server's list has it by default)
    if let Some((_, vars)) = found.filter(|_| tracked) {
        if !vars.split(',').any(|v| matches!(v.trim(), "*" | "autocommit")) {
            let _ = conn.query_drop("SET SESSION session_track_system_variables = CONCAT_WS(',', NULLIF(@@session_track_system_variables, ''), 'autocommit')");
        }
    }
    let open = if tracked && autocommit != Some(false) { Some(false) } else { None };
    Tx { tracked, open, autocommit }
}

thread_local! {
    static DBS: RefCell<HashMap<String, Db>> = RefCell::new(HashMap::new());
    static POOLS: RefCell<HashMap<String, mysql::Pool>> = RefCell::new(HashMap::new());
}

/// `name.method(args)` of an RMYSQL (`method` lowercase).
pub fn method(host: &dyn Host, name: &str, method: &str, args: &[Value]) -> Value {
    let key = name.to_lowercase();
    if let Some(v) = param_method(&key, method, args) {
        return v;
    }
    let int = |i: usize| args.get(i).map_or(0, |v| v.to_i64());
    let text = |i: usize| args.get(i).map(|v| v.to_string_val()).unwrap_or_default();
    match method {
        "connect" => return connect(host, name, &key, args),
        "realconnect" => {
            let mut server = Server {
                host: text(0),
                user: text(1),
                password: text(2),
                db: text(3),
                port: int(4),
                socket: text(5),
                flags: int(6),
                ssl: SslMode::Preferred,
                ssl_ca: String::new(),
            };
            if !server.tls_from(host, name) {
                return v_null();
            }
            // (RapidQ's CLIENT_SSL: encrypted or nothing)
            if server.flags & CLIENT_SSL != 0 && server.ssl == SslMode::Preferred {
                server.ssl = SslMode::Required;
            }
            open(host, name, &key, server);
            return v_null();
        }
        "close" | "disconnect" => return close(host, name, &key),
        "query" | "execute" => return query(host, name, &key, args),
        "queryasync" => return query_async(host, name, &key, args),
        "schema" => return schema(host, name, &key),
        "cancel" => return v_int(background::cancel(&key) as i64),
        "escapestring" => {
            // (EscapeString(S, Length): the first Length characters)
            let s = text(0);
            let s = match args.get(1) {
                Some(n) => s.chars().take(n.to_i64().max(0) as usize).collect(),
                None => s,
            };
            return v_str(&escape(&s));
        }
        "loadblob" => return load_blob(host, name, &text(0)),
        "saveblob" => {
            save_blob(host, name, &key, int(0), &text(1));
            return v_null();
        }
        "rowblob" => {
            let blob = with_db(&key, |db| db.results.cell_bytes(int(0)).map(|b| bytes_to_string(&b[..b.len().min(int(1).max(0) as usize)])));
            return v_str(&blob.flatten().unwrap_or_default());
        }
        "fetchlengths" => {
            let fetched = with_db(&key, |db| match db.results.lengths() {
                Some(lengths) => {
                    db.lengths = lengths;
                    true
                }
                None => false,
            });
            return v_int(fetched.unwrap_or(false) as i64);
        }
        "length" => {
            let n = with_db(&key, |db| usize::try_from(int(0)).ok().and_then(|i| db.lengths.get(i).copied()));
            return v_int(n.flatten().unwrap_or(0));
        }
        "createdb" => return server_command(host, name, &key, &create_db_sql(&text(0)), true),
        "dropdb" => return server_command(host, name, &key, &drop_db_sql(&text(0)), true),
        "refresh" => {
            let mut done = true;
            for sql in refresh_sql(int(0)) {
                done = server_command(host, name, &key, sql, false).to_i64() == 1;
                if !done {
                    break;
                }
            }
            return v_int((done && connected(&key)) as i64);
        }
        "selectdb" => return select_db(host, &key, args),
        "db" => {
            let i = args.first().map(|v| v.to_i64()).unwrap_or(0);
            let db = DBS.with(|d| {
                let dbs = d.borrow();
                usize::try_from(i).ok().and_then(|i| dbs.get(&key)?.databases.get(i).cloned())
            });
            return v_str(&db.unwrap_or_default());
        }
        _ => {}
    }
    let fetched = DBS.with(|d| cursor_method(d.borrow_mut().get_mut(&key).map(|db| &mut db.results), method, args));
    match fetched {
        Some(v) => {
            if method == "fetchrow" && v.to_i64() == 1 {
                host.row_changed(name);
            }
            v
        }
        None => {
            host.report(&format!("[WARN] RMySQL.{method}() not implemented"));
            v_null()
        }
    }
}

impl Db {
    /// The connection for a query now: Err while a background query has it.
    fn conn(&self) -> Result<MutexGuard<'_, mysql::PooledConn>, String> {
        self.conn.try_lock().map_err(|_| background::BUSY.to_string())
    }
}

/// `f(db)` on the component's connection (None: not connected).
fn with_db<T>(key: &str, f: impl FnOnce(&mut Db) -> T) -> Option<T> {
    DBS.with(|d| d.borrow_mut().get_mut(key).map(f))
}

fn connected(key: &str) -> bool {
    with_db(key, |_| ()).is_some()
}

/// The component's name as the program wrote it.
fn name_of(key: &str) -> String {
    with_db(key, |db| db.name.clone()).unwrap_or_else(|| key.to_string())
}

/// Where Connect and RealConnect go.
#[derive(Clone)]
struct Server {
    host: String,
    user: String,
    password: String,
    /// The database to use ("": none).
    db: String,
    /// 0: MySQL's (3306).
    port: i64,
    /// A Unix socket's path (Windows: a named pipe's name); "": TCP.
    socket: String,
    /// The C client's CLIENT_* flags.
    flags: i64,
    /// How the connection is encrypted (SSLMode).
    ssl: SslMode,
    /// The certificate authority VerifyCA and VerifyFull trust (SSLCA: a
    /// .pem or .der file; "" the system's).
    ssl_ca: String,
}

impl Server {
    /// Takes the component's SSLMode and SSLCA; false (and OnError, with
    /// what's wrong) when SSLMode is none of the modes or SSLCA isn't a
    /// file.
    fn tls_from(&mut self, host: &dyn Host, name: &str) -> bool {
        let fail = |e: String| {
            host.report(&format!("[MySQL] Connection error: {e}"));
            host.set(name, "connected", v_int(0));
            host.set(name, "encrypted", v_int(0));
            host.fire(name, "onerror", &[v_str(&e)]);
            false
        };
        self.ssl = match SslMode::parse(&host.get(name, "sslmode").to_string_val()) {
            Ok(m) => m,
            Err(e) => return fail(e),
        };
        // (the authority only the verifying modes use)
        self.ssl_ca = host.get(name, "sslca").to_string_val().trim().to_string();
        if !matches!(self.ssl, SslMode::VerifyCa | SslMode::VerifyFull) {
            self.ssl_ca.clear();
        }
        if !self.ssl_ca.is_empty() && !std::path::Path::new(&self.ssl_ca).is_file() {
            return fail(format!("SSLCA: no such file: {}", self.ssl_ca));
        }
        true
    }
}

/// How a connection is encrypted: RMySQL's SSLMode, MySQL's own client's
/// modes (module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SslMode {
    Disabled,
    Preferred,
    Required,
    VerifyCa,
    VerifyFull,
}

impl SslMode {
    /// The mode SSLMode names (case, spaces, `-` and `_` ignored; MySQL's
    /// VERIFY_IDENTITY is VerifyFull; "" the default, Preferred).
    fn parse(text: &str) -> Result<SslMode, String> {
        let t: String = text.chars().filter(|c| !matches!(c, ' ' | '-' | '_')).flat_map(char::to_lowercase).collect();
        Ok(match t.as_str() {
            "" | "preferred" | "prefer" => SslMode::Preferred,
            "disabled" | "disable" | "off" | "none" => SslMode::Disabled,
            "required" | "require" => SslMode::Required,
            "verifyca" => SslMode::VerifyCa,
            "verifyfull" | "verifyidentity" => SslMode::VerifyFull,
            _ => return Err(format!("SSLMode `{}` isn't one of Disabled, Preferred, Required, VerifyCA, VerifyFull", text.trim())),
        })
    }

    /// The `mysql` crate's TLS options for this mode (`None`: plain).
    fn opts(self, ca: &str) -> Option<mysql::SslOpts> {
        let ca = (!ca.is_empty()).then(|| std::path::PathBuf::from(ca));
        Some(match self {
            SslMode::Disabled => return None,
            // (encrypted, the certificate not checked: as MySQL's client)
            SslMode::Preferred | SslMode::Required => mysql::SslOpts::default().with_danger_accept_invalid_certs(true).with_danger_skip_domain_validation(true),
            SslMode::VerifyCa => mysql::SslOpts::default().with_root_cert_path(ca).with_danger_skip_domain_validation(true),
            SslMode::VerifyFull => mysql::SslOpts::default().with_root_cert_path(ca),
        })
    }
}

/// `Connect(host, user, password [, database])`, each missing one from
/// the component's property (Host, User, Password, DB; Port; SSLMode,
/// SSLCA).
fn connect(host: &dyn Host, name: &str, key: &str, args: &[Value]) -> Value {
    let arg = |i: usize, prop: &str| args.get(i).map(|v| v.to_string_val()).unwrap_or_else(|| host.get(name, prop).to_string_val());
    let mut server = Server {
        host: arg(0, "host"),
        user: arg(1, "user"),
        password: arg(2, "password"),
        db: arg(3, "db"),
        port: 0,
        socket: String::new(),
        flags: 0,
        ssl: SslMode::Preferred,
        ssl_ca: String::new(),
    };
    if !server.tls_from(host, name) {
        return v_int(0);
    }
    v_int(open(host, name, key, server) as i64)
}

/// The C client's flags RealConnect passes on: FOUND_ROWS (2), LONG_FLAG
/// (4), NO_SCHEMA (16), ODBC (64), IGNORE_SPACE (256), INTERACTIVE (1024).
/// COMPRESS (32) turns compression on; SSL (2048) asks for an encrypted
/// connection (SSLMode Required, unless SSLMode says more); the rest are
/// the connection's own business (the `mysql` crate sets them).
const PASSED_FLAGS: u32 = 2 | 4 | 16 | 64 | 256 | 1024;
const CLIENT_COMPRESS: i64 = 32;
const CLIENT_SSL: i64 = 2048;

/// The connection options for `server`: "" is this machine (as the C
/// client's NULL host), port 0 the component's Port or MySQL's 3306;
/// encrypted as its SSLMode says (Preferred: tried encrypted first).
fn options(server: &Server, port_property: i64) -> mysql::OptsBuilder {
    let host = if server.host.is_empty() { "localhost" } else { server.host.as_str() };
    let port = [server.port, port_property].into_iter().find(|p| (1..=65535).contains(p)).unwrap_or(3306) as u16;
    let mut opts = mysql::OptsBuilder::new()
        .ip_or_hostname(Some(host))
        .user(Some(&server.user))
        .pass(Some(&server.password))
        .tcp_port(port)
        // (the session tracker's news in the server's answers:
        // InTransaction, Autocommit)
        .additional_capabilities(CapabilityFlags::from_bits_truncate(server.flags as u32 & PASSED_FLAGS) | CapabilityFlags::CLIENT_SESSION_TRACK);
    if !server.db.is_empty() {
        opts = opts.db_name(Some(&server.db));
    }
    if !server.socket.is_empty() {
        opts = opts.socket(Some(&server.socket));
    }
    if server.flags & CLIENT_COMPRESS != 0 {
        opts = opts.compress(Some(mysql::Compression::default()));
    }
    // (a socket or a pipe is the server's own machine: no TLS there, and
    // Preferred connects plain — `open` refuses the stricter modes)
    let mode = if server.socket.is_empty() { server.ssl } else { SslMode::Disabled };
    // (the crate moves a connection to 127.0.0.1 onto the server's socket,
    // where TLS goes: never for a mode that must be encrypted)
    if matches!(mode, SslMode::Required | SslMode::VerifyCa | SslMode::VerifyFull) {
        opts = opts.prefer_socket(false);
    }
    opts.ssl_opts(mode.opts(&server.ssl_ca))
}

/// Whether `conn` is encrypted, as the server says (its session's cipher):
/// `None` when it won't say.
fn server_says_encrypted(conn: &mut mysql::PooledConn) -> Option<bool> {
    let row: Option<(String, String)> = conn.query_first("SHOW SESSION STATUS LIKE 'Ssl_cipher'").ok()?;
    Some(row.is_some_and(|(_, cipher)| !cipher.trim().is_empty()))
}

/// Why a connection in `mode` can't be made at all (a stricter mode
/// through a socket), before trying.
fn refused(server: &Server) -> Option<String> {
    (!server.socket.is_empty() && !matches!(server.ssl, SslMode::Disabled | SslMode::Preferred))
        .then(|| "an encrypted connection (SSLMode Required, VerifyCA or VerifyFull) needs TCP: a Unix socket or a named pipe carries no TLS".to_string())
}

/// A connection error in plain words: the server without TLS for a mode
/// that needs it, a certificate refused.
fn explain(e: &mysql::Error, server: &Server) -> String {
    match e {
        mysql::Error::DriverError(mysql::DriverError::TlsNotSupported) => {
            "the server doesn't offer encrypted connections (TLS), and SSLMode asks for one: turn TLS on in the server, or set SSLMode = \"Preferred\"".into()
        }
        mysql::Error::TlsError(t) => match server.ssl {
            // (macOS takes no server certificate valid for more than 825
            // days: MySQL's own self-made ones are valid for ten years)
            SslMode::VerifyCa | SslMode::VerifyFull if t.to_string().contains("validity period") => {
                format!("the server's certificate wasn't accepted ({t}): this system takes only certificates valid for at most 825 days, and a server's self-made one usually is valid longer; give the server a shorter one, or use SSLMode = \"Required\"")
            }
            SslMode::VerifyCa | SslMode::VerifyFull => format!("the server's certificate wasn't accepted ({t}): check SSLCA{}", if server.ssl == SslMode::VerifyFull { " and that Host is a name the certificate has" } else { "" }),
            _ => format!("the encrypted connection failed ({t})"),
        },
        e => e.to_string(),
    }
}

/// Connects (Connect, RealConnect): whether it did. Connected, Encrypted,
/// DB(i), DBCount and OnConnect follow, or OnError with the message.
fn open(host: &dyn Host, name: &str, key: &str, server: Server) -> bool {
    // (a credential: the debugger and RapidR's messages mask it)
    rapidr_value::secrets::note(&server.password);
    let port = host.get(name, "port").to_i64();
    let attempt = |server: &Server| mysql::Pool::new(options(server, port)).and_then(|pool| Ok((pool.get_conn()?, pool)));
    let mut encrypted = server.socket.is_empty() && server.ssl != SslMode::Disabled;
    let opened = match refused(&server) {
        Some(e) => Err(e),
        None => match attempt(&server) {
            // (Preferred: the server without TLS, or a TLS it and the
            // system don't share — plain, as before SSLMode)
            Err(mysql::Error::DriverError(mysql::DriverError::TlsNotSupported) | mysql::Error::TlsError(_)) if encrypted && server.ssl == SslMode::Preferred => {
                encrypted = false;
                attempt(&Server { ssl: SslMode::Disabled, ..server.clone() }).map_err(|e| explain(&e, &server))
            }
            r => r.map_err(|e| explain(&e, &server)),
        },
    };
    match opened {
        Ok((mut conn, pool)) => {
            // (what the server says, when it says: Preferred on this
            // machine may have gone onto its socket)
            let encrypted = server_says_encrypted(&mut conn).unwrap_or(encrypted);
            // (the databases the server has: DB(i), DBCount)
            let databases: Vec<String> = conn.query("SHOW DATABASES").unwrap_or_default();
            let count = databases.len() as i64;
            let id = conn.connection_id();
            let tx = start_tracking(&mut conn);
            background::forget(key);
            let db = Db { conn: Arc::new(Mutex::new(conn)), id, worker: None, results: ResultSet::default(), databases, lengths: Vec::new(), tx: Arc::new(Mutex::new(tx)), observed: None, name: name.to_string() };
            DBS.with(|d| d.borrow_mut().insert(key.to_string(), db));
            POOLS.with(|p| p.borrow_mut().insert(key.to_string(), pool));
            host.set(name, "connected", v_int(1));
            host.set(name, "encrypted", v_int(encrypted as i64));
            host.set(name, "intransaction", v_int(tx.value()));
            host.set(name, "autocommit", v_int(tx.autocommit_value()));
            host.set(name, "dbcount", v_int(count));
            host.row_changed(name);
            host.fire(name, "onconnect", &[]);
            true
        }
        Err(e) => {
            // (never the password, should the server's message hold it)
            let e = rapidr_value::secrets::mask(&e).into_owned();
            host.report(&format!("[MySQL] Connection error: {e}"));
            host.set(name, "connected", v_int(0));
            host.set(name, "encrypted", v_int(0));
            host.set(name, "intransaction", v_int(0));
            host.fire(name, "onerror", &[v_str(&e)]);
            false
        }
    }
}

/// A statement that changes the server (CreateDB, DropDB, Refresh's
/// FLUSH): 1 if it ran, else 0 (and OnError, when connected). After
/// CreateDB / DropDB (`databases`) the server's list of databases is read
/// again: DB(i), DBCount.
fn server_command(host: &dyn Host, name: &str, key: &str, sql: &str, databases: bool) -> Value {
    let mut tx = None;
    let done = with_db(key, |db| {
        let found = (|| {
            if let Some(busy) = background::refused(key) {
                return Err(busy);
            }
            let mut conn = db.conn()?;
            let text = |e: mysql::Error| e.to_string();
            let ran = conn.query_drop(sql);
            // (CREATE / DROP DATABASE commit a transaction: the answer says)
            let mut a = Answers::default();
            match &ran {
                Ok(()) => a.set(false),
                Err(e) => a.failed(e),
            }
            tx = Some(observe(&db.tx, &conn, &a));
            ran.map_err(text)?;
            let list = if databases { Some(conn.query("SHOW DATABASES").map_err(text)?) } else { None };
            Ok::<_, String>(list)
        })();
        found.map(|list| {
            if let Some(list) = list {
                db.databases = list;
            }
            db.databases.len() as i64
        })
    });
    if let Some(tx) = tx {
        publish_tx(host, name, tx);
    }
    match done {
        Some(Ok(count)) => {
            if databases {
                host.set(name, "dbcount", v_int(count));
            }
            v_int(1)
        }
        Some(Err(e)) => {
            host.report(&format!("[MySQL] {sql}: {e}"));
            host.fire(name, "onerror", &[v_str(&e)]);
            v_int(0)
        }
        None => v_int(0),
    }
}

/// A database's name as an SQL identifier.
fn identifier(name: &str) -> String {
    format!("`{}`", name.replace('`', "``"))
}

/// CreateDB's statement (what the C client's mysql_create_db asks for).
fn create_db_sql(name: &str) -> String {
    format!("CREATE DATABASE {}", identifier(name))
}

fn drop_db_sql(name: &str) -> String {
    format!("DROP DATABASE {}", identifier(name))
}

/// Refresh(flags)'s statements, the FLUSH each of the C client's REFRESH_*
/// flags asks for (MYSQL.INC's Refresh_Grant 1, Refresh_Log 2,
/// Refresh_Table 4, Refresh_Hosts 8, Refresh_Status 16). Other bits
/// (REFRESH_THREADS 32, Refresh_Fast 32768, …) ask nothing of the server.
fn refresh_sql(flags: i64) -> Vec<&'static str> {
    const FLUSHES: [(i64, &str); 5] =
        [(1, "FLUSH PRIVILEGES"), (2, "FLUSH LOGS"), (4, "FLUSH TABLES"), (8, "FLUSH HOSTS"), (16, "FLUSH STATUS")];
    FLUSHES.iter().filter(|(bit, _)| flags & bit != 0).map(|&(_, sql)| sql).collect()
}

/// Text for a quoted SQL literal, as the C client's mysql_escape_string
/// writes it: NUL, LF, CR, backslash, both quotes and Ctrl+Z (\0 \n \r \\
/// \' \" \Z) escaped with a backslash; every other character as it is.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + text.len() / 8);
    for c in text.chars() {
        match c {
            '\0' => out.push_str("\\0"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            '"' => out.push_str("\\\""),
            '\x1a' => out.push_str("\\Z"),
            c => out.push(c),
        }
    }
    out
}

/// LoadBlob(file): the file's bytes (one character each) escaped for a
/// quoted SQL literal; "" when it can't be read (and OnError).
fn load_blob(host: &dyn Host, name: &str, file: &str) -> Value {
    match std::fs::read(file) {
        Ok(bytes) => v_str(&escape(&bytes_to_string(&bytes))),
        Err(e) => {
            host.report(&format!("[MySQL] LoadBlob {file}: {e}"));
            host.fire(name, "onerror", &[v_str(&format!("{file}: {e}"))]);
            v_str("")
        }
    }
}

/// SaveBlob(field, file): the current row's column `field`, the bytes the
/// server sent, written to the file (made new). Nothing without a current
/// row or such a column.
fn save_blob(host: &dyn Host, name: &str, key: &str, field: i64, file: &str) {
    let bytes = with_db(key, |db| db.results.cell_bytes(field).map(|b| b.into_owned())).flatten();
    if let Some(bytes) = bytes {
        if let Err(e) = std::fs::write(file, bytes) {
            host.report(&format!("[MySQL] SaveBlob {file}: {e}"));
            host.fire(name, "onerror", &[v_str(&format!("{file}: {e}"))]);
        }
    }
}

fn close(host: &dyn Host, name: &str, key: &str) -> Value {
    background::forget(key);
    let closed = DBS.with(|d| d.borrow_mut().remove(key));
    POOLS.with(|p| p.borrow_mut().remove(key));
    host.set(name, "connected", v_int(0));
    host.set(name, "encrypted", v_int(0));
    host.set(name, "busy", v_int(0));
    // (the server rolls an open transaction back as the session ends)
    host.set(name, "intransaction", v_int(0));
    if closed.is_some() {
        drop(closed);
        host.fire(name, "ondisconnect", &[]);
    }
    v_null()
}

/// `Query(sql [, values …])`: 1 if it ran, else 0 (OnError says why).
/// OnQueryDone follows either way.
fn query(host: &dyn Host, name: &str, key: &str, args: &[Value]) -> Value {
    let (sql, values) = sql_and_params(key, args);
    let mut tx = None;
    let outcome = DBS.with(|d| {
        let mut dbs = d.borrow_mut();
        let db = dbs.get_mut(key).ok_or_else(|| crate::NOT_CONNECTED.to_string())?;
        if let Some(busy) = background::refused(key) {
            return Err(busy);
        }
        let table = {
            let mut conn = db.conn()?;
            let mut a = Answers::default();
            let found = run_query(&mut conn, &sql, &values, &mut a);
            // (what the server said about the transaction, whatever happened)
            tx = Some(observe(&db.tx, &conn, &a));
            found?
        };
        Ok::<_, String>(table.map(|(columns, types, rows, raw)| {
            db.results = ResultSet::with_raw(columns, rows, raw).typed(types);
            db.lengths.clear();
            (db.results.rows.len(), db.results.columns.len())
        }))
    });
    if let Some(tx) = tx {
        publish_tx(host, name, tx);
    }
    let ok = match outcome {
        Ok(size) => {
            if let Some(size) = size {
                // (a grid bound to the component shows these rows too)
                crate::publish_frame(key, || DBS.with(|d| d.borrow().get(key).map(|db| (db.results.columns.clone(), db.results.rows.clone()))));
                publish(host, name, size);
            }
            true
        }
        Err(e) => {
            host.report(&format!("[MySQL] Query error: {e}"));
            host.fire(name, "onerror", &[v_str(&e)]);
            false
        }
    };
    host.fire(name, "onquerydone", &[]);
    v_int(ok as i64)
}

/// Query's statement(s) on `conn`: as text without values, else prepared
/// with them bound; the last result that had rows. `a` gets the answers.
fn run_query(conn: &mut mysql::PooledConn, sql: &str, values: &[Value], a: &mut Answers) -> Result<Option<Rows>, String> {
    let text = |e: mysql::Error| e.to_string();
    if values.is_empty() {
        let mut result = conn.query_iter(sql).map_err(|e| {
            a.failed(&e);
            text(e)
        })?;
        collect(&mut result, text_cell, a)
    } else {
        let stmt = conn.prep(sql).map_err(|e| {
            a.failed(&e);
            text(e)
        })?;
        if stmt.num_params() as usize != values.len() {
            return Err(params::wrong_count(stmt.num_params() as usize, values.len()));
        }
        let values: Vec<MyValue> = values.iter().map(my_value).collect();
        let mut result = conn.exec_iter(&stmt, mysql::Params::Positional(values)).map_err(|e| {
            a.failed(&e);
            text(e)
        })?;
        collect(&mut result, binary_cell, a)
    }
}

/// The session's transaction after a query whose answers were `a` (the
/// state shared with background queries updated): InTransaction's and
/// Autocommit's values.
fn observe(tx: &Mutex<Tx>, conn: &mysql::PooledConn, a: &Answers) -> Tx {
    let mut t = tx.lock().unwrap_or_else(|e| e.into_inner());
    t.after(conn, a);
    *t
}

/// InTransaction and Autocommit set from `tx`.
fn publish_tx(host: &dyn Host, name: &str, tx: Tx) {
    host.set(name, "intransaction", v_int(tx.value()));
    host.set(name, "autocommit", v_int(tx.autocommit_value()));
}

/// A result set: its columns' names and types (the server's metadata),
/// its rows' text, and the bytes of the cells that aren't text
/// ([`ResultSet::raw`]).
type Rows = (Vec<String>, Vec<String>, Vec<Vec<String>>, Vec<Vec<Option<Vec<u8>>>>);

/// A result column's type as MySQL names it in a table's definition (the
/// result's metadata: the server sends it with every result set, a
/// read-only connection's too): `INT`, `BIGINT UNSIGNED`, `DECIMAL`,
/// `VARCHAR`, `VARBINARY`, `TEXT`, `BLOB`, `DATETIME`, `ENUM`, `JSON` …
pub fn type_name(t: ColumnType, flags: ColumnFlags, charset: u16) -> String {
    use ColumnType::*;
    // (character set 63 is `binary`: BLOB rather than TEXT, VARBINARY …)
    let binary = charset == 63;
    let numeric = matches!(t, MYSQL_TYPE_TINY | MYSQL_TYPE_SHORT | MYSQL_TYPE_INT24 | MYSQL_TYPE_LONG | MYSQL_TYPE_LONGLONG | MYSQL_TYPE_DECIMAL | MYSQL_TYPE_NEWDECIMAL | MYSQL_TYPE_FLOAT | MYSQL_TYPE_DOUBLE);
    let name = if flags.contains(ColumnFlags::ENUM_FLAG) {
        "ENUM"
    } else if flags.contains(ColumnFlags::SET_FLAG) {
        "SET"
    } else {
        match t {
            MYSQL_TYPE_DECIMAL | MYSQL_TYPE_NEWDECIMAL => "DECIMAL",
            MYSQL_TYPE_TINY => "TINYINT",
            MYSQL_TYPE_SHORT => "SMALLINT",
            MYSQL_TYPE_INT24 => "MEDIUMINT",
            MYSQL_TYPE_LONG => "INT",
            MYSQL_TYPE_LONGLONG => "BIGINT",
            MYSQL_TYPE_FLOAT => "FLOAT",
            MYSQL_TYPE_DOUBLE => "DOUBLE",
            MYSQL_TYPE_NULL => "NULL",
            MYSQL_TYPE_TIMESTAMP | MYSQL_TYPE_TIMESTAMP2 => "TIMESTAMP",
            MYSQL_TYPE_DATE | MYSQL_TYPE_NEWDATE => "DATE",
            MYSQL_TYPE_TIME | MYSQL_TYPE_TIME2 => "TIME",
            MYSQL_TYPE_DATETIME | MYSQL_TYPE_DATETIME2 => "DATETIME",
            MYSQL_TYPE_YEAR => "YEAR",
            MYSQL_TYPE_BIT => "BIT",
            MYSQL_TYPE_JSON => "JSON",
            MYSQL_TYPE_ENUM => "ENUM",
            MYSQL_TYPE_SET => "SET",
            MYSQL_TYPE_GEOMETRY => "GEOMETRY",
            MYSQL_TYPE_VARCHAR | MYSQL_TYPE_VAR_STRING if binary => "VARBINARY",
            MYSQL_TYPE_VARCHAR | MYSQL_TYPE_VAR_STRING => "VARCHAR",
            MYSQL_TYPE_STRING if binary => "BINARY",
            MYSQL_TYPE_STRING => "CHAR",
            MYSQL_TYPE_TINY_BLOB if binary => "TINYBLOB",
            MYSQL_TYPE_TINY_BLOB => "TINYTEXT",
            MYSQL_TYPE_MEDIUM_BLOB if binary => "MEDIUMBLOB",
            MYSQL_TYPE_MEDIUM_BLOB => "MEDIUMTEXT",
            MYSQL_TYPE_LONG_BLOB if binary => "LONGBLOB",
            MYSQL_TYPE_LONG_BLOB => "LONGTEXT",
            MYSQL_TYPE_BLOB if binary => "BLOB",
            MYSQL_TYPE_BLOB => "TEXT",
            _ => "",
        }
    };
    if numeric && flags.contains(ColumnFlags::UNSIGNED_FLAG) {
        format!("{name} UNSIGNED")
    } else {
        name.to_string()
    }
}

/// The result columns' names and types.
fn names_and_types(columns: &[Column]) -> (Vec<String>, Vec<String>) {
    columns.iter().map(|c| (c.name_str().into_owned(), type_name(c.column_type(), c.flags(), c.character_set()))).unzip()
}

/// Every result set of a query read to its end: the last that had columns.
fn collect<T: Protocol>(result: &mut QueryResult<'_, '_, '_, T>, cell: fn(&MyValue, &Column) -> String, a: &mut Answers) -> Result<Option<Rows>, String> {
    let mut last = None;
    while let Some(set) = result.iter() {
        let columns: Vec<Column> = set.columns().as_ref().to_vec();
        let (mut rows, mut raw) = (Vec::new(), Vec::new());
        for row in set {
            let row = row.map_err(|e| {
                a.failed(&e);
                e.to_string()
            })?;
            let cells: Vec<(String, Option<Vec<u8>>)> =
                columns.iter().enumerate().map(|(i, c)| row.as_ref(i).map(|v| bytes_cell(v, c, cell)).unwrap_or_default()).collect();
            let binary = cells.iter().any(|(_, b)| b.is_some());
            let (texts, bytes): (Vec<String>, Vec<Option<Vec<u8>>>) = cells.into_iter().unzip();
            rows.push(texts);
            raw.push(if binary { bytes } else { Vec::new() });
        }
        a.set(!columns.is_empty());
        if !columns.is_empty() {
            if raw.iter().all(Vec::is_empty) {
                raw.clear();
            }
            let (names, types) = names_and_types(&columns);
            last = Some((names, types, rows, raw));
        }
    }
    Ok(last)
}

/// A cell's text, and its bytes when they aren't UTF-8 text (binary
/// data: the text is then "").
fn bytes_cell(v: &MyValue, column: &Column, cell: fn(&MyValue, &Column) -> String) -> (String, Option<Vec<u8>>) {
    match v {
        MyValue::Bytes(b) => match std::str::from_utf8(b) {
            Ok(s) => (s.to_string(), None),
            Err(_) => (String::new(), Some(b.clone())),
        },
        other => (cell(other, column), None),
    }
}

/// A value of the text protocol (NULL as "").
fn text_cell(v: &MyValue, column: &Column) -> String {
    match v {
        MyValue::Bytes(b) => String::from_utf8(b.clone()).unwrap_or_default(),
        other => binary_cell(other, column),
    }
}

/// A value of the binary protocol, as the text protocol writes it.
fn binary_cell(v: &MyValue, column: &Column) -> String {
    let fraction = |micros: u32| match column.decimals() {
        d @ 1..=6 => format!(".{:06}", micros)[..1 + d as usize].to_string(),
        _ => String::new(),
    };
    match *v {
        MyValue::NULL => String::new(),
        MyValue::Bytes(ref b) => String::from_utf8(b.clone()).unwrap_or_default(),
        MyValue::Int(n) => n.to_string(),
        MyValue::UInt(n) => n.to_string(),
        MyValue::Float(x) => number_text(x as f64, true),
        MyValue::Double(x) => number_text(x, false),
        MyValue::Date(y, mo, d, h, mi, s, us) => match column.column_type() {
            ColumnType::MYSQL_TYPE_DATE | ColumnType::MYSQL_TYPE_NEWDATE => format!("{y:04}-{mo:02}-{d:02}"),
            _ => format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02}{}", fraction(us)),
        },
        MyValue::Time(negative, days, h, mi, s, us) => {
            format!("{}{:02}:{mi:02}:{s:02}{}", if negative { "-" } else { "" }, days * 24 + h as u32, fraction(us))
        }
    }
}

/// A FLOAT or DOUBLE as MySQL's text protocol writes it: the shortest
/// digits that read back the same (a FLOAT's at most 6), in full unless
/// that takes more than 15 characters (`1e15`, `1.2345678901234568e17`,
/// `5e-324`; but `100000000000000`, `0.0000000001`).
fn number_text(x: f64, float: bool) -> String {
    if x == 0.0 {
        return if x.is_sign_negative() { "-0" } else { "0" }.to_string();
    }
    let sci = if float { format!("{x:.5e}") } else { format!("{x:e}") };
    let (mantissa, exp) = sci.split_once('e').unwrap_or((&sci, "0"));
    let negative = mantissa.starts_with('-');
    let all: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let digits = match all.trim_end_matches('0') {
        "" => "0",
        d => d,
    };
    let (len, point) = (digits.len() as i64, exp.parse::<i64>().unwrap_or(0) + 1);
    // (MySQL's my_gcvt: the 'f' form when it fits the width, or keeps more
    // digits than the 'e' form would)
    let width = if negative { 14 } else { 15 };
    let exp_len = 1 + (point >= 101 || point <= -99) as i64 + (point >= 11 || point <= -9) as i64;
    let f_len = if point <= 0 { len - point + 2 } else if point < len { len + 1 } else { point };
    let force_e = point <= 0 && width <= 2 - point && width >= 3 + exp_len;
    let fixed = !force_e && (f_len <= width || (point <= width && (point >= -1 || (point == -2 && len > 1))));
    let sign = if negative { "-" } else { "" };
    if !fixed {
        let (first, rest) = digits.split_at(1);
        return format!("{sign}{first}{}{rest}e{}", if rest.is_empty() { "" } else { "." }, point - 1);
    }
    if point <= 0 {
        format!("{sign}0.{}{digits}", "0".repeat(-point as usize))
    } else if point < len {
        format!("{sign}{}.{}", &digits[..point as usize], &digits[point as usize..])
    } else {
        format!("{sign}{digits}{}", "0".repeat((point - len) as usize))
    }
}

/// A BASIC value as a statement's parameter: an integer, a number, text,
/// or NULL.
fn my_value(v: &Value) -> MyValue {
    match v {
        Value::Integer(n) => MyValue::Int(*n),
        Value::Double(x) => MyValue::Double(*x),
        Value::Boolean(_) => MyValue::Int(v.to_i64()),
        Value::Null => MyValue::NULL,
        other => MyValue::Bytes(other.to_string_val().into_bytes()),
    }
}

/// `Schema`: the current database's tables and views with their columns
/// and types (information_schema: a read-only connection's too), as JSON
/// (RCODEEDITOR's SetSchema takes it). "" when not connected, busy, or no
/// database is selected (OnError says why).
fn schema(host: &dyn Host, name: &str, key: &str) -> Value {
    let done = with_db(key, |db| {
        if let Some(busy) = background::refused(key) {
            return Err(busy);
        }
        let mut conn = db.conn()?;
        let text = |e: mysql::Error| e.to_string();
        // (two reads: with autocommit off they may begin a transaction)
        let a = Answers { rows: 2, last_rows: true, ..Answers::default() };
        let tables: Result<Vec<(String, String)>, _> =
            conn.query("SELECT TABLE_NAME, TABLE_TYPE FROM information_schema.TABLES WHERE TABLE_SCHEMA = DATABASE() ORDER BY TABLE_NAME");
        let columns: Result<Vec<(String, String, String)>, _> = conn.query(
            "SELECT TABLE_NAME, COLUMN_NAME, UPPER(COLUMN_TYPE) FROM information_schema.COLUMNS WHERE TABLE_SCHEMA = DATABASE() ORDER BY TABLE_NAME, ORDINAL_POSITION",
        );
        let seen = observe(&db.tx, &conn, &a);
        drop(conn);
        db.observed = Some(seen);
        let (tables, columns) = (tables.map_err(text)?, columns.map_err(text)?);
        Ok::<_, String>(
            tables
                .into_iter()
                .map(|(table, kind)| {
                    let cols = columns.iter().filter(|(t, _, _)| *t == table).map(|(_, c, ty)| (c.clone(), ty.clone())).collect();
                    let kind = if kind.contains("VIEW") { "view" } else { "table" }.to_string();
                    (table, kind, cols)
                })
                .collect::<Vec<_>>(),
        )
    });
    if let Some(tx) = with_db(key, |db| db.observed.take()).flatten() {
        publish_tx(host, name, tx);
    }
    match done {
        Some(Ok(tables)) => v_str(&crate::schema_json(&tables)),
        Some(Err(e)) => {
            host.report(&format!("[MySQL] Schema: {e}"));
            host.fire(name, "onerror", &[v_str(&e)]);
            v_str("")
        }
        None => v_str(""),
    }
}

/// `SelectDB(name)`: 1 if the server now uses that database.
fn select_db(host: &dyn Host, key: &str, args: &[Value]) -> Value {
    let db = args.first().map(|v| v.to_string_val()).unwrap_or_default();
    let done = DBS.with(|d| {
        let mut dbs = d.borrow_mut();
        let state = dbs.get_mut(key)?;
        if let Some(busy) = background::refused(key) {
            return Some(Err(busy));
        }
        // (the lock on a handle of its own: the component is set below)
        let shared = state.conn.clone();
        let got = shared.try_lock().map_err(|_| background::BUSY.to_string());
        let used = match got {
            Ok(mut c) => {
                let ran = c.query_drop(format!("USE `{}`", db.replace('`', "``")));
                let mut a = Answers::default();
                match &ran {
                    Ok(()) => a.set(false),
                    Err(e) => a.failed(e),
                }
                let seen = observe(&state.tx, &c, &a);
                drop(c);
                state.observed = Some(seen);
                ran.map_err(|e| e.to_string())
            }
            Err(e) => Err(e),
        };
        Some(used)
    });
    if let Some(tx) = with_db(key, |db| db.observed.take()).flatten() {
        publish_tx(host, &name_of(key), tx);
    }
    match done {
        Some(Ok(())) => v_int(1),
        Some(Err(e)) => {
            host.report(&format!("[MySQL] SelectDB error: {e}"));
            v_int(0)
        }
        None => v_int(0),
    }
}

// ---------------------------------------------------------------------------
// Background queries
// ---------------------------------------------------------------------------

/// A background query: the SQL, its values, the connection, what it
/// shares with the program's thread.
pub struct Job {
    sql: String,
    values: Vec<MyValue>,
    conn: Arc<Mutex<mysql::PooledConn>>,
    shared: Arc<Shared>,
    tx: Arc<Mutex<Tx>>,
}

impl Job {
    /// Runs it to its end on the worker thread.
    fn run(self) {
        let start = background::now();
        let end = match self.conn.try_lock() {
            Ok(mut conn) => {
                let mut a = Answers::default();
                let end = stream(&mut conn, &self.sql, self.values, &self.shared, &mut a);
                // (InTransaction and Autocommit as the server left them)
                let tx = observe(&self.tx, &conn, &a);
                self.shared.transaction(tx.value(), Some(tx.autocommit_value()));
                end
            }
            // (never: the component refuses a second query while one runs)
            Err(_) => End::Failed(background::BUSY.to_string()),
        };
        // (the connection given back before the program hears it's done)
        self.shared.end(end, background::ms_since(start));
    }
}

/// The query, its result sets' rows streamed into the shared rows in
/// chunks (a later set with columns starts them again). How it ended.
fn stream(conn: &mut mysql::PooledConn, sql: &str, values: Vec<MyValue>, shared: &Shared, a: &mut Answers) -> End {
    let failed = |e: mysql::Error, a: &mut Answers| {
        a.failed(&e);
        if shared.cancelled() {
            End::Cancelled
        } else {
            End::Failed(e.to_string())
        }
    };
    if values.is_empty() {
        match conn.query_iter(sql) {
            Ok(mut result) => stream_sets(&mut result, text_cell, shared, a),
            Err(e) => failed(e, a),
        }
    } else {
        let stmt = match conn.prep(sql) {
            Ok(stmt) => stmt,
            Err(e) => return failed(e, a),
        };
        if stmt.num_params() as usize != values.len() {
            return End::Failed(params::wrong_count(stmt.num_params() as usize, values.len()));
        }
        match conn.exec_iter(&stmt, mysql::Params::Positional(values)) {
            Ok(mut result) => stream_sets(&mut result, binary_cell, shared, a),
            Err(e) => failed(e, a),
        }
    }
}

fn stream_sets<T: Protocol>(result: &mut QueryResult<'_, '_, '_, T>, cell: fn(&MyValue, &Column) -> String, shared: &Shared, a: &mut Answers) -> End {
    let failed = |e: mysql::Error, a: &mut Answers| {
        a.failed(&e);
        if shared.cancelled() {
            End::Cancelled
        } else {
            End::Failed(e.to_string())
        }
    };
    while let Some(set) = result.iter() {
        let columns: Vec<Column> = set.columns().as_ref().to_vec();
        a.set(!columns.is_empty());
        if columns.is_empty() {
            continue;
        }
        let (names, types) = names_and_types(&columns);
        let mut chunk = Chunker::new(shared, names, types, background::slice());
        for row in set {
            let row = match row {
                Ok(row) => row,
                Err(e) => return failed(e, a),
            };
            for (i, column) in columns.iter().enumerate() {
                match row.as_ref(i) {
                    None | Some(MyValue::NULL) => chunk.push(i, None),
                    Some(v) => match bytes_cell(v, column, cell) {
                        (_, Some(bytes)) => chunk.push(i, Some(&String::from_utf8_lossy(&bytes))),
                        (text, None) => chunk.push(i, Some(&text)),
                    },
                }
            }
            if chunk.end_row() && shared.cancelled() {
                return End::Cancelled;
            }
        }
        chunk.flush();
    }
    if shared.cancelled() {
        End::Cancelled
    } else {
        End::Done
    }
}

/// `QueryAsync(sql [, values …])`: 1 if the query started (its rows and
/// end come as events), 0 if it didn't — not connected, or one still
/// running (OnError says which).
fn query_async(host: &dyn Host, name: &str, key: &str, args: &[Value]) -> Value {
    let (sql, values) = sql_and_params(key, args);
    let refused = background::refused(key).or_else(|| (!connected(key)).then(|| crate::NOT_CONNECTED.to_string()));
    if let Some(e) = refused {
        host.report(&format!("[MySQL] QueryAsync: {e}"));
        host.fire(name, "onerror", &[v_str(&e)]);
        return v_int(0);
    }
    let Some((conn, id, tx)) = with_db(key, |db| {
        db.results = ResultSet::framed(key);
        db.lengths.clear();
        (db.conn.clone(), db.id, db.tx.clone())
    }) else {
        return v_int(0);
    };
    let shared = Shared::new();
    let job = Job { sql, values: values.iter().map(my_value).collect(), conn, shared: shared.clone(), tx };
    // (Cancel: the server stops the statement — asked on a second
    // connection, away from the program's thread)
    let pool = POOLS.with(|p| p.borrow().get(key).cloned());
    let kill: Box<dyn Fn()> = Box::new(move || {
        if let Some(pool) = pool.clone() {
            let _ = std::thread::Builder::new().name("rapidr-mysql-kill".into()).spawn(move || {
                if let Ok(mut c) = pool.get_conn() {
                    let _ = c.query_drop(format!("KILL QUERY {id}"));
                }
            });
        }
    });
    background::begin(host, name, key, shared, Some(kill));
    let rejected = DBS.with(|d| {
        let mut dbs = d.borrow_mut();
        let Some(db) = dbs.get_mut(key) else { return Some(job) };
        let job = match &db.worker {
            Some(tx) => match tx.send(job) {
                Ok(()) => return None,
                Err(std::sync::mpsc::SendError(job)) => job,
            },
            None => job,
        };
        let (tx, rx) = std::sync::mpsc::channel::<Job>();
        let spawned = std::thread::Builder::new().name(format!("rapidr-mysql-{key}")).spawn(move || {
            for job in rx {
                job.run();
            }
        });
        if spawned.is_err() {
            return Some(job);
        }
        let sent = tx.send(job).err().map(|e| e.0);
        db.worker = Some(tx);
        sent
    });
    if let Some(job) = rejected {
        job.shared.end(End::Failed("the background query couldn't start (no thread)".into()), 0);
    }
    v_int(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_is_the_c_clients() {
        let s = "\0a'b\"c\\d\r\n\x1ae\u{e9}\u{20ac}";
        assert_eq!(escape(s), "\\0a\\'b\\\"c\\\\d\\r\\n\\Ze\u{e9}\u{20ac}");
        assert_eq!(escape(""), "");
        // (binary data: one character per byte, as LoadBlob reads it)
        assert_eq!(escape(&bytes_to_string(&[0, 0x27, 0xFF, 0x80])), "\\0\\'\u{ff}\u{80}");
    }

    #[test]
    fn escape_string_and_load_blob() {
        struct Quiet;
        impl Host for Quiet {
            fn set(&self, _: &str, _: &str, _: Value) {}
            fn fire(&self, _: &str, _: &str, _: &[Value]) {}
            fn report(&self, _: &str) {}
        }
        let call = |m: &str, args: &[Value]| method(&Quiet, "MySQL", m, args).to_string_val();
        assert_eq!(call("escapestring", &[v_str("a'b\0c"), v_int(5)]), "a\\'b\\0c");
        assert_eq!(call("escapestring", &[v_str("a'b\0c"), v_int(3)]), "a\\'b");
        assert_eq!(call("escapestring", &[v_str("a'b"), v_int(0)]), "");
        assert_eq!(call("escapestring", &[v_str("a'b"), v_int(99)]), "a\\'b");
        assert_eq!(call("escapestring", &[v_str("a'b")]), "a\\'b");
        let dir = std::env::temp_dir().join(format!("rapidr-db-blob-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("blob.bin");
        std::fs::write(&file, [0u8, b'\'', 0xFF, b'\n', b'x']).unwrap();
        assert_eq!(call("loadblob", &[v_str(file.to_str().unwrap())]), "\\0\\'\u{ff}\\nx");
        assert_eq!(call("loadblob", &[v_str(dir.join("none.bin").to_str().unwrap())]), "");
        std::fs::remove_dir_all(&dir).unwrap();
        // Not connected: nothing to read, nothing done.
        assert_eq!(call("rowblob", &[v_int(0), v_int(5)]), "");
        assert_eq!(call("fetchlengths", &[]), "0");
        assert_eq!(call("length", &[v_int(0)]), "0");
        assert_eq!(call("createdb", &[v_str("x")]), "0");
        assert_eq!(call("dropdb", &[v_str("x")]), "0");
        assert_eq!(call("refresh", &[v_int(0)]), "0");
        assert_eq!(call("refresh", &[v_int(1)]), "0");
        call("saveblob", &[v_int(0), v_str("")]);
    }

    #[test]
    fn server_statements() {
        assert_eq!(create_db_sql("shop"), "CREATE DATABASE `shop`");
        assert_eq!(drop_db_sql("a`b"), "DROP DATABASE `a``b`");
        assert_eq!(refresh_sql(0), Vec::<&str>::new());
        assert_eq!(refresh_sql(1 | 4), ["FLUSH PRIVILEGES", "FLUSH TABLES"]);
        assert_eq!(refresh_sql(31), ["FLUSH PRIVILEGES", "FLUSH LOGS", "FLUSH TABLES", "FLUSH HOSTS", "FLUSH STATUS"]);
        assert_eq!(refresh_sql(32 | 32768), Vec::<&str>::new());
    }

    #[test]
    fn real_connect_options() {
        let server = |host: &str, port: i64, socket: &str, flags: i64| Server {
            host: host.into(),
            user: "u".into(),
            password: "p".into(),
            db: "shop".into(),
            port,
            socket: socket.into(),
            flags,
            ssl: SslMode::Disabled,
            ssl_ca: String::new(),
        };
        let opts: mysql::Opts = options(&server("db.example", 12345, "", 0), 0).into();
        assert_eq!((opts.get_ip_or_hostname().to_string(), opts.get_tcp_port()), ("db.example".to_string(), 12345));
        assert_eq!((opts.get_user(), opts.get_pass(), opts.get_db_name()), (Some("u"), Some("p"), Some("shop")));
        assert_eq!((opts.get_socket(), opts.get_compress().is_some()), (None, false));
        // "" is this machine; port 0: the Port property, else 3306
        let opts: mysql::Opts = options(&server("", 0, "/tmp/mysql.sock", 0), 0).into();
        assert_eq!((opts.get_ip_or_hostname().to_string(), opts.get_tcp_port()), ("localhost".to_string(), 3306));
        assert_eq!(opts.get_socket(), Some("/tmp/mysql.sock"));
        let opts: mysql::Opts = options(&server("h", 0, "", 0), 3307).into();
        assert_eq!(opts.get_tcp_port(), 3307);
        // CLIENT_COMPRESS | CLIENT_FOUND_ROWS | CLIENT_SSL (SSL: not a
        // capability passed on — RealConnect turns it into SSLMode Required);
        // the session tracker always (InTransaction, Autocommit)
        let opts: mysql::Opts = options(&server("h", 0, "", 32 | 2 | 2048), 0).into();
        assert!(opts.get_compress().is_some());
        assert_eq!(opts.get_additional_capabilities(), CapabilityFlags::CLIENT_FOUND_ROWS | CapabilityFlags::CLIENT_SESSION_TRACK);
        assert!(opts.get_ssl_opts().is_none(), "Disabled: plain");
    }

    /// SSLMode's words, each mode's TLS options, the default, and what
    /// can't be (a stricter mode through a socket). Live servers:
    /// `connects_encrypted_to_a_local_server` (ignored).
    #[test]
    fn ssl_modes() {
        assert_eq!(SslMode::parse(""), Ok(SslMode::Preferred), "the default");
        assert_eq!(SslMode::parse("preferred"), Ok(SslMode::Preferred));
        assert_eq!(SslMode::parse("DISABLED"), Ok(SslMode::Disabled));
        assert_eq!(SslMode::parse(" Required "), Ok(SslMode::Required));
        assert_eq!(SslMode::parse("VERIFY_CA"), Ok(SslMode::VerifyCa));
        assert_eq!(SslMode::parse("verify-full"), Ok(SslMode::VerifyFull));
        assert_eq!(SslMode::parse("VERIFY_IDENTITY"), Ok(SslMode::VerifyFull), "MySQL's name for it");
        assert!(SslMode::parse("maybe").unwrap_err().contains("Disabled, Preferred, Required, VerifyCA, VerifyFull"));
        let mut s = Server { host: "db.example".into(), user: "u".into(), password: "p".into(), db: String::new(), port: 0, socket: String::new(), flags: 0, ssl: SslMode::Preferred, ssl_ca: String::new() };
        let tls = |s: &Server| -> Option<mysql::SslOpts> { mysql::Opts::from(options(s, 0)).get_ssl_opts().cloned() };
        // (Preferred and Required: encrypted, the certificate not checked)
        let t = tls(&s).expect("Preferred tries TLS first");
        assert!(t.accept_invalid_certs() && t.skip_domain_validation());
        s.ssl = SslMode::Required;
        assert!(tls(&s).is_some_and(|t| t.accept_invalid_certs()));
        // (VerifyCA: the chain checked, against SSLCA when given; VerifyFull: the name too)
        s.ssl = SslMode::VerifyCa;
        s.ssl_ca = "/certs/ca.pem".into();
        let t = tls(&s).unwrap();
        assert!(!t.accept_invalid_certs() && t.skip_domain_validation());
        assert_eq!(t.root_cert_path(), Some(std::path::Path::new("/certs/ca.pem")));
        s.ssl = SslMode::VerifyFull;
        let t = tls(&s).unwrap();
        assert!(!t.accept_invalid_certs() && !t.skip_domain_validation());
        s.ssl_ca.clear();
        assert_eq!(tls(&s).unwrap().root_cert_path(), None, "the system's authorities");
        s.ssl = SslMode::Disabled;
        assert!(tls(&s).is_none());
        // (a socket: no TLS; the stricter modes refuse it with the reason)
        s.socket = "/tmp/mysql.sock".into();
        s.ssl = SslMode::Preferred;
        assert!(tls(&s).is_none() && refused(&s).is_none());
        s.ssl = SslMode::Required;
        assert!(refused(&s).unwrap().contains("needs TCP"));
        // (the server without TLS, in plain words)
        let e = explain(&mysql::Error::DriverError(mysql::DriverError::TlsNotSupported), &s);
        assert!(e.contains("doesn't offer encrypted connections") && e.contains("Preferred"), "{e}");
    }

    /// A server without TLS (a mock on 127.0.0.1 that greets as MySQL 5.5
    /// does, without the SSL capability): Required gives the plain-words
    /// error, Preferred connects on (here: to the mock's next step, which
    /// isn't there — anything but the TLS error).
    #[test]
    fn a_server_without_tls() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for stream in listener.incoming().take(2) {
                let Ok(mut s) = stream else { return };
                // (protocol 10; CLIENT_PROTOCOL_41 | SECURE_CONNECTION |
                // PLUGIN_AUTH and LONG_PASSWORD, no CLIENT_SSL)
                let caps: u32 = 0x0001 | 0x0200 | 0x8000 | 0x0008_0000;
                let mut p = vec![10u8];
                p.extend_from_slice(b"5.5.0-mock\0");
                p.extend_from_slice(&7u32.to_le_bytes());
                p.extend_from_slice(b"abcdefgh\0");
                p.extend_from_slice(&(caps as u16).to_le_bytes());
                p.push(33);
                p.extend_from_slice(&2u16.to_le_bytes());
                p.extend_from_slice(&((caps >> 16) as u16).to_le_bytes());
                p.push(21);
                p.extend_from_slice(&[0u8; 10]);
                p.extend_from_slice(b"ijklmnopqrst\0");
                p.extend_from_slice(b"mysql_native_password\0");
                let mut packet = (p.len() as u32).to_le_bytes()[..3].to_vec();
                packet.push(0);
                packet.extend_from_slice(&p);
                let _ = s.write_all(&packet);
                let mut buf = [0u8; 256];
                let _ = s.read(&mut buf);
            }
        });
        let server = |ssl| Server { host: "127.0.0.1".into(), user: "u".into(), password: String::new(), db: String::new(), port: i64::from(port), socket: String::new(), flags: 0, ssl, ssl_ca: String::new() };
        let required = server(SslMode::Required);
        let e = mysql::Pool::new(options(&required, 0)).and_then(|p| p.get_conn().map(drop)).map(|_| String::new()).unwrap_or_else(|e| explain(&e, &required));
        assert!(e.contains("doesn't offer encrypted connections (TLS)"), "{e}");
        let plain = server(SslMode::Disabled);
        let e = mysql::Pool::new(options(&plain, 0)).and_then(|p| p.get_conn().map(drop)).map(|_| String::new()).unwrap_or_else(|e| explain(&e, &plain));
        assert!(!e.contains("TLS"), "Disabled never asks for TLS: {e}");
    }

    /// A live check against a MySQL or MariaDB server on this machine
    /// (RAPIDR_TEST_MYSQL=user@host:port, a user without a password, e.g.
    /// `rds_test@127.0.0.1:3306`): Required connects encrypted when the
    /// server has TLS, else says the server lacks it; Disabled is plain.
    /// `cargo test -p rapidr-db --features mysql -- --ignored encrypted`.
    #[test]
    #[ignore]
    fn connects_encrypted_to_a_local_server() {
        let Ok(spec) = std::env::var("RAPIDR_TEST_MYSQL") else { return };
        let (user, at) = spec.split_once('@').expect("user@host:port");
        let (h, port) = at.split_once(':').expect("host:port");
        let base = Server { host: h.into(), user: user.into(), password: String::new(), db: String::new(), port: port.parse().unwrap(), socket: String::new(), flags: 0, ssl: SslMode::Required, ssl_ca: String::new() };
        let cipher = |s: &Server| -> Result<String, String> {
            let pool = mysql::Pool::new(options(s, 0)).map_err(|e| explain(&e, s))?;
            let mut c = pool.get_conn().map_err(|e| explain(&e, s))?;
            let row: Option<(String, String)> = c.query_first("SHOW SESSION STATUS LIKE 'Ssl_cipher'").map_err(|e| e.to_string())?;
            assert_eq!(server_says_encrypted(&mut c), Some(row.as_ref().is_some_and(|r| !r.1.is_empty())));
            Ok(row.map(|r| r.1).unwrap_or_default())
        };
        match cipher(&base) {
            Ok(c) => {
                assert!(!c.is_empty(), "Required: a cipher in use");
                eprintln!("Required: encrypted ({c})");
                let full = cipher(&Server { ssl: SslMode::VerifyFull, ..base.clone() });
                eprintln!("VerifyFull, the system's authorities: {full:?}");
                assert!(full.is_err(), "a local server's own certificate isn't one the system trusts");
                assert_eq!(cipher(&Server { ssl: SslMode::Disabled, ..base.clone() }).unwrap(), "", "Disabled: plain");
            }
            Err(e) => {
                assert!(e.contains("doesn't offer encrypted connections"), "{e}");
                eprintln!("Required: {e}");
            }
        }
    }

    /// What MySQL 8's text protocol writes for these (checked on a server).
    #[test]
    fn numbers_as_the_text_protocol_writes_them() {
        let doubles = [
            (1.5, "1.5"), (1e15, "1e15"), (1e14, "100000000000000"), (1e16, "1e16"),
            (123456789012345678.0, "1.2345678901234568e17"), (0.0001, "0.0001"), (1e-5, "0.00001"),
            (1.5e-5, "0.000015"), (-2.5e30, "-2.5e30"), (3.0, "3"), (0.6666666666666666, "0.6666666666666666"),
            (1e100, "1e100"), (5e-324, "5e-324"), (1.7976931348623157e308, "1.7976931348623157e308"),
            (100.0, "100"), (1234567.0, "1234567"), (1e-6, "0.000001"), (1e-7, "0.0000001"),
            (999999999999999.0, "999999999999999"), (1234567890123456.0, "1.234567890123456e15"),
            (123456789012345.6, "123456789012345.6"), (0.1 + 0.2, "0.30000000000000004"),
            (1e-10, "0.0000000001"), (-1e-5, "-0.00001"), (-0.0, "-0"),
        ];
        for (x, text) in doubles {
            assert_eq!(number_text(x, false), text, "{x:e}");
        }
        let floats = [
            (0.1f32, "0.1"), (1e20, "1e20"), (1234567.0, "1234570"), (123456790.0, "123457000"),
            (1.5e-5, "0.000015"), (1e-5, "0.00001"), (123456.0, "123456"), (999999.5, "1000000"),
            (1.25e-7, "0.000000125"), (std::f32::consts::PI, "3.14159"),
        ];
        for (x, text) in floats {
            assert_eq!(number_text(x as f64, true), text, "{x:e}");
        }
    }

    /// A column's type from the metadata every result set carries (a
    /// read-only connection's too: no table made to learn it).
    #[test]
    fn column_types_from_the_metadata() {
        use ColumnType::*;
        let none = ColumnFlags::empty();
        let unsigned = ColumnFlags::UNSIGNED_FLAG;
        let utf8 = 255;
        let cases = [
            (MYSQL_TYPE_LONG, none, 63, "INT"),
            (MYSQL_TYPE_LONG, unsigned, 63, "INT UNSIGNED"),
            (MYSQL_TYPE_LONGLONG, unsigned, 63, "BIGINT UNSIGNED"),
            (MYSQL_TYPE_TINY, none, 63, "TINYINT"),
            (MYSQL_TYPE_NEWDECIMAL, none, 63, "DECIMAL"),
            (MYSQL_TYPE_DOUBLE, none, 63, "DOUBLE"),
            (MYSQL_TYPE_VAR_STRING, none, utf8, "VARCHAR"),
            (MYSQL_TYPE_VAR_STRING, none, 63, "VARBINARY"),
            (MYSQL_TYPE_STRING, none, utf8, "CHAR"),
            (MYSQL_TYPE_STRING, ColumnFlags::ENUM_FLAG, utf8, "ENUM"),
            (MYSQL_TYPE_STRING, ColumnFlags::SET_FLAG, utf8, "SET"),
            (MYSQL_TYPE_BLOB, none, utf8, "TEXT"),
            (MYSQL_TYPE_BLOB, none, 63, "BLOB"),
            (MYSQL_TYPE_LONG_BLOB, none, utf8, "LONGTEXT"),
            (MYSQL_TYPE_DATETIME, none, 63, "DATETIME"),
            (MYSQL_TYPE_DATE, none, 63, "DATE"),
            (MYSQL_TYPE_TIMESTAMP, none, 63, "TIMESTAMP"),
            (MYSQL_TYPE_JSON, none, 63, "JSON"),
            (MYSQL_TYPE_NULL, none, 63, "NULL"),
        ];
        for (t, flags, charset, name) in cases {
            assert_eq!(type_name(t, flags, charset), name, "{t:?}");
        }
    }

    /// A host that records what the component did (the live tests').
    #[derive(Default)]
    struct Log(std::cell::RefCell<Vec<String>>);
    impl Host for Log {
        fn set(&self, _: &str, _: &str, _: Value) {}
        fn get(&self, _: &str, prop: &str) -> Value {
            v_str(if prop == "port" { "0" } else { "" })
        }
        fn fire(&self, _: &str, event: &str, args: &[Value]) {
            let args: Vec<String> = args.iter().map(|a| a.to_string_val()).collect();
            self.0.borrow_mut().push(format!("{event}({})", args.join(",")));
        }
        fn report(&self, _: &str) {}
    }

    /// The tests' own MySQL server, when there is one: RAPIDR_TEST_MYSQL =
    /// `host:port/database/user` (no password; a server on this computer
    /// only). The tests make nothing that lasts: a TEMPORARY table, gone
    /// when the connection closes.
    fn test_server() -> Option<(String, i64, String, String)> {
        let spec = std::env::var("RAPIDR_TEST_MYSQL").ok()?;
        let (hostport, rest) = spec.split_once('/')?;
        let (db, user) = rest.split_once('/')?;
        let (host, port) = hostport.split_once(':').unwrap_or((hostport, "3306"));
        assert!(matches!(host, "127.0.0.1" | "localhost" | "::1"), "RAPIDR_TEST_MYSQL: a server on this computer only");
        Some((host.to_string(), port.parse().ok()?, db.to_string(), user.to_string()))
    }

    /// Names and types come from the server's metadata, for Query and for
    /// QueryAsync; a background query streams, Cancel kills it.
    #[test]
    fn live_names_types_and_background_queries() {
        let Some((host, port, db, user)) = test_server() else {
            eprintln!("(no RAPIDR_TEST_MYSQL: the live MySQL test is skipped)");
            return;
        };
        let h = Log::default();
        let call = |m: &str, args: &[Value]| method(&h, "My", m, args).to_string_val();
        call("realconnect", &[v_str(&host), v_str(&user), v_str(""), v_str(&db), v_int(port), v_str(""), v_int(0)]);
        assert!(connected("my"), "{:?}", h.0.borrow());
        assert_eq!(call("query", &[v_str("CREATE TEMPORARY TABLE rr_types (id INT UNSIGNED PRIMARY KEY, name VARCHAR(40), price DECIMAL(8,2), made DATETIME, note TEXT, data BLOB, kind ENUM('a','b'))")]), "1");
        assert_eq!(call("query", &[v_str("INSERT INTO rr_types VALUES (1, 'Ann', 2.50, '2026-10-22 12:00:00', NULL, NULL, 'a')")]), "1");
        // Query: the names and types of the result's columns
        call("query", &[v_str("SELECT id, name AS who, price, made, note, data, kind, 1 + 1 AS two FROM rr_types")]);
        let cols = |what: &str| (0..8).map(|i| call(what, &[v_int(i)])).collect::<Vec<_>>();
        assert_eq!(cols("colname"), ["id", "who", "price", "made", "note", "data", "kind", "two"]);
        assert_eq!(cols("coltype"), ["INT UNSIGNED", "VARCHAR", "DECIMAL", "DATETIME", "TEXT", "BLOB", "ENUM", "BIGINT"]);
        assert_eq!((call("colname", &[v_int(8)]), call("colname", &[v_int(-1)])), (String::new(), String::new()));
        // QueryAsync: the same names, rows streamed, OnDone
        call("query", &[v_str("SET SESSION cte_max_recursion_depth = 1000000")]);
        h.0.borrow_mut().clear();
        assert_eq!(call("queryasync", &[v_str("WITH RECURSIVE c(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM c WHERE i < ?) SELECT i AS n, CONCAT('row ', i) AS label, t.id AS id, t.id AS id FROM c, rr_types t"), v_int(50000)]), "1");
        let settle = || {
            let start = std::time::Instant::now();
            while background::poll(&h) {
                assert!(start.elapsed().as_secs() < 30, "a background query never ended");
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
        };
        settle();
        let log = h.0.borrow().join("\n");
        assert!(log.contains("ondone(50000,"), "{log}");
        assert_eq!(cols("colname")[..4], ["n", "label", "id", "id"]);
        assert_eq!(cols("coltype")[..4], ["BIGINT", "VARCHAR", "INT UNSIGNED", "INT UNSIGNED"]);
        call("rowseek", &[v_int(49999)]);
        call("fetchrow", &[]);
        assert_eq!((call("row", &[v_int(0)]), call("row", &[v_int(1)])), ("50000".into(), "row 50000".into()));
        // Cancel: the server stops the statement (KILL QUERY)
        h.0.borrow_mut().clear();
        call("queryasync", &[v_str("SELECT SLEEP(20) AS z")]);
        std::thread::sleep(std::time::Duration::from_millis(300));
        let start = std::time::Instant::now();
        assert_eq!(call("cancel", &[]), "1");
        settle();
        assert!(start.elapsed().as_secs() < 10, "Cancel took {:?}", start.elapsed());
        assert!(h.0.borrow().iter().any(|e| e == "onerror(cancelled)"), "{:?}", h.0.borrow());
        // Schema: the database's tables (the TEMPORARY one isn't listed by
        // information_schema; a JSON object of tables)
        let json = call("schema", &[]);
        assert!(json.starts_with("{\"tables\":["), "{json}");
        // and the connection still answers
        assert_eq!(call("query", &[v_str("SELECT 41 + 1 AS answer")]), "1");
        assert_eq!(call("colname", &[v_int(0)]), "answer");
        call("close", &[]);
    }
}

#[cfg(test)]
mod transaction_tests {
    use super::*;
    use std::cell::RefCell;

    /// A host that keeps the properties the component sets, and gives it
    /// Host / User / Database (Connect's arguments are read from them).
    #[derive(Default)]
    struct Props(RefCell<HashMap<String, Value>>);
    impl Host for Props {
        fn set(&self, _: &str, prop: &str, value: Value) {
            self.0.borrow_mut().insert(prop.to_string(), value);
        }
        fn get(&self, _: &str, prop: &str) -> Value {
            self.0.borrow().get(prop).cloned().unwrap_or_else(v_null)
        }
        fn fire(&self, _: &str, _: &str, _: &[Value]) {}
        fn report(&self, _: &str) {}
    }

    /// The tracker's rules without a server: what each kind of answer
    /// leaves InTransaction at.
    #[test]
    fn what_an_answer_leaves_unsaid() {
        let rows = |n: usize, last: bool, oks: usize| Answers { oks, rows: n, last_rows: last, error: None };
        // (no news in the last answer)
        let after = |mut t: Tx, a: Answers| {
            t.heard((None, None), &a);
            t.value()
        };
        let none = Tx { tracked: true, open: Some(false), autocommit: Some(true) };
        let open = Tx { open: Some(true), ..none };
        // one SELECT: nothing changes
        assert_eq!(after(none, rows(1, true, 0)), 0);
        assert_eq!(after(open, rows(1, true, 0)), 1);
        // BEGIN; SELECT — BEGIN's answer unseen: can't tell
        assert_eq!(after(none, rows(1, true, 1)), -1);
        // INSERT; INSERT (no news in the last): the first's unseen
        assert_eq!(after(none, rows(0, false, 2)), -1);
        // a failed first statement: as it was; a deadlock rolled it back
        assert_eq!(after(open, Answers { error: Some(1062), ..Answers::default() }), 1);
        assert_eq!(after(open, Answers { error: Some(ER_LOCK_DEADLOCK), ..Answers::default() }), 0);
        // autocommit off: a read may begin one
        assert_eq!(after(Tx { autocommit: Some(false), ..none }, rows(1, true, 0)), -1);
        // a server that doesn't track: can't tell
        assert_eq!(after(Tx { tracked: false, ..none }, rows(0, false, 1)), -1);
        // the last answer's news decides; autocommit's follows
        let mut t = none;
        t.heard((Some(true), None), &rows(1, true, 3));
        assert_eq!((t.value(), t.autocommit_value()), (1, 1));
        t.heard((Some(false), Some(false)), &rows(0, false, 1));
        assert_eq!((t.value(), t.autocommit_value()), (0, 0));
    }

    /// InTransaction and Autocommit against a local MySQL or MariaDB server
    /// (RAPIDR_TEST_MYSQL=user@host:port, a user without a password, e.g.
    /// `rds_test@127.0.0.1:3306`, its database of the same name; only
    /// TEMPORARY tables and one table made and dropped here):
    /// `cargo test -p rapidr-db --features mysql -- --ignored in_transaction`.
    #[test]
    #[ignore]
    fn in_transaction_on_a_local_server() {
        let Ok(spec) = std::env::var("RAPIDR_TEST_MYSQL") else { return };
        let (user, at) = spec.split_once('@').expect("user@host:port");
        let (h, port) = at.split_once(':').expect("host:port");
        let host = Props::default();
        host.set("", "port", v_int(port.parse().unwrap()));
        let call = |m: &str, args: &[Value]| method(&host, "My", m, args).to_i64();
        let q = |sql: &str| call("query", &[v_str(sql)]);
        let tx = || host.get("", "intransaction").to_i64();
        let auto = || host.get("", "autocommit").to_i64();
        assert_eq!(method(&host, "My", "realconnect", &[v_str(h), v_str(user), v_str(""), v_str(user), v_int(0), v_str(""), v_int(0)]).to_i64(), 0);
        assert_eq!(host.get("", "connected").to_i64(), 1, "connected to {spec}");
        assert_eq!((tx(), auto()), (0, 1));
        assert_eq!(q("CREATE TEMPORARY TABLE rapidr_tx (id INT PRIMARY KEY, v VARCHAR(10)) ENGINE=InnoDB"), 1);
        assert_eq!(tx(), 0);
        // BEGIN … COMMIT, rows read inside
        q("BEGIN");
        assert_eq!(tx(), 1);
        q("INSERT INTO rapidr_tx VALUES (1, 'a')");
        q("SELECT * FROM rapidr_tx");
        assert_eq!(tx(), 1, "a read inside it");
        q("COMMIT");
        assert_eq!(tx(), 0);
        // START TRANSACTION and a row in one query (the last answer says)
        q("START TRANSACTION; INSERT INTO rapidr_tx VALUES (2, 'b')");
        assert_eq!(tx(), 1);
        q("ROLLBACK");
        assert_eq!(tx(), 0);
        // several statements whose last gives rows: can't tell (-1) until
        // a statement's answer says it again
        q("BEGIN; SELECT 1");
        assert_eq!(tx(), -1);
        q("COMMIT");
        assert_eq!(tx(), 0);
        // savepoints inside a transaction
        q("BEGIN");
        q("SAVEPOINT s1");
        q("INSERT INTO rapidr_tx VALUES (3, 'c')");
        q("ROLLBACK TO SAVEPOINT s1");
        q("RELEASE SAVEPOINT s1");
        assert_eq!(tx(), 1);
        // a failed statement inside it: still open
        assert_eq!(q("INSERT INTO rapidr_tx VALUES (1, 'dup')"), 0);
        assert_eq!(tx(), 1);
        q("COMMIT");
        assert_eq!(tx(), 0);
        // autocommit off: the first change opens one implicitly; DDL commits it
        q("SET autocommit = 0");
        assert_eq!((tx(), auto()), (0, 0));
        q("INSERT INTO rapidr_tx VALUES (4, 'd')");
        assert_eq!(tx(), 1);
        q("CREATE TABLE rapidr_dbd_tx_probe (x INT)");
        assert_eq!(tx(), 0, "MySQL's implicit commit after DDL");
        q("DROP TABLE rapidr_dbd_tx_probe");
        assert_eq!(tx(), 0);
        // … a read may have begun one (can't tell), a change says
        q("SELECT * FROM rapidr_tx");
        assert_eq!(tx(), -1);
        q("INSERT INTO rapidr_tx VALUES (5, 'e')");
        assert_eq!(tx(), 1);
        // SET autocommit = 1 commits it
        q("SET autocommit = 1");
        assert_eq!((tx(), auto()), (0, 1));
        // a background query says it too (before OnDone)
        let wait = || {
            let start = std::time::Instant::now();
            while background::poll(&host) {
                assert!(start.elapsed().as_secs() < 20);
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        };
        call("queryasync", &[v_str("BEGIN")]);
        wait();
        assert_eq!(tx(), 1);
        call("queryasync", &[v_str("SELECT * FROM rapidr_tx")]);
        wait();
        assert_eq!(tx(), 1);
        call("queryasync", &[v_str("ROLLBACK")]);
        wait();
        assert_eq!(tx(), 0);
        // CreateDB's DDL commits too (refused here: no rights — as it was)
        q("BEGIN");
        assert_eq!(tx(), 1);
        call("close", &[]);
        assert_eq!(tx(), 0);
    }
}
