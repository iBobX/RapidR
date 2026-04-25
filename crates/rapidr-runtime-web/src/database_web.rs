//! RSQLite web runtime — in-memory table store with basic SQL parsing.
//!
//! No actual SQLite or rusqlite — we store tables as `Vec<Vec<String>>`
//! and parse simple SQL statements: CREATE TABLE, INSERT, SELECT, UPDATE, DELETE.

use crate::object_web;
use crate::value::{v_int, v_null, v_str, Value};
use std::cell::RefCell;
use std::collections::HashMap;

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
            // On web, always creates an in-memory database
            let db_name = args.first().map(|v| v.to_string_val()).unwrap_or_else(|| ":memory:".to_string());
            let existed = DB_STORE.with(|s| s.borrow().contains_key(&uname));
            if !existed {
                DB_STORE.with(|s| s.borrow_mut().insert(uname.clone(), InMemoryDb::new()));
            }
            object_web::rp_comp_set(&uname, "connected", v_int(1));
            object_web::rp_comp_set(&uname, "db", v_str(&db_name));
            v_int(if existed { 1 } else { 0 })
        }
        "close" => {
            DB_STORE.with(|s| s.borrow_mut().remove(&uname));
            object_web::rp_comp_set(&uname, "connected", v_int(0));
            v_null()
        }
        "query" | "exec" => {
            let sql = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            execute_sql(&uname, &sql)
        }
        "fetchrow" => {
            DB_STORE.with(|s| {
                let mut store = s.borrow_mut();
                if let Some(db) = store.get_mut(&uname) {
                    if db.row_cursor < db.result_rows.len() {
                        db.field_cursor = 0;
                        db.row_cursor += 1;
                        v_int(1)
                    } else {
                        v_int(0)
                    }
                } else {
                    v_int(0)
                }
            })
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
