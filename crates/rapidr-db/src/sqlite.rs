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
//! On the web the page has no file system: `Connect(name)` opens a
//! database kept in memory for the page's session (SQLite's memory VFS),
//! the project's file of that name (a `.db` asset) copied in first. Nothing
//! is saved to the browser's storage yet.

use std::cell::RefCell;
use std::collections::HashMap;

use rapidr_value::{v_int, v_null, v_str, Value};
use rusqlite::fallible_iterator::FallibleIterator;
use rusqlite::types::{Value as SqlValue, ValueRef};
use rusqlite::{Batch, Connection, Statement};

use crate::{cursor_method, param_method, params, publish, sql_and_params, Host, ResultSet};

struct Db {
    conn: Connection,
    results: ResultSet,
    /// Where each result column came from (table, column): None for an
    /// expression. The web's bound widgets write their edits back there.
    origins: Vec<Option<(String, String)>>,
}

thread_local! {
    /// The open databases, by component (lowercase name).
    static DBS: RefCell<HashMap<String, Db>> = RefCell::new(HashMap::new());
}

/// A statement's rows.
struct Table {
    columns: Vec<String>,
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
    match open(host, &path) {
        Ok((conn, existed)) => {
            let db = Db { conn, results: ResultSet::default(), origins: Vec::new() };
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
    let closed = DBS.with(|d| d.borrow_mut().remove(key));
    host.set(name, "connected", v_int(0));
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
        let table = run(&db.conn, &sql, &values)?;
        Ok::<_, String>(table.map(|t| {
            db.results = ResultSet::new(t.columns, t.rows);
            db.origins = t.origins;
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
        run(&db.conn, &sql, &values)
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
    Ok(Some(Table { columns, rows, origins }))
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

/// The web's bound widgets: each column of the row they show, and whether
/// there is one. None when the component isn't connected.
pub fn bound_fields(name: &str) -> Option<(Vec<(String, String)>, bool)> {
    DBS.with(|d| {
        let dbs = d.borrow();
        let r = &dbs.get(&name.to_lowercase())?.results;
        let row = r.rows.get(r.bound_row());
        let fields = r.columns.iter().cloned().zip(row.cloned().unwrap_or_default()).collect();
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
        let Some(c) = db.results.columns.iter().position(|col| col.eq_ignore_ascii_case(field)) else { return Ok(()) };
        let Some(row) = db.results.rows.get_mut(r) else { return Ok(()) };
        row[c] = value.to_string();
        let Some(Some((table, column))) = db.origins.get(c) else { return Ok(()) };
        let id = db.origins.iter().position(|o| matches!(o, Some((t, col)) if t == table && col.eq_ignore_ascii_case("id")));
        let (Some(i), Some(Some((_, id_column)))) = (id, id.and_then(|i| db.origins.get(i))) else { return Ok(()) };
        let quote = |s: &str| format!("\"{}\"", s.replace('"', "\"\""));
        let sql = format!("UPDATE {} SET {} = ?1 WHERE {} = ?2", quote(table), quote(column), quote(id_column));
        db.conn.execute(&sql, [value, row[i].as_str()]).map(|_| ()).map_err(|e| e.to_string())
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
            let n = DBS.with(|d| d.borrow()["db"].results.columns.len());
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
}
