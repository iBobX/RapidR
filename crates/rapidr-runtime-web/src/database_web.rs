//! RSQLite web runtime — in-memory table store with basic SQL parsing.
//!
//! No actual SQLite or rusqlite — we store tables as `Vec<Vec<String>>`
//! and parse simple SQL statements: CREATE TABLE, INSERT, SELECT, UPDATE, DELETE.
//!
//! RMySQL is **not implementable in the browser** because the MySQL wire protocol
//! requires raw TCP, which browsers cannot open. `mysql_method` below emits a
//! single clear error and returns a sentinel value so user programs fail loudly
//! instead of silently warning on every call.

use crate::object_web;
use crate::value::{v_int, v_null, v_str, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use wasm_bindgen::JsValue;

// ======================================================================
// RMySQL — native-only stub for the web runtime
// ======================================================================

thread_local! {
    static MYSQL_WARNED: RefCell<std::collections::HashSet<String>> =
        RefCell::new(std::collections::HashSet::new());
}

pub fn mysql_method(name: &str, _method: &str, _args: &[Value]) -> Value {
    // Emit the error once per RMySQL instance to avoid log spam.
    let key = name.to_uppercase();
    let already = MYSQL_WARNED.with(|s| {
        let mut set = s.borrow_mut();
        if set.contains(&key) {
            true
        } else {
            set.insert(key.clone());
            false
        }
    });
    if !already {
        web_sys::console::error_1(&JsValue::from_str(&format!(
            "[RapidR] {} (RMySQL) is unavailable on the web runtime: browsers cannot open raw TCP connections to MySQL. Use the native target, or call your backend via RHTTP.",
            name
        )));
    }
    v_int(0)
}

pub fn mysql_get_prop(_name: &str, _prop: &str) -> Value {
    v_str("")
}

pub fn mysql_set_prop(_name: &str, _prop: &str, _val: &Value) {
    // silently accept property assignments (Host=, User=, Password=, …)
    // — they're harmless config and shouldn't error every time.
}

// ======================================================================
// Storage
// ======================================================================

struct InMemoryDb {
    tables: HashMap<String, InMemoryTable>, // table_name -> table
    // Query results
    result_rows: Vec<Vec<String>>,
    result_cols: Vec<String>,
    row_cursor: usize,
    field_cursor: usize,
}

struct InMemoryTable {
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl InMemoryDb {
    fn new() -> Self {
        InMemoryDb {
            tables: HashMap::new(),
            result_rows: Vec::new(),
            result_cols: Vec::new(),
            row_cursor: 0,
            field_cursor: 0,
        }
    }
}

thread_local! {
    static DB_STORE: RefCell<HashMap<String, InMemoryDb>> = RefCell::new(HashMap::new());
}

// ======================================================================
// Public API
// ======================================================================

pub fn sqlite_method(name: &str, method: &str, args: &[Value]) -> Value {
    let uname = name.to_uppercase();
    match method {
        "queryscalar" | "query_scalar" | "scalar" => {
            // Execute a SELECT and return the first cell of the first row as a string.
            let sql = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            execute_sql(&uname, &sql);
            DB_STORE.with(|s| {
                let store = s.borrow();
                if let Some(db) = store.get(&uname) {
                    if let Some(row) = db.result_rows.first() {
                        if let Some(cell) = row.first() {
                            return v_str(cell);
                        }
                    }
                }
                v_str("")
            })
        }
        "open" | "connect" => {
            let db_name = args.first().map(|v| v.to_string_val()).unwrap_or_else(|| ":memory:".to_string());
            let existed = DB_STORE.with(|s| s.borrow().contains_key(&uname));
            if !existed {
                let mut db = InMemoryDb::new();
                if let Some(base64_data) = get_rapidr_asset(&db_name) {
                    if let Some(bytes) = decode_base64(&base64_data) {
                        load_sqlite_binary(&mut db, &bytes);
                    }
                }
                DB_STORE.with(|s| s.borrow_mut().insert(uname.clone(), db));
            }
            object_web::rp_comp_set(&uname, "connected", v_int(1));
            object_web::rp_comp_set(&uname, "db", v_str(&db_name));
            object_web::rp_fire_event(&uname, "onconnect");
            
            sync_bound_widgets(&uname);
            
            v_int(if existed { 1 } else { 0 })
        }
        "close" => {
            DB_STORE.with(|s| s.borrow_mut().remove(&uname));
            object_web::rp_comp_set(&uname, "connected", v_int(0));
            object_web::rp_fire_event(&uname, "ondisconnect");
            v_null()
        }
        "query" | "exec" => {
            let sql = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let result = execute_sql(&uname, &sql);
            object_web::rp_fire_event(&uname, "onquerydone");
            if sql.trim().to_uppercase().starts_with("SELECT") {
                if result.to_i64() == 1 {
                    sync_bound_widgets(&uname);
                }
            }
            result
        }
        "fetchrow" => {
            let res = DB_STORE.with(|s| {
                let mut store = s.borrow_mut();
                if let Some(db) = store.get_mut(&uname) {
                    if db.row_cursor < db.result_rows.len() {
                        db.field_cursor = 0;
                        db.row_cursor += 1;
                        1
                    } else {
                        0
                    }
                } else {
                    0
                }
            });
            if res == 1 {
                sync_bound_widgets(&uname);
            }
            v_int(res)
        }
        "fetchfield" => {
            DB_STORE.with(|s| {
                let mut store = s.borrow_mut();
                if let Some(db) = store.get_mut(&uname) {
                    let ri = if db.row_cursor > 0 { db.row_cursor - 1 } else { 0 };
                    if let Some(row) = db.result_rows.get(ri) {
                        if db.field_cursor < row.len() {
                            db.field_cursor += 1;
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
        "row" => {
            let col_idx = args.first().map(|v| v.to_i64()).unwrap_or(0) as usize;
            DB_STORE.with(|s| {
                let store = s.borrow();
                if let Some(db) = store.get(&uname) {
                    let ri = if db.row_cursor > 0 { db.row_cursor - 1 } else { 0 };
                    if let Some(row) = db.result_rows.get(ri) {
                        if let Some(val) = row.get(col_idx) {
                            return v_str(val);
                        }
                    }
                }
                v_str("")
            })
        }
        "fieldseek" => {
            let pos = args.first().map(|v| v.to_i64()).unwrap_or(0) as usize;
            DB_STORE.with(|s| {
                if let Some(db) = s.borrow_mut().get_mut(&uname) {
                    db.field_cursor = pos;
                }
            });
            v_null()
        }
        "rowseek" => {
            let row = args.first().map(|v| v.to_i64()).unwrap_or(0) as usize;
            DB_STORE.with(|s| {
                if let Some(db) = s.borrow_mut().get_mut(&uname) {
                    db.row_cursor = if row > 0 { row - 1 } else { 0 };
                }
            });
            sync_bound_widgets(&uname);
            v_null()
        }
        "escapestring" => {
            let s = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            v_str(&s.replace('\'', "''"))
        }
        _ => {
            web_sys::console::warn_1(&wasm_bindgen::JsValue::from_str(
                &format!("[WARN] RSQLite.{} not implemented on web", method)
            ));
            v_null()
        }
    }
}

// ======================================================================
// Simple SQL parser/executor
// ======================================================================

fn execute_sql(name: &str, sql: &str) -> Value {
    let trimmed = sql.trim();
    let upper = trimmed.to_uppercase();

    if upper.starts_with("CREATE TABLE") {
        exec_create_table(name, trimmed)
    } else if upper.starts_with("INSERT") {
        exec_insert(name, trimmed)
    } else if upper.starts_with("SELECT") {
        exec_select(name, trimmed)
    } else if upper.starts_with("UPDATE") {
        exec_update(name, trimmed)
    } else if upper.starts_with("DELETE") {
        exec_delete(name, trimmed)
    } else if upper.starts_with("DROP TABLE") {
        exec_drop_table(name, trimmed)
    } else {
        web_sys::console::warn_1(&wasm_bindgen::JsValue::from_str(
            &format!("[WARN] Unsupported SQL: {}", trimmed)
        ));
        v_int(0)
    }
}

// CREATE TABLE name (col1 TYPE, col2 TYPE, ...)
fn exec_create_table(name: &str, sql: &str) -> Value {
    // Extract table name and columns
    let upper = sql.to_uppercase();
    let rest = &sql[upper.find("CREATE TABLE").unwrap() + 12..].trim_start();

    // Handle IF NOT EXISTS
    let rest = if rest.to_uppercase().starts_with("IF NOT EXISTS") {
        rest[13..].trim_start()
    } else {
        rest
    };

    let paren = match rest.find('(') {
        Some(i) => i,
        None => return v_int(0),
    };
    let table_name = rest[..paren].trim().to_uppercase();
    let cols_str = &rest[paren + 1..];
    let cols_str = match cols_str.rfind(')') {
        Some(i) => &cols_str[..i],
        None => cols_str,
    };

    let columns: Vec<String> = cols_str.split(',')
        .map(|c| {
            let c = c.trim();
            // Take just the column name (first word)
            c.split_whitespace().next().unwrap_or("").to_string()
        })
        .filter(|c| !c.is_empty())
        .collect();

    DB_STORE.with(|s| {
        if let Some(db) = s.borrow_mut().get_mut(name) {
            db.tables.insert(table_name, InMemoryTable { columns, rows: Vec::new() });
        }
    });
    v_int(1)
}

// INSERT INTO table (col1, col2) VALUES (val1, val2)
fn exec_insert(name: &str, sql: &str) -> Value {
    let upper = sql.to_uppercase();

    // Find table name
    let into_pos = match upper.find("INTO") {
        Some(i) => i + 4,
        None => return v_int(0),
    };
    let rest = sql[into_pos..].trim_start();

    // Table name is the first word after INTO
    let table_name = rest.split_whitespace()
        .next()
        .unwrap_or("")
        .trim_end_matches('(')
        .to_uppercase();

    // Find VALUES clause
    let values_pos = match upper.find("VALUES") {
        Some(i) => i + 6,
        None => return v_int(0),
    };
    let vals_str = sql[values_pos..].trim_start();
    let vals_str = vals_str.trim_start_matches('(');
    let vals_str = match vals_str.rfind(')') {
        Some(i) => &vals_str[..i],
        None => vals_str,
    };

    let values: Vec<String> = parse_csv_values(vals_str);

    DB_STORE.with(|s| {
        if let Some(db) = s.borrow_mut().get_mut(name) {
            if let Some(table) = db.tables.get_mut(&table_name) {
                table.rows.push(values);
                return v_int(1);
            }
        }
        v_int(0)
    })
}

// SELECT col1, col2 FROM table [WHERE col op val] [ORDER BY col]
fn exec_select(name: &str, sql: &str) -> Value {
    let upper = sql.to_uppercase();

    // Extract columns
    let select_end = match upper.find("FROM") {
        Some(i) => i,
        None => return v_int(0),
    };
    let cols_str = sql[6..select_end].trim();

    // Extract table name
    let from_rest = sql[select_end + 4..].trim_start();
    let table_end = from_rest.to_uppercase()
        .find("WHERE")
        .or_else(|| from_rest.to_uppercase().find("ORDER"))
        .or_else(|| from_rest.to_uppercase().find("LIMIT"))
        .unwrap_or(from_rest.len());
    let table_name = from_rest[..table_end].trim().to_uppercase();

    // WHERE clause
    let where_clause = if let Some(w) = upper.find("WHERE") {
        let rest = sql[w + 5..].trim_start();
        let end = rest.to_uppercase().find("ORDER")
            .or_else(|| rest.to_uppercase().find("LIMIT"))
            .unwrap_or(rest.len());
        Some(rest[..end].trim().to_string())
    } else {
        None
    };

    // ORDER BY
    let order_by = if let Some(o) = upper.find("ORDER BY") {
        let rest = sql[o + 8..].trim_start();
        let end = rest.to_uppercase().find("LIMIT").unwrap_or(rest.len());
        Some(rest[..end].trim().to_string())
    } else {
        None
    };

    DB_STORE.with(|s| {
        let mut store = s.borrow_mut();
        if let Some(db) = store.get_mut(name) {
            let table = match db.tables.get(&table_name) {
                Some(t) => t,
                None => return v_int(0),
            };

            // --- Aggregate functions: COUNT(*), COUNT(col), SUM(col), AVG(col), MIN(col), MAX(col) ---
            // Detect simple aggregate-only SELECT (single expression).
            let cs = cols_str.trim();
            let cs_up = cs.to_uppercase();
            let is_agg = (cs_up.starts_with("COUNT(") || cs_up.starts_with("SUM(")
                || cs_up.starts_with("AVG(") || cs_up.starts_with("MIN(")
                || cs_up.starts_with("MAX(")) && cs_up.ends_with(')');
            if is_agg {
                // Filter rows first
                let filtered: Vec<&Vec<String>> = table.rows.iter()
                    .filter(|row| {
                        if let Some(ref wc) = where_clause {
                            eval_where(wc, row, &table.columns)
                        } else { true }
                    })
                    .collect();
                let open = cs.find('(').unwrap();
                let close = cs.rfind(')').unwrap();
                let inner = cs[open + 1..close].trim();
                let func = cs_up[..open].to_string();
                let val = if func == "COUNT" {
                    filtered.len() as f64
                } else if let Some(ci) = table.columns.iter().position(|c| c.eq_ignore_ascii_case(inner)) {
                    let nums: Vec<f64> = filtered.iter()
                        .filter_map(|r| r.get(ci).and_then(|v| v.parse::<f64>().ok()))
                        .collect();
                    match func.as_str() {
                        "SUM" => nums.iter().sum::<f64>(),
                        "AVG" => if nums.is_empty() { 0.0 } else { nums.iter().sum::<f64>() / nums.len() as f64 },
                        "MIN" => nums.iter().cloned().fold(f64::INFINITY, f64::min),
                        "MAX" => nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
                        _ => 0.0,
                    }
                } else { 0.0 };
                let cell = if val == val.floor() && val.abs() < 1e15 {
                    format!("{}", val as i64)
                } else { format!("{}", val) };
                db.result_cols = vec![cs.to_string()];
                db.result_rows = vec![vec![cell]];
                db.row_cursor = 0;
                db.field_cursor = 0;
                let uname = name.to_string();
                drop(store);
                object_web::rp_comp_set(&uname, "rowcount", v_int(1));
                object_web::rp_comp_set(&uname, "colcount", v_int(1));
                return v_int(1);
            }

            // Determine selected column indices
            let select_all = cols_str.trim() == "*";
            let col_indices: Vec<usize> = if select_all {
                (0..table.columns.len()).collect()
            } else {
                cols_str.split(',')
                    .filter_map(|c| {
                        let c = c.trim().to_uppercase();
                        table.columns.iter().position(|tc| tc.to_uppercase() == c)
                    })
                    .collect()
            };

            let result_cols: Vec<String> = col_indices.iter()
                .map(|&i| table.columns[i].clone())
                .collect();

            // Filter rows
            let mut rows: Vec<Vec<String>> = table.rows.iter()
                .filter(|row| {
                    if let Some(ref wc) = where_clause {
                        eval_where(wc, row, &table.columns)
                    } else {
                        true
                    }
                })
                .map(|row| {
                    col_indices.iter().map(|&i| row.get(i).cloned().unwrap_or_default()).collect()
                })
                .collect();

            // Sort
            if let Some(ref ob) = order_by {
                let parts: Vec<&str> = ob.split_whitespace().collect();
                let col_name = parts.first().unwrap_or(&"").to_uppercase();
                let desc = parts.get(1).map(|s| s.to_uppercase() == "DESC").unwrap_or(false);
                if let Some(ci) = result_cols.iter().position(|c| c.to_uppercase() == col_name) {
                    rows.sort_by(|a, b| {
                        let av = a.get(ci).cloned().unwrap_or_default();
                        let bv = b.get(ci).cloned().unwrap_or_default();
                        let cmp = if let (Ok(an), Ok(bn)) = (av.parse::<f64>(), bv.parse::<f64>()) {
                            an.partial_cmp(&bn).unwrap_or(std::cmp::Ordering::Equal)
                        } else {
                            av.cmp(&bv)
                        };
                        if desc { cmp.reverse() } else { cmp }
                    });
                }
            }

            let nrows = rows.len();
            let ncols = result_cols.len();
            db.result_rows = rows;
            db.result_cols = result_cols;
            db.row_cursor = 0;
            db.field_cursor = 0;

            // Update properties
            let uname = name.to_string();
            drop(store);
            object_web::rp_comp_set(&uname, "rowcount", v_int(nrows as i64));
            object_web::rp_comp_set(&uname, "colcount", v_int(ncols as i64));
            object_web::rp_comp_set(&uname, "fieldcount", v_int(ncols as i64));
            v_int(1)
        } else {
            v_int(0)
        }
    })
}

// UPDATE table SET col=val [WHERE ...]
fn exec_update(name: &str, sql: &str) -> Value {
    let upper = sql.to_uppercase();

    let table_start = match upper.find("UPDATE") {
        Some(i) => i + 6,
        None => return v_int(0),
    };
    let set_pos = match upper.find("SET") {
        Some(i) => i,
        None => return v_int(0),
    };
    let table_name = sql[table_start..set_pos].trim().to_uppercase();

    let where_pos = upper.find("WHERE");
    let set_clause = if let Some(w) = where_pos {
        sql[set_pos + 3..w].trim()
    } else {
        sql[set_pos + 3..].trim()
    };
    let where_clause = where_pos.map(|w| sql[w + 5..].trim().to_string());

    // Parse SET assignments
    let assignments: Vec<(String, String)> = set_clause.split(',')
        .filter_map(|a| {
            let mut parts = a.splitn(2, '=');
            let col = parts.next()?.trim().to_uppercase();
            let val = parts.next()?.trim().to_string();
            let val = strip_quotes(&val);
            Some((col, val))
        })
        .collect();

    DB_STORE.with(|s| {
        if let Some(db) = s.borrow_mut().get_mut(name) {
            if let Some(table) = db.tables.get_mut(&table_name) {
                let mut count = 0;
                for row in table.rows.iter_mut() {
                    let matches = if let Some(ref wc) = where_clause {
                        eval_where(wc, row, &table.columns)
                    } else {
                        true
                    };
                    if matches {
                        for (col, val) in &assignments {
                            if let Some(ci) = table.columns.iter().position(|c| c.to_uppercase() == *col) {
                                while row.len() <= ci { row.push(String::new()); }
                                row[ci] = val.clone();
                            }
                        }
                        count += 1;
                    }
                }
                return v_int(count);
            }
        }
        v_int(0)
    })
}

// DELETE FROM table [WHERE ...]
fn exec_delete(name: &str, sql: &str) -> Value {
    let upper = sql.to_uppercase();

    let from_pos = match upper.find("FROM") {
        Some(i) => i + 4,
        None => return v_int(0),
    };
    let where_pos = upper.find("WHERE");
    let table_end = where_pos.unwrap_or(sql.len());
    let table_name = sql[from_pos..table_end].trim().to_uppercase();

    let where_clause = where_pos.map(|w| sql[w + 5..].trim().to_string());

    DB_STORE.with(|s| {
        if let Some(db) = s.borrow_mut().get_mut(name) {
            if let Some(table) = db.tables.get_mut(&table_name) {
                let before = table.rows.len();
                if let Some(ref wc) = where_clause {
                    let cols = table.columns.clone();
                    table.rows.retain(|row| !eval_where(wc, row, &cols));
                } else {
                    table.rows.clear();
                }
                return v_int((before - table.rows.len()) as i64);
            }
        }
        v_int(0)
    })
}

// DROP TABLE name
fn exec_drop_table(name: &str, sql: &str) -> Value {
    let upper = sql.to_uppercase();
    let rest = &sql[upper.find("DROP TABLE").unwrap() + 10..].trim_start();
    let rest = if rest.to_uppercase().starts_with("IF EXISTS") {
        rest[9..].trim_start()
    } else {
        rest
    };
    let table_name = rest.trim().to_uppercase();

    DB_STORE.with(|s| {
        if let Some(db) = s.borrow_mut().get_mut(name) {
            db.tables.remove(&table_name);
        }
    });
    v_int(1)
}

// ======================================================================
// Helpers
// ======================================================================

fn eval_where(clause: &str, row: &[String], columns: &[String]) -> bool {
    // Simple: "col op value" with AND/OR
    // For now, handle single condition and AND chains
    let parts: Vec<&str> = clause.split(" AND ").collect();
    for part in parts {
        if !eval_single_condition(part.trim(), row, columns) {
            return false;
        }
    }
    true
}

fn eval_single_condition(cond: &str, row: &[String], columns: &[String]) -> bool {
    // Try operators: >=, <=, !=, <>, =, >, <, LIKE
    let ops = [">=", "<=", "!=", "<>", "=", ">", "<"];
    for op in &ops {
        if let Some(pos) = cond.find(op) {
            let col = cond[..pos].trim().to_uppercase();
            let val = strip_quotes(cond[pos + op.len()..].trim());
            if let Some(ci) = columns.iter().position(|c| c.to_uppercase() == col) {
                let cell = row.get(ci).cloned().unwrap_or_default();
                return match *op {
                    "=" => cell == val,
                    "!=" | "<>" => cell != val,
                    ">" => cmp_vals(&cell, &val) == std::cmp::Ordering::Greater,
                    ">=" => cmp_vals(&cell, &val) != std::cmp::Ordering::Less,
                    "<" => cmp_vals(&cell, &val) == std::cmp::Ordering::Less,
                    "<=" => cmp_vals(&cell, &val) != std::cmp::Ordering::Greater,
                    _ => false,
                };
            }
            return false;
        }
    }

    // LIKE
    let upper_cond = cond.to_uppercase();
    if let Some(pos) = upper_cond.find(" LIKE ") {
        let col = cond[..pos].trim().to_uppercase();
        let pattern = strip_quotes(cond[pos + 6..].trim());
        if let Some(ci) = columns.iter().position(|c| c.to_uppercase() == col) {
            let cell = row.get(ci).cloned().unwrap_or_default().to_uppercase();
            let pat = pattern.to_uppercase().replace('%', "");
            return cell.contains(&pat);
        }
    }

    true
}

fn cmp_vals(a: &str, b: &str) -> std::cmp::Ordering {
    if let (Ok(an), Ok(bn)) = (a.parse::<f64>(), b.parse::<f64>()) {
        an.partial_cmp(&bn).unwrap_or(std::cmp::Ordering::Equal)
    } else {
        a.cmp(b)
    }
}

fn strip_quotes(s: &str) -> String {
    let s = s.trim();
    if (s.starts_with('\'') && s.ends_with('\'')) || (s.starts_with('"') && s.ends_with('"')) {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

fn parse_csv_values(s: &str) -> Vec<String> {
    let mut vals = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut quote_char = ' ';

    for ch in s.chars() {
        if in_quotes {
            if ch == quote_char {
                in_quotes = false;
            } else {
                current.push(ch);
            }
        } else if ch == '\'' || ch == '"' {
            in_quotes = true;
            quote_char = ch;
        } else if ch == ',' {
            vals.push(current.trim().to_string());
            current = String::new();
        } else {
            current.push(ch);
        }
    }
    let last = current.trim().to_string();
    if !last.is_empty() { vals.push(last); }
    vals
}

// ======================================================================
// SQLite Binary B-tree Parser & Data Binding helpers
// ======================================================================

pub(crate) fn get_rapidr_asset(filename: &str) -> Option<String> {
    let window = web_sys::window()?;
    let assets_val = js_sys::Reflect::get(&window, &wasm_bindgen::JsValue::from_str("__rapidr_assets")).ok()?;
    if assets_val.is_undefined() || assets_val.is_null() {
        return None;
    }
    
    let val = js_sys::Reflect::get(&assets_val, &wasm_bindgen::JsValue::from_str(filename)).ok();
    if let Some(v) = val {
        if !v.is_undefined() && !v.is_null() {
            return v.as_string();
        }
    }
    
    let alternative = if filename.starts_with("assets/") {
        filename.strip_prefix("assets/").unwrap()
    } else {
        &format!("assets/{}", filename)
    };
    let val2 = js_sys::Reflect::get(&assets_val, &wasm_bindgen::JsValue::from_str(alternative)).ok();
    if let Some(v) = val2 {
        if !v.is_undefined() && !v.is_null() {
            return v.as_string();
        }
    }
    None
}

pub(crate) fn decode_base64(mut s: &str) -> Option<Vec<u8>> {
    if let Some(pos) = s.find("base64,") {
        s = &s[pos + 7..];
    }
    let s = s.trim();
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    let mut buffer = 0u32;
    let mut bits = 0;
    
    for &b in bytes {
        let val = match b {
            b'A'..=b'Z' => (b - b'A') as u32,
            b'a'..=b'z' => (b - b'a' + 26) as u32,
            b'0'..=b'9' => (b - b'0' + 52) as u32,
            b'+' => 62,
            b'/' => 63,
            b'=' => continue,
            _ if b.is_ascii_whitespace() => continue,
            _ => return None,
        };
        buffer = (buffer << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
        }
    }
    Some(out)
}

#[derive(Debug, Clone)]
enum SqliteValue {
    Null,
    Integer(i64),
    Float(f64),
    Text(String),
    Blob(Vec<u8>),
}

impl SqliteValue {
    fn to_string_val(&self) -> String {
        match self {
            SqliteValue::Null => String::new(),
            SqliteValue::Integer(x) => x.to_string(),
            SqliteValue::Float(f) => f.to_string(),
            SqliteValue::Text(s) => s.clone(),
            SqliteValue::Blob(b) => String::from_utf8_lossy(b).into_owned(),
        }
    }
}

fn read_varint(data: &[u8], offset: &mut usize) -> u64 {
    let mut val: u64 = 0;
    for i in 0..9 {
        if *offset >= data.len() {
            break;
        }
        let b = data[*offset];
        *offset += 1;
        if i == 8 {
            val = (val << 8) | (b as u64);
            break;
        } else {
            val = (val << 7) | ((b & 0x7F) as u64);
            if (b & 0x80) == 0 {
                break;
            }
        }
    }
    val
}

fn read_varint_signed(data: &[u8], offset: &mut usize) -> i64 {
    read_varint(data, offset) as i64
}

fn parse_record(payload: &[u8], _num_columns_hint: usize) -> Vec<SqliteValue> {
    let mut offset = 0;
    if payload.is_empty() {
        return Vec::new();
    }
    let header_size = read_varint(payload, &mut offset) as usize;
    let mut serial_types = Vec::new();
    while offset < header_size && offset < payload.len() {
        serial_types.push(read_varint(payload, &mut offset));
    }
    
    let mut values = Vec::new();
    for &st in &serial_types {
        let val = match st {
            0 => SqliteValue::Null,
            1 => {
                let v = if offset < payload.len() { payload[offset] as i8 as i64 } else { 0 };
                offset += 1;
                SqliteValue::Integer(v)
            }
            2 => {
                let mut v = 0i16;
                if offset + 2 <= payload.len() {
                    v = i16::from_be_bytes([payload[offset], payload[offset+1]]);
                }
                offset += 2;
                SqliteValue::Integer(v as i64)
            }
            3 => {
                let mut v = 0i32;
                if offset + 3 <= payload.len() {
                    let bytes = [payload[offset], payload[offset+1], payload[offset+2]];
                    v = ((bytes[0] as i32) << 16) | ((bytes[1] as i32) << 8) | (bytes[2] as i32);
                    if (v & 0x800000) != 0 {
                        v |= !0xFFFFFF;
                    }
                }
                offset += 3;
                SqliteValue::Integer(v as i64)
            }
            4 => {
                let mut v = 0i32;
                if offset + 4 <= payload.len() {
                    v = i32::from_be_bytes([payload[offset], payload[offset+1], payload[offset+2], payload[offset+3]]);
                }
                offset += 4;
                SqliteValue::Integer(v as i64)
            }
            5 => {
                let mut v = 0i64;
                if offset + 6 <= payload.len() {
                    let bytes = [payload[offset], payload[offset+1], payload[offset+2], payload[offset+3], payload[offset+4], payload[offset+5]];
                    v = ((bytes[0] as i64) << 40) | ((bytes[1] as i64) << 32) | ((bytes[2] as i64) << 24) |
                        ((bytes[3] as i64) << 16) | ((bytes[4] as i64) << 8) | (bytes[5] as i64);
                    if (v & 0x800000000000) != 0 {
                        v |= !0xFFFFFFFFFFFF;
                    }
                }
                offset += 6;
                SqliteValue::Integer(v)
            }
            6 => {
                let mut v = 0i64;
                if offset + 8 <= payload.len() {
                    v = i64::from_be_bytes([
                        payload[offset], payload[offset+1], payload[offset+2], payload[offset+3],
                        payload[offset+4], payload[offset+5], payload[offset+6], payload[offset+7]
                    ]);
                }
                offset += 8;
                SqliteValue::Integer(v)
            }
            7 => {
                let mut v = 0.0f64;
                if offset + 8 <= payload.len() {
                    v = f64::from_be_bytes([
                        payload[offset], payload[offset+1], payload[offset+2], payload[offset+3],
                        payload[offset+4], payload[offset+5], payload[offset+6], payload[offset+7]
                    ]);
                }
                offset += 8;
                SqliteValue::Float(v)
            }
            8 => SqliteValue::Integer(0),
            9 => SqliteValue::Integer(1),
            10 | 11 => SqliteValue::Null,
            st if st >= 12 && st % 2 == 0 => {
                let len = ((st - 12) / 2) as usize;
                let data = if offset + len <= payload.len() {
                    payload[offset..offset+len].to_vec()
                } else {
                    Vec::new()
                };
                offset += len;
                SqliteValue::Blob(data)
            }
            st if st >= 13 && st % 2 == 1 => {
                let len = ((st - 13) / 2) as usize;
                let text = if offset + len <= payload.len() {
                    String::from_utf8_lossy(&payload[offset..offset+len]).into_owned()
                } else {
                    String::new()
                };
                offset += len;
                SqliteValue::Text(text)
            }
            _ => SqliteValue::Null,
        };
        values.push(val);
    }
    values
}

fn collect_leaves(data: &[u8], page_num: usize, page_size: usize, leaves: &mut Vec<usize>) {
    if page_num == 0 || (page_num - 1) * page_size >= data.len() {
        return;
    }
    let page_offset = (page_num - 1) * page_size;
    let page_data = &data[page_offset..];
    
    let btree_offset = if page_num == 1 { 100 } else { 0 };
    if btree_offset + 8 > page_data.len() {
        return;
    }
    
    let page_type = page_data[btree_offset];
    if page_type == 0x0d {
        leaves.push(page_num);
    } else if page_type == 0x05 {
        if btree_offset + 12 > page_data.len() {
            return;
        }
        let num_cells = u16::from_be_bytes([page_data[btree_offset + 3], page_data[btree_offset + 4]]) as usize;
        let cell_ptr_start = btree_offset + 12;
        let right_most = u32::from_be_bytes([
            page_data[btree_offset + 8], page_data[btree_offset + 9],
            page_data[btree_offset + 10], page_data[btree_offset + 11]
        ]) as usize;
        
        for i in 0..num_cells {
            let ptr_offset = cell_ptr_start + i * 2;
            if ptr_offset + 2 > page_data.len() {
                break;
            }
            let cell_offset = u16::from_be_bytes([page_data[ptr_offset], page_data[ptr_offset + 1]]) as usize;
            if cell_offset + 4 > page_data.len() {
                continue;
            }
            let left_child = u32::from_be_bytes([
                page_data[cell_offset], page_data[cell_offset + 1],
                page_data[cell_offset + 2], page_data[cell_offset + 3]
            ]) as usize;
            collect_leaves(data, left_child, page_size, leaves);
        }
        collect_leaves(data, right_most, page_size, leaves);
    }
}

fn parse_leaf_page(data: &[u8], page_num: usize, page_size: usize) -> Vec<(i64, Vec<SqliteValue>)> {
    let mut records = Vec::new();
    if page_num == 0 || (page_num - 1) * page_size >= data.len() {
        return records;
    }
    let page_offset = (page_num - 1) * page_size;
    let page_data = &data[page_offset..std::cmp::min(page_offset + page_size, data.len())];
    
    let btree_offset = if page_num == 1 { 100 } else { 0 };
    if btree_offset + 8 > page_data.len() {
        return records;
    }
    
    let page_type = page_data[btree_offset];
    if page_type != 0x0d {
        return records;
    }
    
    let num_cells = u16::from_be_bytes([page_data[btree_offset + 3], page_data[btree_offset + 4]]) as usize;
    let cell_ptr_start = btree_offset + 8;
    
    for i in 0..num_cells {
        let ptr_offset = cell_ptr_start + i * 2;
        if ptr_offset + 2 > page_data.len() {
            break;
        }
        let cell_offset = u16::from_be_bytes([page_data[ptr_offset], page_data[ptr_offset + 1]]) as usize;
        if cell_offset >= page_data.len() {
            continue;
        }
        
        let mut cell_cursor = cell_offset;
        let payload_size = read_varint(page_data, &mut cell_cursor) as usize;
        let rowid = read_varint_signed(page_data, &mut cell_cursor);
        
        let actual_payload_size = std::cmp::min(payload_size, page_data.len() - cell_cursor);
        let payload = &page_data[cell_cursor..cell_cursor + actual_payload_size];
        
        let record = parse_record(payload, 0);
        records.push((rowid, record));
    }
    records
}

fn load_sqlite_binary(db: &mut InMemoryDb, data: &[u8]) {
    if data.len() < 100 {
        return;
    }
    if &data[0..16] != b"SQLite format 3\0" {
        return;
    }
    let mut page_size = u16::from_be_bytes([data[16], data[17]]) as usize;
    if page_size == 1 {
        page_size = 65536;
    }
    if page_size < 512 || page_size > 65536 || (page_size & (page_size - 1)) != 0 {
        return;
    }
    
    let mut master_leaves = Vec::new();
    collect_leaves(data, 1, page_size, &mut master_leaves);
    
    let mut tables_to_load = Vec::new();
    for leaf in master_leaves {
        let records = parse_leaf_page(data, leaf, page_size);
        for (_rowid, rec) in records {
            if rec.len() >= 5 {
                if let (SqliteValue::Text(ref ty), SqliteValue::Text(ref tbl_name), SqliteValue::Integer(rootpage)) = 
                       (&rec[0], &rec[1], rec[3].clone()) {
                    if ty == "table" && tbl_name != "sqlite_sequence" {
                        let sql_str = match &rec[4] {
                            SqliteValue::Text(s) => s.clone(),
                            _ => String::new(),
                        };
                        tables_to_load.push((tbl_name.to_uppercase(), rootpage as usize, sql_str));
                    }
                }
            }
        }
    }
    
    for (tbl_name, rootpage, sql_str) in tables_to_load {
        let cols_str = match sql_str.find('(') {
            Some(idx) => {
                let rest = &sql_str[idx + 1..];
                match rest.rfind(')') {
                    Some(ridx) => &rest[..ridx],
                    None => rest,
                }
            }
            None => continue,
        };
        let columns: Vec<String> = cols_str.split(',')
            .map(|c| {
                c.trim().split_whitespace().next().unwrap_or("").to_string()
            })
            .filter(|c| {
                let u = c.to_uppercase();
                !u.is_empty() && u != "CONSTRAINT" && u != "PRIMARY" && u != "FOREIGN" && u != "KEY" && u != "UNIQUE" && u != "CHECK"
            })
            .collect();
            
        let mut table_leaves = Vec::new();
        collect_leaves(data, rootpage, page_size, &mut table_leaves);
        
        let mut rows = Vec::new();
        for leaf in table_leaves {
            let records = parse_leaf_page(data, leaf, page_size);
            for (rowid, rec) in records {
                let mut row_strings = Vec::new();
                for (col_idx, col_name) in columns.iter().enumerate() {
                    if col_name.eq_ignore_ascii_case("id") {
                        row_strings.push(rowid.to_string());
                    } else {
                        let val = rec.get(col_idx);
                        match val {
                            Some(SqliteValue::Null) => row_strings.push(String::new()),
                            Some(v) => row_strings.push(v.to_string_val()),
                            None => row_strings.push(String::new()),
                        }
                    }
                }
                rows.push(row_strings);
            }
        }
        
        db.tables.insert(tbl_name, InMemoryTable { columns, rows });
    }
}

pub fn update_bound_data(db_name: &str, field_name: &str, new_val: &str) {
    let uname = db_name.to_uppercase();
    DB_STORE.with(|s| {
        let mut store = s.borrow_mut();
        if let Some(db) = store.get_mut(&uname) {
            let ri = if db.row_cursor > 0 { db.row_cursor - 1 } else { 0 };
            if ri < db.result_rows.len() {
                if let Some(ci) = db.result_cols.iter().position(|c| c.eq_ignore_ascii_case(field_name)) {
                    db.result_rows[ri][ci] = new_val.to_string();
                    
                    if let Some(id_col_idx) = db.result_cols.iter().position(|c| c.eq_ignore_ascii_case("id")) {
                        let id_val = &db.result_rows[ri][id_col_idx];
                        for (_tbl_name, table) in db.tables.iter_mut() {
                            if let (Some(tbl_id_idx), Some(tbl_field_idx)) = (
                                table.columns.iter().position(|c| c.eq_ignore_ascii_case("id")),
                                table.columns.iter().position(|c| c.eq_ignore_ascii_case(field_name))
                            ) {
                                for row in table.rows.iter_mut() {
                                    if row.get(tbl_id_idx) == Some(id_val) {
                                        if tbl_field_idx < row.len() {
                                            row[tbl_field_idx] = new_val.to_string();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });
}

pub fn sync_bound_widgets(db_name: &str) {
    let uname = db_name.to_uppercase();
    let mut field_vals = HashMap::new();
    let has_row = DB_STORE.with(|s| {
        let db = s.borrow();
        if let Some(db) = db.get(&uname) {
            let ri = if db.row_cursor > 0 { db.row_cursor - 1 } else { 0 };
            if ri < db.result_rows.len() {
                for (ci, col) in db.result_cols.iter().enumerate() {
                    if let Some(val) = db.result_rows[ri].get(ci) {
                        field_vals.insert(col.to_uppercase(), val.clone());
                    }
                }
                true
            } else {
                false
            }
        } else {
            false
        }
    });
    
    crate::object_web::rp_sync_bound_widgets(&uname, &field_vals, has_row);
}
