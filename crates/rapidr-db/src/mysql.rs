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

use std::cell::RefCell;
use std::collections::HashMap;

use mysql::consts::ColumnType;
use mysql::prelude::{Protocol, Queryable};
use mysql::{Column, QueryResult, Value as MyValue};
use rapidr_value::{v_int, v_null, v_str, Value};

use crate::{cursor_method, param_method, params, publish, sql_and_params, Host, ResultSet};

struct Db {
    conn: mysql::PooledConn,
    results: ResultSet,
    databases: Vec<String>,
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
    match method {
        "connect" => return connect(host, name, &key, args),
        "close" | "disconnect" => return close(host, name, &key),
        "query" | "execute" => return query(host, name, &key, args),
        "escapestring" => {
            let s = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            return v_str(&s.replace('\\', "\\\\").replace('\'', "\\'").replace('"', "\\\""));
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

/// `Connect(host, user, password [, database])`, each missing one from
/// the component's property (Host, User, Password, DB; Port).
fn connect(host: &dyn Host, name: &str, key: &str, args: &[Value]) -> Value {
    let arg = |i: usize, prop: &str| args.get(i).map(|v| v.to_string_val()).unwrap_or_else(|| host.get(name, prop).to_string_val());
    let (server, user, password, db) = (arg(0, "host"), arg(1, "user"), arg(2, "password"), arg(3, "db"));
    let port = match host.get(name, "port").to_i64() {
        p @ 1..=65535 => p as u16,
        _ => 3306,
    };
    let mut opts = mysql::OptsBuilder::new().ip_or_hostname(Some(&server)).user(Some(&user)).pass(Some(&password)).tcp_port(port);
    if !db.is_empty() {
        opts = opts.db_name(Some(&db));
    }
    let opened = mysql::Pool::new(opts).and_then(|pool| Ok((pool.get_conn()?, pool)));
    match opened {
        Ok((mut conn, pool)) => {
            // (the databases the server has: DB(i), DBCount)
            let databases: Vec<String> = conn.query("SHOW DATABASES").unwrap_or_default();
            let count = databases.len() as i64;
            DBS.with(|d| d.borrow_mut().insert(key.to_string(), Db { conn, results: ResultSet::default(), databases }));
            POOLS.with(|p| p.borrow_mut().insert(key.to_string(), pool));
            host.set(name, "connected", v_int(1));
            host.set(name, "dbcount", v_int(count));
            host.row_changed(name);
            host.fire(name, "onconnect", &[]);
            v_int(1)
        }
        Err(e) => {
            host.report(&format!("[MySQL] Connection error: {e}"));
            host.set(name, "connected", v_int(0));
            host.fire(name, "onerror", &[v_str(&e.to_string())]);
            v_int(0)
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
        Ok::<_, String>(table.map(|(columns, rows)| {
            db.results = ResultSet::new(columns, rows);
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

type Rows = (Vec<String>, Vec<Vec<String>>);

/// Every result set of a query read to its end: the last that had columns.
fn collect<T: Protocol>(result: &mut QueryResult<'_, '_, '_, T>, cell: fn(&MyValue, &Column) -> String) -> Result<Option<Rows>, String> {
    let mut last = None;
    while let Some(set) = result.iter() {
        let columns: Vec<Column> = set.columns().as_ref().to_vec();
        let mut rows = Vec::new();
        for row in set {
            let row = row.map_err(|e| e.to_string())?;
            rows.push(columns.iter().enumerate().map(|(i, c)| row.as_ref(i).map(|v| cell(v, c)).unwrap_or_default()).collect());
        }
        if !columns.is_empty() {
            last = Some((columns.iter().map(|c| c.name_str().into_owned()).collect(), rows));
        }
    }
    Ok(last)
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
    use super::number_text;

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
