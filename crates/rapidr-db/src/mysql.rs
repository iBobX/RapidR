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

use std::cell::RefCell;
use std::collections::HashMap;

use mysql::consts::{CapabilityFlags, ColumnType};
use mysql::prelude::{Protocol, Queryable};
use mysql::{Column, QueryResult, Value as MyValue};
use rapidr_value::objects::codec::bytes_to_string;
use rapidr_value::{v_int, v_null, v_str, Value};

use crate::{cursor_method, param_method, params, publish, sql_and_params, Host, ResultSet};

struct Db {
    conn: mysql::PooledConn,
    results: ResultSet,
    databases: Vec<String>,
    /// What the last FetchLengths found (Length(i)): the byte lengths of
    /// that row's cells.
    lengths: Vec<i64>,
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
            let server = Server {
                host: text(0),
                user: text(1),
                password: text(2),
                db: text(3),
                port: int(4),
                socket: text(5),
                flags: int(6),
            };
            open(host, name, &key, server);
            return v_null();
        }
        "close" | "disconnect" => return close(host, name, &key),
        "query" | "execute" => return query(host, name, &key, args),
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

/// `f(db)` on the component's connection (None: not connected).
fn with_db<T>(key: &str, f: impl FnOnce(&mut Db) -> T) -> Option<T> {
    DBS.with(|d| d.borrow_mut().get_mut(key).map(f))
}

fn connected(key: &str) -> bool {
    with_db(key, |_| ()).is_some()
}

/// Where Connect and RealConnect go.
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
}

/// `Connect(host, user, password [, database])`, each missing one from
/// the component's property (Host, User, Password, DB; Port).
fn connect(host: &dyn Host, name: &str, key: &str, args: &[Value]) -> Value {
    let arg = |i: usize, prop: &str| args.get(i).map(|v| v.to_string_val()).unwrap_or_else(|| host.get(name, prop).to_string_val());
    let server = Server {
        host: arg(0, "host"),
        user: arg(1, "user"),
        password: arg(2, "password"),
        db: arg(3, "db"),
        port: 0,
        socket: String::new(),
        flags: 0,
    };
    v_int(open(host, name, key, server) as i64)
}

/// The C client's flags RealConnect passes on: FOUND_ROWS (2), LONG_FLAG
/// (4), NO_SCHEMA (16), ODBC (64), IGNORE_SPACE (256), INTERACTIVE (1024).
/// COMPRESS (32) turns compression on; the rest are the connection's own
/// business (the `mysql` crate sets them) or need TLS, which RapidR's
/// client doesn't have (SSL, 2048).
const PASSED_FLAGS: u32 = 2 | 4 | 16 | 64 | 256 | 1024;
const CLIENT_COMPRESS: i64 = 32;

/// The connection options for `server`: "" is this machine (as the C
/// client's NULL host), port 0 the component's Port or MySQL's 3306.
fn options(server: &Server, port_property: i64) -> mysql::OptsBuilder {
    let host = if server.host.is_empty() { "localhost" } else { server.host.as_str() };
    let port = [server.port, port_property].into_iter().find(|p| (1..=65535).contains(p)).unwrap_or(3306) as u16;
    let mut opts = mysql::OptsBuilder::new()
        .ip_or_hostname(Some(host))
        .user(Some(&server.user))
        .pass(Some(&server.password))
        .tcp_port(port)
        .additional_capabilities(CapabilityFlags::from_bits_truncate(server.flags as u32 & PASSED_FLAGS));
    if !server.db.is_empty() {
        opts = opts.db_name(Some(&server.db));
    }
    if !server.socket.is_empty() {
        opts = opts.socket(Some(&server.socket));
    }
    if server.flags & CLIENT_COMPRESS != 0 {
        opts = opts.compress(Some(mysql::Compression::default()));
    }
    opts
}

/// Connects (Connect, RealConnect): whether it did. Connected, DB(i),
/// DBCount and OnConnect follow, or OnError with the message.
fn open(host: &dyn Host, name: &str, key: &str, server: Server) -> bool {
    let opts = options(&server, host.get(name, "port").to_i64());
    let opened = mysql::Pool::new(opts).and_then(|pool| Ok((pool.get_conn()?, pool)));
    match opened {
        Ok((mut conn, pool)) => {
            // (the databases the server has: DB(i), DBCount)
            let databases: Vec<String> = conn.query("SHOW DATABASES").unwrap_or_default();
            let count = databases.len() as i64;
            let db = Db { conn, results: ResultSet::default(), databases, lengths: Vec::new() };
            DBS.with(|d| d.borrow_mut().insert(key.to_string(), db));
            POOLS.with(|p| p.borrow_mut().insert(key.to_string(), pool));
            host.set(name, "connected", v_int(1));
            host.set(name, "dbcount", v_int(count));
            host.row_changed(name);
            host.fire(name, "onconnect", &[]);
            true
        }
        Err(e) => {
            host.report(&format!("[MySQL] Connection error: {e}"));
            host.set(name, "connected", v_int(0));
            host.fire(name, "onerror", &[v_str(&e.to_string())]);
            false
        }
    }
}

