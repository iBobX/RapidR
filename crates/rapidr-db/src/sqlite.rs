//! RSQLITE: SQLite itself on every runtime (rusqlite; on the web its wasm
//! build, sqlite-wasm-rs).
//!
//! `DB.Query(sql)` runs the statements of `sql` in order. A statement that
//! has result columns (SELECT, PRAGMA, WITH … SELECT, EXPLAIN, INSERT …
//! RETURNING) gives the component its rows — the last such statement's —
//! and RowCount / ColCount / FieldCount; the others leave the rows of the
//! last query that had some. Values come as text the way Row(i) has always
//! given them: NULL as "", integers in full, reals as Rust writes them
//! (2.5, 3, 0.1, 100000000000000000000).
//!
//! The values after the SQL (and those queued with AddParam) are bound to
//! its placeholders in order. Their counts must be the same — checked
//! before the statement runs — so a `?` in a query given no values is the
//! error it always was, never a NULL. (SQL of several statements takes the
//! values statement after statement: one whose placeholders they don't
//! fill isn't run; values left over are an error once the others ran.)
//!
//! `DB.QueryAsync(sql [, values …])` runs it in the background
//! ([`crate::background`]): on the desktop on the connection's worker
//! thread, on the web in slices of the page's thread ([`take_pending`]);
//! the rows stream into the frame under the component's name, `Cancel`
//! interrupts it.
//!
//! On the web the page has no file system: `Connect(name)` opens a
//! database kept in memory for the page's session (SQLite's memory VFS),
//! the project's file of that name (a `.db` asset) copied in first. Nothing
//! is saved to the browser's storage yet.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::future::Future;
use std::sync::{Arc, Mutex, MutexGuard};

use rapidr_value::{v_int, v_null, v_str, Value};
use rusqlite::fallible_iterator::FallibleIterator;
use rusqlite::types::{Value as SqlValue, ValueRef};
use rusqlite::{Batch, Connection, InterruptHandle, Statement};

use crate::background::{self, Chunker, End, Shared};
use crate::{cursor_method, param_method, params, publish, sql_and_params, Host, ResultSet};

struct Db {
    /// The connection, shared with a background query while it runs (it
    /// holds the lock then: a query meanwhile is refused, never waited for).
    conn: Arc<Mutex<Connection>>,
    /// Stops what the connection is running (Cancel), from any thread.
    interrupt: Arc<InterruptHandle>,
    results: ResultSet,
    /// Where each result column came from (table, column): None for an
    /// expression. The web's bound widgets write their edits back there.
    origins: Vec<Option<(String, String)>>,
    /// The connection's worker thread (the desktop's background queries),
    /// made by the first QueryAsync; it ends when the component closes.
    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    worker: Option<std::sync::mpsc::Sender<Job>>,
}

impl Db {
    fn new(conn: Connection) -> Db {
        let interrupt = Arc::new(conn.get_interrupt_handle());
        Db {
            conn: Arc::new(Mutex::new(conn)),
            interrupt,
            results: ResultSet::default(),
            origins: Vec::new(),
            #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
            worker: None,
        }
    }

    /// The connection for a query now: Err while a background query has it.
    fn conn(&self) -> Result<MutexGuard<'_, Connection>, String> {
        self.conn.try_lock().map_err(|_| background::BUSY.to_string())
    }
}

thread_local! {
    /// The open databases, by component (lowercase name).
    static DBS: RefCell<HashMap<String, Db>> = RefCell::new(HashMap::new());
}

/// A statement's rows.
struct Table {
    columns: Vec<String>,
    /// Their declared types ("" for an expression).
    types: Vec<String>,
    rows: Vec<Vec<String>>,
    origins: Vec<Option<(String, String)>>,
}

/// `name.method(args)` of an RSQLITE (`method` lowercase).
pub fn method(host: &dyn Host, name: &str, method: &str, args: &[Value]) -> Value {
    let key = name.to_lowercase();
    if let Some(v) = param_method(&key, method, args) {
        return v;
    }
    match method {
        "connect" | "open" => return connect(host, name, &key, args),
        "close" | "disconnect" => return close(host, name, &key),
        "query" | "exec" => return query(host, name, &key, args),
        "queryasync" => return query_async(host, name, &key, args),
        "cancel" => return v_int(background::cancel(&key) as i64),
        "queryscalar" | "query_scalar" | "scalar" => return query_scalar(host, name, &key, args),
        "escapestring" => {
            let s = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            return v_str(&s.replace('\'', "''"));
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
            host.report(&format!("[WARN] RSQLite.{method}() not implemented"));
            v_null()
        }
    }
}

