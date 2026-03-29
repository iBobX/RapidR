//! Database backends for RSQLite (and eventually RMySQL).

use std::cell::RefCell;
use std::collections::HashMap;

use crate::object::{rp_comp_get, rp_comp_set};
use crate::value::{v_int, v_null, v_str, Value};

// ---------------------------------------------------------------------------
// SQLite backend
// ---------------------------------------------------------------------------

struct SqliteState {
    conn: rusqlite::Connection,
    /// Current result rows after a SELECT query.
    rows: Vec<Vec<String>>,
    /// Column names from last query.
    columns: Vec<String>,
    /// Current row cursor (0-based, -1 means before first).
    row_cursor: i64,
    /// Current field cursor within the fetched row.
    field_cursor: i64,
}

thread_local! {
    static SQLITE_STATES: RefCell<HashMap<String, SqliteState>> = RefCell::new(HashMap::new());
}

pub fn sqlite_method(name: &str, method: &str, args: &[Value]) -> Value {
    let name_lower = name.to_lowercase();
    match method {
        "connect" => sqlite_connect(&name_lower, args),
        "close" => sqlite_close(&name_lower),
        "query" => sqlite_query(&name_lower, args),
        "fetchrow" => sqlite_fetchrow(&name_lower),
        "fetchfield" => sqlite_fetchfield(&name_lower),
        "fieldseek" => sqlite_fieldseek(&name_lower, args),
        "rowseek" => sqlite_rowseek(&name_lower, args),
        "row" => sqlite_row(&name_lower, args),
        "escapestring" => sqlite_escape(args),
        _ => {
            eprintln!("[WARN] RSQLite.{}() not implemented", method);
            v_null()
        }
    }
}

fn sqlite_connect(name: &str, args: &[Value]) -> Value {
    let db_path = args.first().map(|v| v.to_string_val()).unwrap_or_default();
    let exists = std::path::Path::new(&db_path).exists();

    match rusqlite::Connection::open(&db_path) {
        Ok(conn) => {
            let state = SqliteState {
                conn,
                rows: Vec::new(),
                columns: Vec::new(),
                row_cursor: -1,
                field_cursor: 0,
            };
            SQLITE_STATES.with(|s| {
                s.borrow_mut().insert(name.to_string(), state);
            });
            rp_comp_set(name, "connected", v_int(1));
            rp_comp_set(name, "db", v_str(&db_path));
            // Return 1 if database existed, 0 if newly created
            v_int(if exists { 1 } else { 0 })
        }
        Err(e) => {
            eprintln!("[SQLite] Connect error: {}", e);
            rp_comp_set(name, "connected", v_int(0));
            v_int(0)
        }
    }
}

fn sqlite_close(name: &str) -> Value {
    SQLITE_STATES.with(|s| {
        s.borrow_mut().remove(name);
    });
    rp_comp_set(name, "connected", v_int(0));
    v_null()
}