/// A statement that changes the server (CreateDB, DropDB, Refresh's
/// FLUSH): 1 if it ran, else 0 (and OnError, when connected). After
/// CreateDB / DropDB (`databases`) the server's list of databases is read
/// again: DB(i), DBCount.
fn server_command(host: &dyn Host, name: &str, key: &str, sql: &str, databases: bool) -> Value {
    let done = with_db(key, |db| {
        db.conn.query_drop(sql)?;
        if databases {
            db.databases = db.conn.query("SHOW DATABASES")?;
        }
        Ok::<_, mysql::Error>(db.databases.len() as i64)
    });
    match done {
        Some(Ok(count)) => {
            if databases {
                host.set(name, "dbcount", v_int(count));
            }
            v_int(1)
        }
        Some(Err(e)) => {
            host.report(&format!("[MySQL] {sql}: {e}"));
            host.fire(name, "onerror", &[v_str(&e.to_string())]);
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
    let bytes = with_db(key, |db| db.results.cell_bytes(field).map(<[u8]>::to_vec)).flatten();
    if let Some(bytes) = bytes {
        if let Err(e) = std::fs::write(file, bytes) {
            host.report(&format!("[MySQL] SaveBlob {file}: {e}"));
            host.fire(name, "onerror", &[v_str(&format!("{file}: {e}"))]);
        }
    }
}

fn close(host: &dyn Host, name: &str, key: &str) -> Value {
    let closed = DBS.with(|d| d.borrow_mut().remove(key));
    POOLS.with(|p| p.borrow_mut().remove(key));
    host.set(name, "connected", v_int(0));
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
    let outcome = DBS.with(|d| {
        let mut dbs = d.borrow_mut();
        let db = dbs.get_mut(key).ok_or_else(|| crate::NOT_CONNECTED.to_string())?;
        let text = |e: mysql::Error| e.to_string();
        let table = if values.is_empty() {
            let mut result = db.conn.query_iter(&sql).map_err(text)?;
            collect(&mut result, text_cell)?
        } else {
            let stmt = db.conn.prep(&sql).map_err(text)?;
            if stmt.num_params() as usize != values.len() {
                return Err(params::wrong_count(stmt.num_params() as usize, values.len()));
            }
            let values: Vec<MyValue> = values.iter().map(my_value).collect();
            let mut result = db.conn.exec_iter(&stmt, mysql::Params::Positional(values)).map_err(text)?;
            collect(&mut result, binary_cell)?
        };
        Ok::<_, String>(table.map(|(columns, rows, raw)| {
            db.results = ResultSet::with_raw(columns, rows, raw);
            db.lengths.clear();
            (db.results.rows.len(), db.results.columns.len())
        }))
    });
    let ok = match outcome {
        Ok(size) => {
            if let Some(size) = size {
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

/// A result set: its columns' names, its rows' text, and the bytes of the
/// cells that aren't text ([`ResultSet::raw`]).
type Rows = (Vec<String>, Vec<Vec<String>>, Vec<Vec<Option<Vec<u8>>>>);

/// Every result set of a query read to its end: the last that had columns.
fn collect<T: Protocol>(result: &mut QueryResult<'_, '_, '_, T>, cell: fn(&MyValue, &Column) -> String) -> Result<Option<Rows>, String> {
    let mut last = None;
    while let Some(set) = result.iter() {
        let columns: Vec<Column> = set.columns().as_ref().to_vec();
        let (mut rows, mut raw) = (Vec::new(), Vec::new());
        for row in set {
            let row = row.map_err(|e| e.to_string())?;
            let cells: Vec<(String, Option<Vec<u8>>)> =
                columns.iter().enumerate().map(|(i, c)| row.as_ref(i).map(|v| bytes_cell(v, c, cell)).unwrap_or_default()).collect();
            let binary = cells.iter().any(|(_, b)| b.is_some());
            let (texts, bytes): (Vec<String>, Vec<Option<Vec<u8>>>) = cells.into_iter().unzip();
            rows.push(texts);
            raw.push(if binary { bytes } else { Vec::new() });
        }
        if !columns.is_empty() {
            if raw.iter().all(Vec::is_empty) {
                raw.clear();
            }
            last = Some((columns.iter().map(|c| c.name_str().into_owned()).collect(), rows, raw));
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

/// `SelectDB(name)`: 1 if the server now uses that database.
fn select_db(host: &dyn Host, key: &str, args: &[Value]) -> Value {
    let db = args.first().map(|v| v.to_string_val()).unwrap_or_default();
    let done = DBS.with(|d| {
        let mut dbs = d.borrow_mut();
        let state = dbs.get_mut(key)?;
        Some(state.conn.query_drop(format!("USE `{}`", db.replace('`', "``"))))
    });
    match done {
        Some(Ok(())) => v_int(1),
        Some(Err(e)) => {
            host.report(&format!("[MySQL] SelectDB error: {e}"));
            v_int(0)
        }
        None => v_int(0),
    }
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
        // CLIENT_COMPRESS | CLIENT_FOUND_ROWS | CLIENT_SSL (no TLS: dropped)
        let opts: mysql::Opts = options(&server("h", 0, "", 32 | 2 | 2048), 0).into();
        assert!(opts.get_compress().is_some());
        assert_eq!(opts.get_additional_capabilities(), CapabilityFlags::CLIENT_FOUND_ROWS);
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
}