/// `Connect(file)`: 1 if the database was there already, 0 if it's new
/// (or the connection failed: Connected says which).
fn connect(host: &dyn Host, name: &str, key: &str, args: &[Value]) -> Value {
    let path = args.first().map(|v| v.to_string_val()).unwrap_or_default();
    // (a background query on the connection it had stops)
    background::forget(key);
    match open(host, &path) {
        Ok((conn, existed)) => {
            let db = Db::new(conn);
            // (a connection the component had closes)
            let old = DBS.with(|d| d.borrow_mut().insert(key.to_string(), db));
            drop(old);
            host.set(name, "connected", v_int(1));
            host.set(name, "db", v_str(&path));
            host.row_changed(name);
            host.fire(name, "onconnect", &[]);
            v_int(existed as i64)
        }
        Err(e) => {
            host.report(&format!("[SQLite] Connect error: {e}"));
            host.set(name, "connected", v_int(0));
            host.fire(name, "onerror", &[v_str(&e)]);
            v_int(0)
        }
    }
}

fn is_memory(path: &str) -> bool {
    path.is_empty() || path == ":memory:"
}

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
fn open(_host: &dyn Host, path: &str) -> Result<(Connection, bool), String> {
    if is_memory(path) {
        return Connection::open_in_memory().map(|c| (c, false)).map_err(|e| e.to_string());
    }
    let existed = std::path::Path::new(path).exists();
    Connection::open(path).map(|c| (c, existed)).map_err(|e| e.to_string())
}

/// The web: a database the page opened before this session is opened
/// again; the first time, the project's file of that name is copied in.
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
fn open(host: &dyn Host, path: &str) -> Result<(Connection, bool), String> {
    use rusqlite::{OpenFlags, MAIN_DB};
    let text = |e: rusqlite::Error| e.to_string();
    if is_memory(path) {
        return Connection::open_in_memory().map(|c| (c, false)).map_err(text);
    }
    let again = OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_URI | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    if let Ok(conn) = Connection::open_with_flags(path, again) {
        return Ok((conn, true));
    }
    let mut conn = Connection::open(path).map_err(text)?;
    let Some(bytes) = host.file_bytes(path) else { return Ok((conn, false)) };
    let mut file = Connection::open_in_memory().map_err(text)?;
    file.deserialize_read_exact(MAIN_DB, bytes.as_slice(), bytes.len(), true)
        .map_err(|e| format!("{path} is not a database SQLite can read ({e})"))?;
    // (all its pages in one step: a step that waited would sleep, which a
    // page can't)
    rusqlite::backup::Backup::new(&file, &mut conn).and_then(|b| b.step(-1)).map_err(text)?;
    Ok((conn, true))
}

fn close(host: &dyn Host, name: &str, key: &str) -> Value {
    background::forget(key);
    let closed = DBS.with(|d| d.borrow_mut().remove(key));
    host.set(name, "connected", v_int(0));
    host.set(name, "busy", v_int(0));
    if closed.is_some() {
        drop(closed);
        host.fire(name, "ondisconnect", &[]);
    }
    v_null()
}

/// `Query(sql [, values …])`: 1 if every statement ran, else 0 (OnError
/// says why). OnQueryDone follows either way.
fn query(host: &dyn Host, name: &str, key: &str, args: &[Value]) -> Value {
    let (sql, values) = sql_and_params(key, args);
    let outcome = DBS.with(|d| {
        let mut dbs = d.borrow_mut();
        let db = dbs.get_mut(key).ok_or_else(|| crate::NOT_CONNECTED.to_string())?;
        if let Some(busy) = background::refused(key) {
            return Err(busy);
        }
        let table = run(&*db.conn()?, &sql, &values)?;
        Ok::<_, String>(table.map(|t| {
            db.results = ResultSet::new(t.columns, t.rows).typed(t.types);
            db.origins = t.origins;
            (db.results.rows.len(), db.results.columns.len())
        }))
    });
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
            host.report(&format!("[SQLite] Query error: {e}"));
            host.fire(name, "onerror", &[v_str(&e)]);
            false
        }
    };
    host.fire(name, "onquerydone", &[]);
    v_int(ok as i64)
}

/// `QueryScalar(sql [, values …])`: the first column of the first row the
/// query gives ("" if none). The component's rows stay as they were.
fn query_scalar(host: &dyn Host, name: &str, key: &str, args: &[Value]) -> Value {
    let (sql, values) = sql_and_params(key, args);
    let outcome = DBS.with(|d| {
        let dbs = d.borrow();
        let db = dbs.get(key).ok_or_else(|| crate::NOT_CONNECTED.to_string())?;
        if let Some(busy) = background::refused(key) {
            return Err(busy);
        }
        let found = run(&*db.conn()?, &sql, &values);
        found
    });
    match outcome {
        Ok(table) => v_str(table.and_then(|t| t.rows.into_iter().next()?.into_iter().next()).as_deref().unwrap_or("")),
        Err(e) => {
            host.report(&format!("[SQLite] Query error: {e}"));
            host.fire(name, "onerror", &[v_str(&e)]);
            v_str("")
        }
    }
}