fn sqlite_query(name: &str, args: &[Value]) -> Value {
    let query_str = args.first().map(|v| v.to_string_val()).unwrap_or_default();
    let query_upper = query_str.trim_start().to_uppercase();
    let is_select = query_upper.starts_with("SELECT") || query_upper.starts_with("PRAGMA");

    SQLITE_STATES.with(|s| {
        let mut states = s.borrow_mut();
        if let Some(state) = states.get_mut(name) {
            if is_select {
                // Execute SELECT and gather results
                match state.conn.prepare(&query_str) {
                    Ok(mut stmt) => {
                        let col_count = stmt.column_count();
                        let columns: Vec<String> = (0..col_count)
                            .map(|i| stmt.column_name(i).unwrap_or("").to_string())
                            .collect();

                        let rows_result: Result<Vec<Vec<String>>, _> =
                            stmt.query_map([], |row| {
                                let mut vals = Vec::new();
                                for i in 0..col_count {
                                    let val: String = row
                                        .get::<_, rusqlite::types::Value>(i)
                                        .map(|v| match v {
                                            rusqlite::types::Value::Null => String::new(),
                                            rusqlite::types::Value::Integer(n) => n.to_string(),
                                            rusqlite::types::Value::Real(n) => n.to_string(),
                                            rusqlite::types::Value::Text(s) => s,
                                            rusqlite::types::Value::Blob(b) => {
                                                String::from_utf8_lossy(&b).into_owned()
                                            }
                                        })
                                        .unwrap_or_default();
                                    vals.push(val);
                                }
                                Ok(vals)
                            })
                            .and_then(|mapped| mapped.collect());

                        match rows_result {
                            Ok(rows) => {
                                let row_count = rows.len() as i64;
                                state.rows = rows;
                                state.columns = columns;
                                state.row_cursor = -1;
                                state.field_cursor = 0;
                                rp_comp_set(name, "rowcount", v_int(row_count));
                                rp_comp_set(
                                    name,
                                    "colcount",
                                    v_int(col_count as i64),
                                );
                                rp_comp_set(
                                    name,
                                    "fieldcount",
                                    v_int(col_count as i64),
                                );
                                v_int(1)
                            }
                            Err(e) => {
                                eprintln!("[SQLite] Query error: {}", e);
                                state.rows.clear();
                                state.columns.clear();
                                v_int(0)
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[SQLite] Prepare error: {}", e);
                        v_int(0)
                    }
                }
            } else {
                // Execute non-SELECT (INSERT, UPDATE, DELETE, CREATE, etc.)
                match state.conn.execute(&query_str, []) {
                    Ok(_) => v_int(1),
                    Err(e) => {
                        eprintln!("[SQLite] Execute error: {}", e);
                        v_int(0)
                    }
                }
            }
        } else {
            eprintln!("[SQLite] Not connected: {}", name);
            v_int(0)
        }
    })
}

fn sqlite_fetchrow(name: &str) -> Value {
    SQLITE_STATES.with(|s| {
        let mut states = s.borrow_mut();
        if let Some(state) = states.get_mut(name) {
            state.row_cursor += 1;
            state.field_cursor = 0;
            if (state.row_cursor as usize) < state.rows.len() {
                v_int(1)
            } else {
                v_int(0)
            }
        } else {
            v_int(0)
        }
    })
}

fn sqlite_fetchfield(name: &str) -> Value {
    SQLITE_STATES.with(|s| {
        let mut states = s.borrow_mut();
        if let Some(state) = states.get_mut(name) {
            if let Some(row) = state.rows.get(state.row_cursor as usize) {
                if (state.field_cursor as usize) < row.len() {
                    state.field_cursor += 1;
                    v_int(1)
                } else {
                    v_int(0)
                }
            } else {
                v_int(0)
            }
        } else {
            v_int(0)
        }
    })
}

fn sqlite_fieldseek(name: &str, args: &[Value]) -> Value {
    let pos = args.first().map(|v| v.to_i64()).unwrap_or(0);
    SQLITE_STATES.with(|s| {
        let mut states = s.borrow_mut();
        if let Some(state) = states.get_mut(name) {
            state.field_cursor = pos;
        }
    });
    v_null()
}

fn sqlite_rowseek(name: &str, args: &[Value]) -> Value {
    let row = args.first().map(|v| v.to_i64()).unwrap_or(0);
    SQLITE_STATES.with(|s| {
        let mut states = s.borrow_mut();
        if let Some(state) = states.get_mut(name) {
            state.row_cursor = row - 1; // Will be incremented by next FetchRow
        }
    });
    v_null()
}

fn sqlite_row(name: &str, args: &[Value]) -> Value {
    let col = args.first().map(|v| v.to_i64()).unwrap_or(0) as usize;
    SQLITE_STATES.with(|s| {
        let states = s.borrow();
        if let Some(state) = states.get(name) {
            if let Some(row) = state.rows.get(state.row_cursor as usize) {
                if col < row.len() {
                    return v_str(&row[col]);
                }
            }
        }
        v_str("")
    })
}

fn sqlite_escape(args: &[Value]) -> Value {
    let s = args.first().map(|v| v.to_string_val()).unwrap_or_default();
    v_str(&s.replace('\'', "''"))
}

// ---------------------------------------------------------------------------
// MySQL backend (via `mysql` crate)
// ---------------------------------------------------------------------------

struct MysqlState {
    conn: mysql::PooledConn,
    rows: Vec<Vec<String>>,
    columns: Vec<String>,
    row_cursor: i64,
    field_cursor: i64,
    databases: Vec<String>,
}

thread_local! {
    static MYSQL_STATES: RefCell<HashMap<String, MysqlState>> = RefCell::new(HashMap::new());
    static MYSQL_POOLS: RefCell<HashMap<String, mysql::Pool>> = RefCell::new(HashMap::new());
}

pub fn mysql_method(name: &str, method: &str, args: &[Value]) -> Value {
    let name_lower = name.to_lowercase();
    match method {
        "connect" => mysql_connect(&name_lower, name, args),
        "close" | "disconnect" => mysql_close(&name_lower, name),
        "query" => mysql_query(&name_lower, name, args),
        "execute" => mysql_query(&name_lower, name, args),
        "fetchrow" => mysql_fetchrow(&name_lower),
        "fetchfield" => mysql_fetchfield(&name_lower),
        "fieldseek" => mysql_fieldseek(&name_lower, args),
        "rowseek" => mysql_rowseek(&name_lower, args),
        "row" => mysql_row(&name_lower, args),
        "escapestring" => mysql_escape(args),
        "selectdb" => mysql_selectdb(&name_lower, args),
        "db" => mysql_db_at(&name_lower, args),
        _ => {
            eprintln!("[WARN] RMySQL.{}() not implemented", method);
            v_null()
        }
    }
}

fn mysql_connect(name_lower: &str, name: &str, args: &[Value]) -> Value {
    // Connect(host, user, password[, database]) — reads more from properties if needed
    let host = args.first().map(|v| v.to_string_val())
        .unwrap_or_else(|| rp_comp_get(name, "host").to_string_val());
    let user = args.get(1).map(|v| v.to_string_val())
        .unwrap_or_else(|| rp_comp_get(name, "user").to_string_val());
    let password = args.get(2).map(|v| v.to_string_val())
        .unwrap_or_else(|| rp_comp_get(name, "password").to_string_val());
    // 4th arg or property for database name
    let db = args.get(3).map(|v| v.to_string_val())
        .unwrap_or_else(|| rp_comp_get(name, "db").to_string_val());
    let port_val = rp_comp_get(name, "port").to_i64();
    let port = if port_val > 0 { port_val as u16 } else { 3306 };

    let mut opts_builder = mysql::OptsBuilder::new()
        .ip_or_hostname(Some(&host))
        .user(Some(&user))
        .pass(Some(&password))
        .tcp_port(port);

    if !db.is_empty() {
        opts_builder = opts_builder.db_name(Some(&db));
    }

    match mysql::Pool::new(opts_builder) {
        Ok(pool) => {
            match pool.get_conn() {
                Ok(mut conn) => {
                    // Query available databases
                    let mut databases = Vec::new();
                    {
                        use mysql::prelude::Queryable;
                        if let Ok(result) = conn.query_iter("SHOW DATABASES") {
                            for row_result in result {
                                if let Ok(row) = row_result {
                                    if let Some(Ok(db_name)) = row.get_opt::<String, usize>(0) {
                                        databases.push(db_name);
                                    }
                                }
                            }
                        }
                    }
                    let db_count = databases.len() as i64;
                    MYSQL_STATES.with(|s| {
                        s.borrow_mut().insert(name_lower.to_string(), MysqlState {
                            conn,
                            rows: Vec::new(),
                            columns: Vec::new(),
                            row_cursor: -1,
                            field_cursor: 0,
                            databases,
                        });
                    });
                    MYSQL_POOLS.with(|p| {
                        p.borrow_mut().insert(name_lower.to_string(), pool);
                    });
                    rp_comp_set(name, "connected", v_int(1));
                    rp_comp_set(name, "dbcount", v_int(db_count));
                    v_int(1)
                }
                Err(e) => {
                    eprintln!("[MySQL] Connection error: {}", e);
                    rp_comp_set(name, "connected", v_int(0));
                    v_int(0)
                }
            }
        }
        Err(e) => {
            eprintln!("[MySQL] Pool error: {}", e);
            rp_comp_set(name, "connected", v_int(0));
            v_int(0)
        }
    }
}

fn mysql_close(name_lower: &str, name: &str) -> Value {
    MYSQL_STATES.with(|s| { s.borrow_mut().remove(name_lower); });
    MYSQL_POOLS.with(|p| { p.borrow_mut().remove(name_lower); });
    rp_comp_set(name, "connected", v_int(0));
    v_null()
}

fn mysql_query(name_lower: &str, name: &str, args: &[Value]) -> Value {
    use mysql::prelude::Queryable;
    let query_str = args.first().map(|v| v.to_string_val()).unwrap_or_default();
    let query_upper = query_str.trim_start().to_uppercase();
    let is_select = query_upper.starts_with("SELECT") || query_upper.starts_with("SHOW")
        || query_upper.starts_with("DESCRIBE") || query_upper.starts_with("EXPLAIN");

    MYSQL_STATES.with(|s| {
        let mut states = s.borrow_mut();
        if let Some(state) = states.get_mut(name_lower) {
            if is_select {
                match state.conn.query_iter(&query_str) {
                    Ok(result) => {
                        // Get column names
                        let columns: Vec<String> = result.columns().as_ref()
                            .iter()
                            .map(|c| c.name_str().to_string())
                            .collect();
                        let col_count = columns.len();

                        let mut rows = Vec::new();
                        for row_result in result {
                            match row_result {
                                Ok(row) => {
                                    let mut vals = Vec::new();
                                    for i in 0..col_count {
                                        let val: String = row.get_opt::<String, usize>(i)
                                            .and_then(|r| r.ok())
                                            .unwrap_or_default();
                                        vals.push(val);
                                    }
                                    rows.push(vals);
                                }
                                Err(e) => {
                                    eprintln!("[MySQL] Row error: {}", e);
                                }
                            }
                        }

                        let row_count = rows.len() as i64;
                        state.rows = rows;
                        state.columns = columns;
                        state.row_cursor = -1;
                        state.field_cursor = 0;
                        rp_comp_set(name, "rowcount", v_int(row_count));
                        rp_comp_set(name, "colcount", v_int(col_count as i64));
                        rp_comp_set(name, "fieldcount", v_int(col_count as i64));
                        v_int(1)
                    }
                    Err(e) => {
                        eprintln!("[MySQL] Query error: {}", e);
                        state.rows.clear();
                        state.columns.clear();
                        v_int(0)
                    }
                }
            } else {
                match state.conn.query_drop(&query_str) {
                    Ok(()) => v_int(1),
                    Err(e) => {
                        eprintln!("[MySQL] Execute error: {}", e);
                        v_int(0)
                    }
                }
            }
        } else {
            eprintln!("[MySQL] Not connected: {}", name_lower);
            v_int(0)
        }
    })
}

fn mysql_fetchrow(name_lower: &str) -> Value {
    MYSQL_STATES.with(|s| {
        let mut states = s.borrow_mut();
        if let Some(state) = states.get_mut(name_lower) {
            state.row_cursor += 1;
            state.field_cursor = 0;
            if (state.row_cursor as usize) < state.rows.len() {
                v_int(1)
            } else {
                v_int(0)
            }
        } else {
            v_int(0)
        }
    })
}

fn mysql_fetchfield(name_lower: &str) -> Value {
    MYSQL_STATES.with(|s| {
        let mut states = s.borrow_mut();
        if let Some(state) = states.get_mut(name_lower) {
            if let Some(row) = state.rows.get(state.row_cursor as usize) {
                if (state.field_cursor as usize) < row.len() {
                    state.field_cursor += 1;
                    v_int(1)
                } else {
                    v_int(0)
                }
            } else {
                v_int(0)
            }
        } else {
            v_int(0)
        }
    })
}

fn mysql_fieldseek(name_lower: &str, args: &[Value]) -> Value {
    let pos = args.first().map(|v| v.to_i64()).unwrap_or(0);
    MYSQL_STATES.with(|s| {
        let mut states = s.borrow_mut();
        if let Some(state) = states.get_mut(name_lower) {
            state.field_cursor = pos;
        }
    });
    v_null()
}

fn mysql_rowseek(name_lower: &str, args: &[Value]) -> Value {
    let row = args.first().map(|v| v.to_i64()).unwrap_or(0);
    MYSQL_STATES.with(|s| {
        let mut states = s.borrow_mut();
        if let Some(state) = states.get_mut(name_lower) {
            state.row_cursor = row - 1;
        }
    });
    v_null()
}

fn mysql_row(name_lower: &str, args: &[Value]) -> Value {
    let col = args.first().map(|v| v.to_i64()).unwrap_or(0) as usize;
    MYSQL_STATES.with(|s| {
        let states = s.borrow();
        if let Some(state) = states.get(name_lower) {
            if let Some(row) = state.rows.get(state.row_cursor as usize) {
                if col < row.len() {
                    return v_str(&row[col]);
                }
            }
        }
        v_str("")
    })
}

fn mysql_escape(args: &[Value]) -> Value {
    let s = args.first().map(|v| v.to_string_val()).unwrap_or_default();
    // Basic SQL escaping for MySQL
    v_str(&s.replace('\\', "\\\\").replace('\'', "\\'").replace('"', "\\\""))
}

fn mysql_db_at(name_lower: &str, args: &[Value]) -> Value {
    let idx = args.first().map(|v| v.to_i64()).unwrap_or(0) as usize;
    MYSQL_STATES.with(|s| {
        let states = s.borrow();
        if let Some(state) = states.get(name_lower) {
            if idx < state.databases.len() {
                return v_str(&state.databases[idx]);
            }
        }
        v_str("")
    })
}

fn mysql_selectdb(name_lower: &str, args: &[Value]) -> Value {
    use mysql::prelude::Queryable;
    let db = args.first().map(|v| v.to_string_val()).unwrap_or_default();
    MYSQL_STATES.with(|s| {
        let mut states = s.borrow_mut();
        if let Some(state) = states.get_mut(name_lower) {
            match state.conn.query_drop(format!("USE `{}`", db.replace('`', "``"))) {
                Ok(()) => v_int(1),
                Err(e) => {
                    eprintln!("[MySQL] SelectDB error: {}", e);
                    v_int(0)
                }
            }
        } else {
            v_int(0)
        }
    })
}
