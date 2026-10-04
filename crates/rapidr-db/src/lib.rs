//! RSQLITE and RMYSQL — the database components — as every runtime has
//! them. Native builds and the interpreter (rapidr-runtime-core) and the
//! web (rapidr-runtime-web) call this same code, so a program's queries
//! give the same rows, the same text and the same messages everywhere.
//!
//! SQLite is the real one on every runtime: compiled from its C sources
//! into the desktop runtime and into the web's wasm ([`sqlite`]). MySQL
//! needs a TCP connection, which a browser can't open: native builds and
//! the interpreter only ([`mysql`], feature `mysql`).
//!
//! What only a runtime has, it gives the components through [`Host`]:
//! their properties (Connected, DB, RowCount, ColCount, FieldCount), their
//! events (OnConnect, OnQueryDone, OnDisconnect, OnError), where messages
//! go, and on the web the project's files.
//!
//! Parameter binding (RapidR's; SECURITY.md SEC-07): `DB.Query(sql, p1,
//! p2, …)` binds the values after the SQL to its `?` placeholders, and
//! `DB.AddParam v` queues one for the next query (`DB.ClearParams` drops
//! them). A bound value is never SQL text, so no value — `x' OR '1'='1` —
//! can change what the statement does (SQL injection).

use rapidr_value::{v_int, v_null, v_str, Value};

mod params;
mod results;

#[cfg(feature = "mysql")]
pub mod mysql;
#[cfg(feature = "sqlite")]
pub mod sqlite;

pub use params::{flatten, wrong_count};
pub use results::ResultSet;

/// A query on a component that isn't connected (OnError gets it).
pub const NOT_CONNECTED: &str = "not connected: Connect first";

/// What a runtime gives the database components.
pub trait Host {
    /// Sets one of the component's properties (`connected`, `db`,
    /// `rowcount`, `colcount`, `fieldcount`, `dbcount`).
    fn set(&self, name: &str, prop: &str, value: Value);

    /// Reads one (RMYSQL's Host, User, Password, DB and Port).
    fn get(&self, _name: &str, _prop: &str) -> Value {
        v_null()
    }

    /// Fires the component's event (`onconnect`, `onquerydone`,
    /// `ondisconnect`; `onerror` with the message).
    fn fire(&self, name: &str, event: &str, args: &[Value]);

    /// A message where the runtime shows them (stderr, the browser's
    /// console).
    fn report(&self, message: &str);

    /// A database file's bytes where the runtime has no file system (the
    /// web: the project's files, the ones the program saved).
    fn file_bytes(&self, _path: &str) -> Option<Vec<u8>> {
        None
    }

    /// The current row changed (a query's rows, FetchRow): the web's
    /// widgets bound to the component (DataSource / DataField) show it.
    fn row_changed(&self, _name: &str) {}
}

/// A component's new result set (`rows` × `columns`) after a query that
/// returned rows: its properties, and the bound widgets. (Called once the
/// component's state isn't borrowed: the host may run the program's code.)
fn publish(host: &dyn Host, name: &str, (rows, columns): (usize, usize)) {
    host.set(name, "rowcount", v_int(rows as i64));
    host.set(name, "colcount", v_int(columns as i64));
    host.set(name, "fieldcount", v_int(columns as i64));
    host.row_changed(name);
}

/// The methods that walk a result set, the same for every database:
/// FetchRow, FetchField, FieldSeek, RowSeek, Row(i). `None` for any other.
fn cursor_method(results: Option<&mut ResultSet>, method: &str, args: &[Value]) -> Option<Value> {
    let n = || args.first().map(|v| v.to_i64()).unwrap_or(0);
    Some(match (method, results) {
        ("fetchrow", Some(r)) => v_int(r.fetch_row() as i64),
        ("fetchfield", Some(r)) => v_int(r.fetch_field() as i64),
        ("fieldseek", Some(r)) => {
            r.field_seek(n());
            v_null()
        }
        ("rowseek", Some(r)) => {
            r.row_seek(n());
            v_null()
        }
        ("row", Some(r)) => v_str(&r.cell(n())),
        ("fetchrow" | "fetchfield", None) => v_int(0),
        ("fieldseek" | "rowseek", None) => v_null(),
        ("row", None) => v_str(""),
        _ => return None,
    })
}

/// A query's SQL and its parameters: the ones queued with AddParam first,
/// then the arguments after the SQL (arrays flattened). The queue is
/// emptied.
fn sql_and_params(key: &str, args: &[Value]) -> (String, Vec<Value>) {
    let sql = args.first().map(|v| v.to_string_val()).unwrap_or_default();
    (sql, params::take(key, args.get(1..).unwrap_or(&[])))
}

/// AddParam / ClearParams, the same for every database.
fn param_method(key: &str, method: &str, args: &[Value]) -> Option<Value> {
    match method {
        "addparam" => params::add(key, args),
        "clearparams" => params::clear(key),
        _ => return None,
    }
    Some(v_null())
}