/// Runs the statements of `sql` in order with `values` bound to their
/// placeholders: the last result rows, if a statement had result columns.
fn run(conn: &Connection, sql: &str, values: &[Value]) -> Result<Option<Table>, String> {
    let mut used = 0;
    // One statement, the usual: its placeholders and the values are
    // counted before it runs. (prepare refuses several; one SQLite
    // refuses, the batch below refuses with the same message.)
    if let Ok(mut stmt) = conn.prepare(sql) {
        // (none at all: only blanks and comments)
        if stmt.expanded_sql().is_none() {
            return Ok(None);
        }
        if stmt.parameter_count() != values.len() {
            return Err(params::wrong_count(stmt.parameter_count(), values.len()));
        }
        return step(&mut stmt, values, &mut used);
    }
    let mut last = None;
    let mut batch = Batch::new(conn, sql);
    while let Some(mut stmt) = batch.next().map_err(|e| e.to_string())? {
        if let Some(table) = step(&mut stmt, values, &mut used)? {
            last = Some(table);
        }
    }
    if used != values.len() {
        return Err(params::wrong_count(used, values.len()));
    }
    Ok(last)
}

/// One statement, with the next of `values` bound to its placeholders:
/// gathered if it has result columns, executed otherwise.
fn step(stmt: &mut Statement, values: &[Value], used: &mut usize) -> Result<Option<Table>, String> {
    let text = |e: rusqlite::Error| e.to_string();
    let wanted = stmt.parameter_count();
    let Some(mine) = values.get(*used..*used + wanted) else {
        return Err(params::wrong_count(*used + wanted, values.len()));
    };
    for (i, v) in mine.iter().enumerate() {
        stmt.raw_bind_parameter(i + 1, sql_value(v)).map_err(text)?;
    }
    *used += wanted;
    let n = stmt.column_count();
    if n == 0 {
        stmt.raw_execute().map_err(text)?;
        return Ok(None);
    }
    let columns = stmt.column_names().into_iter().map(String::from).collect();
    let types = decl_types(stmt);
    let origins = stmt
        .columns_with_metadata()
        .iter()
        .map(|c| Some((c.table_name()?.to_string(), c.origin_name()?.to_string())))
        .collect();
    let mut rows = Vec::new();
    let mut found = stmt.raw_query();
    while let Some(row) = found.next().map_err(text)? {
        rows.push((0..n).map(|i| row.get_ref(i).map(cell_text).unwrap_or_default()).collect());
    }
    Ok(Some(Table { columns, types, rows, origins }))
}

/// A statement's result columns' declared types ("" for an expression).
fn decl_types(stmt: &Statement) -> Vec<String> {
    stmt.columns().iter().map(|c| c.decl_type().unwrap_or("").to_string()).collect()
}

/// A BASIC value as SQLite's: an integer INTEGER, a number REAL, a string
/// TEXT, Null NULL (a comparison's true is -1, as RapidQ has it).
fn sql_value(v: &Value) -> SqlValue {
    match v {
        Value::Integer(n) => SqlValue::Integer(*n),
        Value::Double(x) => SqlValue::Real(*x),
        Value::Boolean(_) => SqlValue::Integer(v.to_i64()),
        Value::Null => SqlValue::Null,
        other => SqlValue::Text(other.to_string_val()),
    }
}

/// A value as Row(i) gives it.
fn cell_text(v: ValueRef) -> String {
    match v {
        ValueRef::Null => String::new(),
        ValueRef::Integer(n) => n.to_string(),
        ValueRef::Real(x) => x.to_string(),
        ValueRef::Text(t) | ValueRef::Blob(t) => String::from_utf8_lossy(t).into_owned(),
    }
}

// ---------------------------------------------------------------------------
// Background queries
// ---------------------------------------------------------------------------

/// A background query: the SQL, its values, the connection, what it
/// shares with the program's thread.
pub struct Job {
    sql: String,
    values: Vec<SqlValue>,
    conn: Arc<Mutex<Connection>>,
    shared: Arc<Shared>,
}

impl Job {
    /// Runs it to its end, `pause` awaited after each chunk of rows (the
    /// web: the page's turn; the desktop's worker: nothing).
    pub async fn run<P, F>(self, pause: P)
    where
        P: FnMut() -> F,
        F: Future<Output = ()>,
    {
        let start = background::now();
        let end = match self.conn.try_lock() {
            Ok(conn) => stream(&conn, &self.sql, &self.values, &self.shared, pause).await,
            // (never: the component refuses a second query while one runs)
            Err(_) => End::Failed(background::BUSY.to_string()),
        };
        // (the connection given back before the program hears it's done)
        self.shared.end(end, background::ms_since(start));
    }
}

/// The statements of `sql` in order, each one with result columns
/// streamed into the shared rows in chunks (a later one starts them
/// again), the values bound to their placeholders. How it ended.
async fn stream<P, F>(conn: &Connection, sql: &str, values: &[SqlValue], shared: &Shared, mut pause: P) -> End
where
    P: FnMut() -> F,
    F: Future<Output = ()>,
{
    let failed = |e: String| if shared.cancelled() { End::Cancelled } else { End::Failed(e) };
    // (one statement: its placeholders counted before it runs, as Query)
    if let Ok(stmt) = conn.prepare(sql) {
        if stmt.parameter_count() != values.len() && stmt.expanded_sql().is_some() {
            return End::Failed(params::wrong_count(stmt.parameter_count(), values.len()));
        }
    }
    let mut used = 0;
    let mut batch = Batch::new(conn, sql);
    let mut text = String::new();
    loop {
        if shared.cancelled() {
            return End::Cancelled;
        }
        let mut stmt = match batch.next() {
            Ok(Some(stmt)) => stmt,
            Ok(None) => break,
            Err(e) => return failed(e.to_string()),
        };
        let wanted = stmt.parameter_count();
        let Some(mine) = values.get(used..used + wanted) else {
            return End::Failed(params::wrong_count(used + wanted, values.len()));
        };
        for (i, v) in mine.iter().enumerate() {
            if let Err(e) = stmt.raw_bind_parameter(i + 1, v) {
                return failed(e.to_string());
            }
        }
        used += wanted;
        let n = stmt.column_count();
        if n == 0 {
            if let Err(e) = stmt.raw_execute() {
                return failed(e.to_string());
            }
            continue;
        }
        let columns: Vec<String> = stmt.column_names().into_iter().map(String::from).collect();
        let types = decl_types(&stmt);
        let mut chunk = Chunker::new(shared, columns, types, background::slice());
        let mut rows = stmt.raw_query();
        loop {
            let row = match rows.next() {
                Ok(Some(row)) => row,
                Ok(None) => break,
                Err(e) => return failed(e.to_string()),
            };
            for i in 0..n {
                match row.get_ref(i) {
                    Ok(ValueRef::Null) | Err(_) => chunk.push(i, None),
                    Ok(ValueRef::Integer(v)) => {
                        text.clear();
                        let _ = write!(text, "{v}");
                        chunk.push(i, Some(&text));
                    }
                    Ok(ValueRef::Real(x)) => {
                        text.clear();
                        let _ = write!(text, "{x}");
                        chunk.push(i, Some(&text));
                    }
                    Ok(ValueRef::Text(t) | ValueRef::Blob(t)) => match std::str::from_utf8(t) {
                        Ok(s) => chunk.push(i, Some(s)),
                        Err(_) => chunk.push(i, Some(&String::from_utf8_lossy(t))),
                    },
                }
            }
            if chunk.end_row() {
                pause().await;
                if shared.cancelled() {
                    return End::Cancelled;
                }
            }
        }
        chunk.flush();
    }
    if used != values.len() {
        return End::Failed(params::wrong_count(used, values.len()));
    }
    End::Done
}

/// `QueryAsync(sql [, values …])`: 1 if the query started (its rows and
/// end come as events), 0 if it didn't — not connected, or one still
/// running (OnError says which).
fn query_async(host: &dyn Host, name: &str, key: &str, args: &[Value]) -> Value {
    let (sql, values) = sql_and_params(key, args);
    let refused = background::refused(key).or_else(|| (!DBS.with(|d| d.borrow().contains_key(key))).then(|| crate::NOT_CONNECTED.to_string()));
    if let Some(e) = refused {
        host.report(&format!("[SQLite] QueryAsync: {e}"));
        host.fire(name, "onerror", &[v_str(&e)]);
        return v_int(0);
    }
    let Some((conn, interrupt)) = DBS.with(|d| {
        let mut dbs = d.borrow_mut();
        let db = dbs.get_mut(key)?;
        db.results = ResultSet::framed(key);
        db.origins.clear();
        Some((db.conn.clone(), db.interrupt.clone()))
    }) else {
        return v_int(0);
    };
    let shared = Shared::new();
    let job = Job { sql, values: values.iter().map(sql_value).collect(), conn, shared: shared.clone() };
    background::begin(host, name, key, shared, Some(Box::new(move || interrupt.interrupt())));
    start(key, job);
    v_int(1)
}

/// The desktop: the job goes to the connection's worker thread (made the
/// first time).
#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
fn start(key: &str, job: Job) {
    let rejected = DBS.with(|d| {
        let mut dbs = d.borrow_mut();
        let Some(db) = dbs.get_mut(key) else { return Some(job) };
        let job = match &db.worker {
            Some(tx) => match tx.send(job) {
                Ok(()) => return None,
                // (its thread ended: a new one)
                Err(std::sync::mpsc::SendError(job)) => job,
            },
            None => job,
        };
        let (tx, rx) = std::sync::mpsc::channel::<Job>();
        let spawned = std::thread::Builder::new().name(format!("rapidr-sqlite-{key}")).spawn(move || {
            for job in rx {
                background::block_on(job.run(background::no_pause));
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
}

thread_local! {
    /// The web's background queries waiting to be run on the page's thread.
    static PENDING: RefCell<Vec<Job>> = const { RefCell::new(Vec::new()) };
}

/// The web: the job waits for the runtime ([`take_pending`]).
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
fn start(_key: &str, job: Job) {
    PENDING.with(|p| p.borrow_mut().push(job));
}

/// (the web's runtime, after each RSQLITE call) The background queries
/// just started: it runs each on the page's thread, `pause` giving the
/// page its turn between chunks (and polling, so their rows show).
pub fn take_pending() -> Vec<Job> {
    PENDING.with(|p| std::mem::take(&mut *p.borrow_mut()))
}

/// The web's bound widgets: each column of the row they show, and whether
/// there is one. None when the component isn't connected.
pub fn bound_fields(name: &str) -> Option<(Vec<(String, String)>, bool)> {
    DBS.with(|d| {
        let dbs = d.borrow();
        let r = &dbs.get(&name.to_lowercase())?.results;
        let row = r.row_cells(r.bound_row());
        let fields = r.column_names().into_iter().zip(row.clone().unwrap_or_default()).collect();
        Some((fields, row.is_some()))
    })
}

/// A bound widget edited: its field of the row the widgets show takes the
/// new value, and so does the database — in the table the column came
/// from, the row with the `id` the result has. Err with SQLite's message.
pub fn update_bound_field(name: &str, field: &str, value: &str) -> Result<(), String> {
    DBS.with(|d| {
        let mut dbs = d.borrow_mut();
        let Some(db) = dbs.get_mut(&name.to_lowercase()) else { return Ok(()) };
        let r = db.results.bound_row();
        let Some(c) = db.results.column_names().iter().position(|col| col.eq_ignore_ascii_case(field)) else { return Ok(()) };
        let Some(row) = db.results.row_cells(r) else { return Ok(()) };
        db.results.set_cell(r, c, value);
        let Some(Some((table, column))) = db.origins.get(c) else { return Ok(()) };
        let id = db.origins.iter().position(|o| matches!(o, Some((t, col)) if t == table && col.eq_ignore_ascii_case("id")));
        let (Some(i), Some(Some((_, id_column)))) = (id, id.and_then(|i| db.origins.get(i))) else { return Ok(()) };
        let quote = |s: &str| format!("\"{}\"", s.replace('"', "\"\""));
        let sql = format!("UPDATE {} SET {} = ?1 WHERE {} = ?2", quote(table), quote(column), quote(id_column));
        let done = db.conn()?.execute(&sql, [value, row[i].as_str()]).map(|_| ()).map_err(|e| e.to_string());
        done
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell as Cell;

    /// A host that records what the components did.
    #[derive(Default)]
    struct Log(Cell<Vec<String>>);
    impl Host for Log {
        fn set(&self, _: &str, prop: &str, value: Value) {
            self.0.borrow_mut().push(format!("{prop}={}", value.to_string_val()));
        }
        fn fire(&self, _: &str, event: &str, args: &[Value]) {
            let args: Vec<String> = args.iter().map(|a| a.to_string_val()).collect();
            self.0.borrow_mut().push(format!("{event}({})", args.join(",")));
        }
        fn report(&self, _: &str) {}
    }

    fn call(h: &Log, m: &str, args: &[Value]) -> String {
        method(h, "Db", m, args).to_string_val()
    }

    fn rows(h: &Log) -> Vec<String> {
        let mut out = Vec::new();
        while call(h, "fetchrow", &[]) == "1" {
            let n = DBS.with(|d| d.borrow()["db"].results.column_names().len());
            out.push((0..n).map(|i| call(h, "row", &[v_int(i as i64)])).collect::<Vec<_>>().join("|"));
        }
        out
    }

    #[test]
    fn queries_decide_by_the_statement() {
        let h = Log::default();
        assert_eq!(call(&h, "connect", &[v_str(":memory:")]), "0");
        assert_eq!(call(&h, "query", &[v_str("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT, v REAL)")]), "1");
        assert_eq!(call(&h, "query", &[v_str("INSERT INTO t (name, v) VALUES ('a=b', 2.5), (NULL, 3.0), ('x', 1e20)")]), "1");
        call(&h, "query", &[v_str("WITH c AS (SELECT * FROM t) SELECT name, v FROM c ORDER BY id")]);
        assert_eq!(rows(&h), ["a=b|2.5", "|3", "x|100000000000000000000"]);
        call(&h, "query", &[v_str("INSERT INTO t (name) VALUES ('new') RETURNING id, name")]);
        assert_eq!(rows(&h), ["4|new"]);
        call(&h, "query", &[v_str("CREATE TABLE u (a); INSERT INTO u VALUES (1); SELECT a + 1 FROM u")]);
        assert_eq!(rows(&h), ["2"]);
        // (a statement without rows leaves the last ones)
        call(&h, "query", &[v_str("SELECT name FROM t WHERE id = 1")]);
        call(&h, "query", &[v_str("DELETE FROM u")]);
        assert_eq!(rows(&h), ["a=b"]);
    }

    #[test]
    fn values_are_bound_never_spliced() {
        let h = Log::default();
        call(&h, "connect", &[v_str("")]);
        call(&h, "query", &[v_str("CREATE TABLE t (name TEXT, n)")]);
        call(&h, "query", &[v_str("INSERT INTO t VALUES (?, ?)"), v_str("it's, \"quoted\" AND = "), v_int(7)]);
        call(&h, "addparam", &[v_str("x' OR '1'='1")]);
        call(&h, "query", &[v_str("SELECT * FROM t WHERE name = ?")]);
        assert_eq!(rows(&h), Vec::<String>::new());
        call(&h, "query", &[v_str("SELECT n, typeof(n) FROM t WHERE name = ?"), v_str("it's, \"quoted\" AND = ")]);
        assert_eq!(rows(&h), ["7|integer"]);
        assert_eq!(call(&h, "queryscalar", &[v_str("SELECT ? * 2"), Value::Double(1.25)]), "2.5");
        assert_eq!(call(&h, "queryscalar", &[v_str("SELECT ? IS NULL"), Value::Null]), "1");
    }

    #[test]
    fn wrong_counts_are_errors() {
        let h = Log::default();
        call(&h, "connect", &[v_str("")]);
        call(&h, "query", &[v_str("CREATE TABLE t (a)")]);
        h.0.borrow_mut().clear();
        assert_eq!(call(&h, "query", &[v_str("INSERT INTO t VALUES (?)")]), "0");
        assert_eq!(call(&h, "query", &[v_str("INSERT INTO t VALUES (?)"), v_int(1), v_int(2)]), "0");
        assert_eq!(call(&h, "queryscalar", &[v_str("SELECT COUNT(*) FROM t")]), "0");
        let log = h.0.borrow().join("\n");
        assert!(log.contains("onerror(wrong number of parameters: the SQL has 1 placeholder (?) and 0 values were given)"), "{log}");
        assert!(log.contains("onerror(wrong number of parameters: the SQL has 1 placeholder (?) and 2 values were given)"), "{log}");
    }

    /// Runs the background queries until they end (at most ~20 s): the
    /// events land in the log as the program's thread would hear them.
    fn settle(h: &Log) {
        let start = std::time::Instant::now();
        while background::poll(h) {
            assert!(start.elapsed().as_secs() < 20, "a background query never ended");
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }

    fn events(h: &Log, prefix: &str) -> Vec<String> {
        h.0.borrow().iter().filter(|l| l.starts_with(prefix)).cloned().collect()
    }

    fn with_numbers(h: &Log, n: i64) {
        call(h, "connect", &[v_str(":memory:")]);
        let sql = format!(
            "CREATE TABLE big (id INTEGER PRIMARY KEY, name TEXT, score REAL, note TEXT);
             WITH RECURSIVE c(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM c WHERE i < {n})
             INSERT INTO big SELECT i, 'p' || i, i % 97 + 0.5, CASE WHEN i % 10 = 0 THEN NULL ELSE 'n' END FROM c"
        );
        assert_eq!(call(h, "query", &[v_str(&sql)]), "1");
        h.0.borrow_mut().clear();
    }

    #[test]
    fn a_background_query_streams_its_rows() {
        let h = Log::default();
        with_numbers(&h, 20_000);
        assert_eq!(call(&h, "queryasync", &[v_str("SELECT * FROM big WHERE id > ?"), v_int(5)]), "1");
        settle(&h);
        let log = h.0.borrow().join("\n");
        assert!(log.contains("busy=1") && log.contains("busy=0"), "{log}");
        assert!(log.contains("colcount=4"), "{log}");
        let done = events(&h, "ondone(");
        assert_eq!(done.len(), 1, "{log}");
        assert!(done[0].starts_with("ondone(19995,"), "{log}");
        // (OnRows counts up to every row)
        let rows_told: Vec<i64> = events(&h, "onrows(").iter().map(|e| e["onrows(".len()..e.len() - 1].parse().unwrap()).collect();
        assert!(!rows_told.is_empty() && rows_told.windows(2).all(|w| w[0] < w[1]) && *rows_told.last().unwrap() == 19995, "{rows_told:?}");
        // the rows, where FetchRow walks them and a grid draws them
        assert_eq!(rapidr_value::datascience::frame::with_frame("db", |f| (f.height(), f.width(), f.columns.join(","))), (19995, 4, "id,name,score,note".to_string()));
        assert_eq!(call(&h, "fetchrow", &[]), "1");
        assert_eq!((call(&h, "row", &[v_int(0)]), call(&h, "row", &[v_int(1)]), call(&h, "row", &[v_int(2)])), ("6".into(), "p6".into(), "6.5".into()));
        call(&h, "rowseek", &[v_int(4)]);
        call(&h, "fetchrow", &[]);
        // (a NULL is "", as Row(i) always gave it)
        assert_eq!((call(&h, "row", &[v_int(0)]), call(&h, "row", &[v_int(3)])), ("10".into(), "".into()));
        // a query afterwards works again
        assert_eq!(call(&h, "queryscalar", &[v_str("SELECT COUNT(*) FROM big")]), "20000");
    }

    /// The same code the web runs: the rows come in chunks — the first
    /// ones small, so they show at once — with a pause after each.
    #[test]
    fn rows_come_in_growing_chunks_with_pauses() {
        let h = Log::default();
        with_numbers(&h, 10_000);
        let conn = DBS.with(|d| d.borrow()["db"].conn.clone());
        let shared = Shared::new();
        let job = Job { sql: "SELECT id FROM big".into(), values: Vec::new(), conn, shared: shared.clone() };
        background::begin(&h, "Db", "db", shared.clone(), None);
        let mut sizes = Vec::new();
        background::block_on(job.run(|| {
            background::poll(&h);
            sizes.push(rapidr_value::datascience::frame::with_frame("db", |f| f.height()));
            background::no_pause()
        }));
        background::poll(&h);
        assert_eq!(sizes.first(), Some(&64), "{sizes:?}");
        assert!(sizes.len() >= 5 && sizes.windows(2).all(|w| w[0] < w[1]), "{sizes:?}");
        assert!(events(&h, "ondone(10000,").len() == 1, "{:?}", h.0.borrow());
    }

    #[test]
    fn cancel_stops_a_query_with_cancelled() {
        let h = Log::default();
        call(&h, "connect", &[v_str(":memory:")]);
        // rows without end
        call(&h, "queryasync", &[v_str("WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM c) SELECT x FROM c")]);
        std::thread::sleep(std::time::Duration::from_millis(30));
        background::poll(&h);
        assert_eq!(call(&h, "cancel", &[]), "1");
        settle(&h);
        assert_eq!(events(&h, "onerror("), ["onerror(cancelled)"]);
        assert!(events(&h, "ondone(").is_empty());
        // one long step (no row before the end): SQLite's interrupt stops it
        h.0.borrow_mut().clear();
        call(&h, "queryasync", &[v_str("WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM c WHERE x < 1000000000) SELECT COUNT(*) FROM c")]);
        std::thread::sleep(std::time::Duration::from_millis(50));
        let start = std::time::Instant::now();
        call(&h, "cancel", &[]);
        settle(&h);
        assert!(start.elapsed().as_millis() < 2000);
        assert_eq!(events(&h, "onerror("), ["onerror(cancelled)"]);
        // nothing running: Cancel says so
        assert_eq!(call(&h, "cancel", &[]), "0");
        assert_eq!(call(&h, "queryscalar", &[v_str("SELECT 41 + 1")]), "42");
    }

    #[test]
    fn background_errors_and_refusals() {
        let h = Log::default();
        // not connected
        assert_eq!(call(&h, "queryasync", &[v_str("SELECT 1")]), "0");
        assert_eq!(events(&h, "onerror("), ["onerror(not connected: Connect first)"]);
        with_numbers(&h, 300_000);
        // SQLite's own message
        call(&h, "queryasync", &[v_str("SELECT * FROM nothing_here")]);
        settle(&h);
        assert_eq!(events(&h, "onerror("), ["onerror(no such table: nothing_here)"]);
        h.0.borrow_mut().clear();
        // wrong counts, before anything runs
        call(&h, "queryasync", &[v_str("SELECT * FROM big WHERE id = ?")]);
        settle(&h);
        assert_eq!(events(&h, "onerror("), ["onerror(wrong number of parameters: the SQL has 1 placeholder (?) and 0 values were given)"]);
        h.0.borrow_mut().clear();
        // one at a time: the others are refused, never waited for
        assert_eq!(call(&h, "queryasync", &[v_str("SELECT a.id FROM big a, big b")]), "1");
        assert_eq!(call(&h, "queryasync", &[v_str("SELECT 1")]), "0");
        assert_eq!(call(&h, "query", &[v_str("SELECT 1")]), "0");
        assert_eq!(call(&h, "queryscalar", &[v_str("SELECT 1")]), "");
        let busy = format!("onerror({})", background::BUSY);
        assert_eq!(events(&h, "onerror("), [busy.clone(), busy.clone(), busy]);
        // Close stops it: no event after
        h.0.borrow_mut().clear();
        call(&h, "close", &[]);
        settle(&h);
        assert!(events(&h, "ondone(").is_empty() && events(&h, "onrows(").is_empty(), "{:?}", h.0.borrow());
    }

    #[test]
    fn several_statements_stream_the_last_rows() {
        let h = Log::default();
        call(&h, "connect", &[v_str(":memory:")]);
        call(&h, "queryasync", &[v_str("CREATE TABLE t (a, b); INSERT INTO t VALUES (?, ?), (3, 'x'); SELECT b, a FROM t ORDER BY a"), v_int(1), v_str("y")]);
        settle(&h);
        assert_eq!(events(&h, "ondone(").len(), 1, "{:?}", h.0.borrow());
        assert_eq!(rows(&h), ["y|1", "x|3"]);
        // a Query afterwards: its rows, and the frame a grid shows follows
        call(&h, "query", &[v_str("SELECT a FROM t WHERE a > 2")]);
        assert_eq!(rows(&h), ["3"]);
        assert_eq!(rapidr_value::datascience::frame::with_frame("db", |f| (f.height(), f.columns.join(","))), (1, "a".to_string()));
    }
}

#[cfg(test)]
mod column_tests {
    use super::*;

    struct Quiet;
    impl Host for Quiet {
        fn set(&self, _: &str, _: &str, _: Value) {}
        fn fire(&self, _: &str, _: &str, _: &[Value]) {}
        fn report(&self, _: &str) {}
    }

    /// ColName(i) and ColType(i): the result's own columns — names as the
    /// statement gives them (a repeated one too), the declared types (""
    /// for an expression) — for Query and for QueryAsync.
    #[test]
    fn column_names_and_types() {
        let h = Quiet;
        let call = |m: &str, args: &[Value]| method(&h, "Cols", m, args).to_string_val();
        call("connect", &[v_str(":memory:")]);
        call("query", &[v_str("CREATE TABLE people (id INTEGER PRIMARY KEY, name VARCHAR(40), born DATE, score REAL); INSERT INTO people (name) VALUES ('Ann')")]);
        let cols = |what: &str| (0..5).map(|i| call(what, &[v_int(i)])).collect::<Vec<_>>();
        assert_eq!(call("query", &[v_str("SELECT p.id, q.id, p.name AS who, p.born, p.score * 2 FROM people p, people q")]), "1");
        assert_eq!(cols("colname"), ["id", "id", "who", "born", "p.score * 2"]);
        assert_eq!(cols("coltype"), ["INTEGER", "INTEGER", "VARCHAR(40)", "DATE", ""]);
        assert_eq!((call("colname", &[v_int(5)]), call("coltype", &[v_int(-1)])), (String::new(), String::new()));
        call("queryasync", &[v_str("SELECT name, id AS n, id FROM people")]);
        let start = std::time::Instant::now();
        while background::poll(&h) {
            assert!(start.elapsed().as_secs() < 10);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert_eq!(cols("colname")[..3], ["name", "n", "id"]);
        assert_eq!(cols("coltype")[..3], ["VARCHAR(40)", "INTEGER", "INTEGER"]);
        // (not connected: nothing)
        assert_eq!(method(&h, "Other", "colname", &[v_int(0)]).to_string_val(), "");
    }
}
