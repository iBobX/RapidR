//! SQL in the code editor (REQUEST-001 item 1): the dialects' words, the
//! statement at the caret, and completion from a schema the program gives.
//!
//! - **Dialects.** `sql.toml` holds the words SQLite and MySQL / MariaDB
//!   share; each dialect adds its own ([`keyword_groups`]): the built-in
//!   `sql` has both (as it always coloured them), `sql-sqlite` and
//!   `sql-mysql` one each (RCODEEDITOR's `SqlDialect`).
//! - **Statements** end at a `;` outside strings, quoted names and comments
//!   ([`statement_at`]): what Ctrl+Enter runs when nothing is selected,
//!   and what `[folding] statements` folds.
//! - **Completion** ([`completions`]): after `name.` the columns of that
//!   table (or of the table an alias names: `FROM people p` … `p.`); after
//!   FROM, JOIN, INTO, UPDATE, TABLE the tables; elsewhere the columns of
//!   the tables the statement names, then the tables, then the dialect's
//!   keywords and functions. The tables, views and columns are the
//!   program's ([`Schema`], RCODEEDITOR's `SetSchema`).

use std::ops::Range;

use crate::lang::{Language, TokenKind};
use crate::service::{Completion, CompletionKind};

/// Which SQL's words the editor knows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Dialect {
    /// Every dialect's (the built-in `sql`).
    #[default]
    Any,
    Sqlite,
    Mysql,
}

impl Dialect {
    /// A dialect by name (`SqlDialect`): "sqlite", "mysql" (or "mariadb");
    /// anything else every dialect's words.
    pub fn named(name: &str) -> Dialect {
        match name.trim().to_ascii_lowercase().as_str() {
            "sqlite" | "sqlite3" => Dialect::Sqlite,
            "mysql" | "mariadb" => Dialect::Mysql,
            _ => Dialect::Any,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Dialect::Any => "",
            Dialect::Sqlite => "sqlite",
            Dialect::Mysql => "mysql",
        }
    }

    /// The built-in language with its words.
    pub fn language_id(self) -> &'static str {
        match self {
            Dialect::Any => "sql",
            Dialect::Sqlite => "sql-sqlite",
            Dialect::Mysql => "sql-mysql",
        }
    }
}

/// Whether a language id is SQL's (any dialect).
pub fn is_sql(language_id: &str) -> bool {
    language_id.eq_ignore_ascii_case("sql") || language_id.to_ascii_lowercase().starts_with("sql-")
}

const SQLITE_KEYWORDS: &[&str] = &[
    "PRAGMA", "GLOB", "VACUUM", "AUTOINCREMENT", "ATTACH", "DETACH", "WITHOUT", "ROWID", "STRICT", "REINDEX", "ANALYZE", "INDEXED",
    "ISNULL", "NOTNULL", "VIRTUAL", "CONFLICT", "ABORT", "FAIL", "RAISE", "INSTEAD", "DEFERRABLE", "DEFERRED", "IMMEDIATE", "EXCLUSIVE",
    "REGEXP", "MATCH", "PLAN", "QUERY", "SAVEPOINT", "RELEASE", "GENERATED", "ALWAYS", "STORED", "MATERIALIZED", "NULLS", "FIRST", "LAST",
    "FILTER", "UPSERT", "DO", "NOTHING",
];
const SQLITE_FUNCTIONS: &[&str] = &[
    "LAST_INSERT_ROWID", "CHANGES", "TOTAL_CHANGES", "JULIANDAY", "STRFTIME", "PRINTF", "FORMAT", "TYPEOF", "ZEROBLOB", "RANDOMBLOB",
    "SQLITE_VERSION", "IIF", "UNIXEPOCH", "DATE", "TIME", "DATETIME", "LIKELIHOOD", "HEX", "QUOTE", "CHAR", "UNICODE", "INSTR", "RANDOM",
    "TOTAL", "GROUP_CONCAT", "JSON", "JSON_EXTRACT", "JSON_ARRAY", "JSON_OBJECT", "JSON_EACH", "JSON_TREE", "JSON_GROUP_ARRAY",
    "JSON_GROUP_OBJECT", "SUBSTR", "REPLACE", "ROUND", "NTILE", "FIRST_VALUE", "LAST_VALUE", "NTH_VALUE", "PERCENT_RANK", "CUME_DIST",
];
const MYSQL_KEYWORDS: &[&str] = &[
    "SHOW", "DESCRIBE", "USE", "ENGINE", "CHARSET", "CHARACTER", "AUTO_INCREMENT", "ZEROFILL", "DATABASES", "TABLES", "COLUMNS",
    "PROCESSLIST", "STATUS", "VARIABLES", "LOCK", "UNLOCK", "DUPLICATE", "REGEXP", "RLIKE", "DIV", "XOR", "STRAIGHT_JOIN",
    "SQL_CALC_FOUND_ROWS", "LOW_PRIORITY", "HIGH_PRIORITY", "DELAYED", "PROCEDURE", "FUNCTION", "CALL", "DECLARE", "DELIMITER", "KILL",
    "FLUSH", "GRANT", "REVOKE", "TRUNCATE", "MODIFY", "CHANGE", "FULLTEXT", "SPATIAL", "ALGORITHM", "DEFINER", "INVOKER", "ROUTINE",
    "EVENTS", "PRIVILEGES", "IDENTIFIED", "USER", "WARNINGS", "ERRORS", "GLOBAL", "SESSION", "INTERVAL", "OPTIMIZE", "REPAIR",
];
const MYSQL_TYPES: &[&str] = &[
    "UNSIGNED", "ENUM", "MEDIUMTEXT", "LONGTEXT", "TINYTEXT", "MEDIUMBLOB", "LONGBLOB", "TINYBLOB", "VARBINARY", "BINARY", "YEAR", "BIT",
    "SET", "GEOMETRY", "POINT",
];
const MYSQL_FUNCTIONS: &[&str] = &[
    "NOW", "CONCAT_WS", "DATE_FORMAT", "DATE_ADD", "DATE_SUB", "DATEDIFF", "CURDATE", "CURTIME", "LAST_INSERT_ID", "UUID", "IF",
    "FOUND_ROWS", "ROW_COUNT", "FROM_UNIXTIME", "UNIX_TIMESTAMP", "STR_TO_DATE", "LOCATE", "LPAD", "RPAD", "FORMAT", "MD5", "SHA1",
    "SHA2", "CHAR_LENGTH", "GREATEST", "LEAST", "DATABASE", "VERSION", "CONNECTION_ID", "JSON_EXTRACT", "JSON_OBJECT", "JSON_ARRAY",
    "GROUP_CONCAT", "TIMESTAMPDIFF", "YEAR", "MONTH", "DAY", "HOUR", "MINUTE", "SECOND", "WEEKDAY", "DAYNAME", "MONTHNAME", "INSTR",
    "RAND", "TRUNCATE", "FLOOR", "CEIL", "CEILING", "MOD", "POW", "POWER", "SQRT",
];

/// A dialect's words on top of `sql.toml`'s, as `Language::from_toml_with`
/// takes them (the first group that has a word decides its kind).
pub fn keyword_groups(d: Dialect) -> Vec<(&'static str, &'static [&'static str])> {
    let mut groups = Vec::new();
    if d != Dialect::Mysql {
        groups.push(("keyword", SQLITE_KEYWORDS));
        groups.push(("function", SQLITE_FUNCTIONS));
    }
    if d != Dialect::Sqlite {
        groups.push(("keyword", MYSQL_KEYWORDS));
        groups.push(("type", MYSQL_TYPES));
        groups.push(("function", MYSQL_FUNCTIONS));
    }
    groups
}

/// A dialect's language definition: `sql.toml` as that dialect (its own
/// id and name; `.sql` files stay the built-in `sql`'s).
pub fn dialect_source(base: &str, d: Dialect) -> String {
    let (id, name) = match d {
        Dialect::Any => return base.to_string(),
        Dialect::Sqlite => ("sql-sqlite", "SQL (SQLite)"),
        Dialect::Mysql => ("sql-mysql", "SQL (MySQL)"),
    };
    base.replacen("id = \"sql\"", &format!("id = \"{id}\""), 1).replacen("name = \"SQL\"", &format!("name = \"{name}\""), 1).replacen("extensions = [\"sql\"]", "extensions = []", 1)
}

// ---------------------------------------------------------------------------
// The text: statements, words
// ---------------------------------------------------------------------------

/// Where `text`'s statements end: each `;` outside strings ('…'), quoted
/// names ("…", `…`, […]) and comments (-- …, /* … */).
pub fn terminators(text: &str) -> Vec<usize> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'\'' | b'"' | b'`' => {
                let q = b[i];
                i += 1;
                while i < b.len() {
                    if b[i] == q {
                        // (a doubled quote is one inside)
                        if b.get(i + 1) == Some(&q) {
                            i += 2;
                            continue;
                        }
                        break;
                    }
                    i += 1;
                }
            }
            b'-' if b.get(i + 1) == Some(&b'-') => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                i += 2;
                while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                    i += 1;
                }
                i += 1;
            }
            b';' => out.push(i),
            _ => {}
        }
        i += 1;
    }
    out
}

/// The statement byte `at` is in: from after the `;` before it to its own
/// `;` (included), blanks trimmed. Between statements (only blanks after a
/// `;`), the statement just ended.
pub fn statement_at(text: &str, at: usize) -> Range<usize> {
    let ends = terminators(text);
    let at = at.min(text.len());
    let piece = |k: usize| -> Range<usize> {
        let start = if k == 0 { 0 } else { ends[k - 1] + 1 };
        let end = ends.get(k).map_or(text.len(), |e| e + 1);
        trim(text, start..end)
    };
    // (the piece the caret is in: its `;` at or after it)
    let k = ends.iter().position(|&e| e >= at).unwrap_or(ends.len());
    let r = piece(k);
    if r.is_empty() && k > 0 && text[ends[k - 1] + 1..at].trim().is_empty() {
        return piece(k - 1);
    }
    r
}

fn trim(text: &str, r: Range<usize>) -> Range<usize> {
    let s = &text[r.clone()];
    let start = r.start + (s.len() - s.trim_start().len());
    let end = r.end - (s.len() - s.trim_end().len());
    if start >= end {
        start..start
    } else {
        start..end
    }
}

fn word_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'$' || c >= 0x80
}

/// A statement's words and punctuation, comments and strings left out
/// (a quoted name is its name).
fn words(sql: &str) -> Vec<String> {
    let b = sql.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if word_byte(c) {
            let s = i;
            while i < b.len() && word_byte(b[i]) {
                i += 1;
            }
            out.push(sql[s..i].to_string());
            continue;
        }
        match c {
            b'"' | b'`' | b'[' => {
                let close = if c == b'[' { b']' } else { c };
                let s = i + 1;
                i += 1;
                while i < b.len() && b[i] != close {
                    i += 1;
                }
                out.push(sql[s..i.min(b.len())].to_string());
            }
            b'\'' => {
                i += 1;
                while i < b.len() && !(b[i] == b'\'' && b.get(i + 1) != Some(&b'\'')) {
                    i += if b[i] == b'\'' { 2 } else { 1 };
                }
            }
            b'-' if b.get(i + 1) == Some(&b'-') => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                i += 2;
                while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                    i += 1;
                }
                i += 1;
            }
            b'.' | b',' | b'(' | b')' => out.push((c as char).to_string()),
            _ => {}
        }
        i += 1;
    }
    out
}

/// The words after which a table's name comes.
const TABLE_BEFORE: &[&str] = &["FROM", "JOIN", "INTO", "UPDATE", "TABLE", "DESCRIBE", "DESC"];

/// The tables a statement names, with their aliases: (table, alias).
fn tables_named(sql: &str, lang: &Language) -> Vec<(String, Option<String>)> {
    let w = words(sql);
    let is_kw = |s: &str| lang.keyword_kind(s).is_some();
    let mut out = Vec::new();
    let mut i = 0;
    while i < w.len() {
        let up = w[i].to_ascii_uppercase();
        let mut listing = TABLE_BEFORE.contains(&up.as_str());
        let mut j = i + 1;
        while listing && j < w.len() {
            // (a schema's prefix: main.people)
            let mut name = w[j].clone();
            // (a subquery: no table here)
            if name == "(" || name.eq_ignore_ascii_case("SELECT") {
                break;
            }
            if w.get(j + 1).map(String::as_str) == Some(".") && w.get(j + 2).is_some() {
                name = w[j + 2].clone();
                j += 2;
            }
            if !word_byte(name.as_bytes().first().copied().unwrap_or(b' ')) {
                break;
            }
            j += 1;
            let mut alias = None;
            if w.get(j).is_some_and(|s| s.eq_ignore_ascii_case("AS")) {
                j += 1;
            }
            if let Some(a) = w.get(j) {
                if a.as_bytes().first().copied().is_some_and(word_byte) && !is_kw(a) {
                    alias = Some(a.clone());
                    j += 1;
                }
            }
            out.push((name, alias));
            // (FROM a x, b y: another table after a comma)
            listing = up == "FROM" && w.get(j).map(String::as_str) == Some(",");
            j += 1;
        }
        i += 1;
    }
    out
}

// ---------------------------------------------------------------------------
// The schema and completion
// ---------------------------------------------------------------------------

/// The tables (and views) completion offers, with their columns.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Schema {
    pub tables: Vec<Table>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Table {
    pub name: String,
    /// "table" or "view".
    pub kind: String,
    pub columns: Vec<Column>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Column {
    pub name: String,
    /// Its type as the database says it (INTEGER, VARCHAR(40) …; "" none).
    pub kind: String,
}

impl Schema {
    /// The table (or view) `name` (any case).
    pub fn table(&self, name: &str) -> Option<&Table> {
        self.tables.iter().find(|t| t.name.eq_ignore_ascii_case(name))
    }
}

/// The completions at byte `at` of `text` (an SQL editor's), and where the
/// word they replace starts.
pub fn completions(text: &str, at: usize, schema: &Schema, lang: &Language) -> (usize, Vec<Completion>) {
    let at = at.min(text.len());
    let b = text.as_bytes();
    let mut start = at;
    while start > 0 && word_byte(b[start - 1]) {
        start -= 1;
    }
    let stmt = statement_at(text, at);
    let stmt_text = &text[stmt.start.min(start)..stmt.end.max(at)];
    let named = tables_named(stmt_text, lang);
    let column = |t: &Table, c: &Column, sort: &str| {
        let mut item = Completion::new(c.name.clone(), CompletionKind::Field);
        item.detail = if c.kind.is_empty() { t.name.clone() } else { format!("{} · {}", t.name, c.kind) };
        item.sort = format!("{sort}{}", c.name.to_lowercase());
        item
    };
    // `name.`: that table's columns (an alias's table's)
    if start > 0 && b[start - 1] == b'.' {
        let mut q = start - 1;
        while q > 0 && word_byte(b[q - 1]) {
            q -= 1;
        }
        let qualifier = text[q..start - 1].trim_matches(['"', '`']);
        let table_name = named.iter().find(|(_, a)| a.as_deref().is_some_and(|a| a.eq_ignore_ascii_case(qualifier))).map(|(t, _)| t.as_str()).unwrap_or(qualifier);
        let items = schema.table(table_name).map(|t| t.columns.iter().map(|c| column(t, c, "1")).collect()).unwrap_or_default();
        return (start, items);
    }
    let table_item = |t: &Table| {
        let mut item = Completion::new(t.name.clone(), CompletionKind::Type);
        item.detail = format!("{} · {} columns", if t.kind.is_empty() { "table" } else { &t.kind }, t.columns.len());
        item.sort = format!("2{}", t.name.to_lowercase());
        item
    };
    // (the word before: a table's place?)
    let before = &text[stmt.start.min(start)..start];
    let prev = words(before).pop().map(|w| w.to_ascii_uppercase());
    let mut items: Vec<Completion> = Vec::new();
    if prev.as_deref().is_some_and(|p| TABLE_BEFORE.contains(&p)) || prev.as_deref() == Some(",") && in_from_list(before) {
        items.extend(schema.tables.iter().map(|t| {
            let mut item = table_item(t);
            item.sort.replace_range(0..1, "1");
            item
        }));
    } else {
        // the named tables' columns (every table's when it names none)
        let mut seen = std::collections::HashSet::new();
        let tables: Vec<&Table> = if named.is_empty() { schema.tables.iter().collect() } else { named.iter().filter_map(|(t, _)| schema.table(t)).collect() };
        for t in tables {
            for c in &t.columns {
                if seen.insert(c.name.to_lowercase()) {
                    items.push(column(t, c, "1"));
                }
            }
        }
        items.extend(schema.tables.iter().map(table_item));
    }
    for (word, kind) in lang.keywords() {
        let (k, sort) = if kind.base() == TokenKind::FUNCTION { (CompletionKind::Function, "4") } else { (CompletionKind::Keyword, "3") };
        let mut item = Completion::new(word.to_string(), k);
        item.sort = format!("{sort}{}", word.to_lowercase());
        if k == CompletionKind::Function {
            item.detail = "function".into();
        }
        items.push(item);
    }
    (start, items)
}

/// Whether the text before a `,` is a FROM list (`FROM a, ` … a table).
fn in_from_list(before: &str) -> bool {
    let w = words(before);
    for word in w.iter().rev() {
        let up = word.to_ascii_uppercase();
        if up == "FROM" {
            return true;
        }
        if matches!(up.as_str(), "SELECT" | "WHERE" | "ON" | "SET" | "VALUES" | "(" | ")" | "GROUP" | "ORDER" | "BY") {
            return false;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Languages;

    fn schema() -> Schema {
        let col = |n: &str, k: &str| Column { name: n.into(), kind: k.into() };
        Schema {
            tables: vec![
                Table { name: "people".into(), kind: "table".into(), columns: vec![col("id", "INTEGER"), col("name", "TEXT"), col("city", "TEXT")] },
                Table { name: "orders".into(), kind: "table".into(), columns: vec![col("id", "INTEGER"), col("person_id", "INTEGER"), col("total", "REAL")] },
            ],
        }
    }

    fn labels(text: &str, lang: &str) -> Vec<String> {
        let at = text.find('|').unwrap();
        let text = text.replace('|', "");
        let l = Languages::builtin().get(lang).unwrap();
        let (start, items) = completions(&text, at, &schema(), &l);
        assert!(start <= at);
        let mut items: Vec<(String, String)> = items.into_iter().map(|i| (i.sort, i.label)).collect();
        items.sort();
        items.into_iter().map(|(_, l)| l).collect()
    }

    #[test]
    fn statements_end_at_semicolons_outside_literals() {
        let t = "SELECT ';' FROM a; -- x;y\n  SELECT \"a;b\" /* ; */ FROM b ;\n\n";
        assert_eq!(terminators(t).len(), 2);
        assert_eq!(&t[statement_at(t, 3)], "SELECT ';' FROM a;");
        assert_eq!(&t[statement_at(t, 40)], "-- x;y\n  SELECT \"a;b\" /* ; */ FROM b ;");
        // (after the last `;`: the statement just ended)
        assert_eq!(&t[statement_at(t, t.len())], "-- x;y\n  SELECT \"a;b\" /* ; */ FROM b ;");
        // (a statement without its `;` yet)
        let t = "SELECT 1;\nSELECT name FROM people WHERE";
        assert_eq!(&t[statement_at(t, t.len())], "SELECT name FROM people WHERE");
    }

    #[test]
    fn completes_columns_tables_and_words() {
        // after a dot: the alias's table's columns
        assert_eq!(labels("SELECT p.| FROM people p JOIN orders AS o ON o.person_id = p.id", "sql"), ["city", "id", "name"]);
        assert_eq!(labels("SELECT o.t| FROM people p JOIN orders AS o", "sql"), ["id", "person_id", "total"]);
        assert_eq!(labels("SELECT orders.| FROM orders", "sql"), ["id", "person_id", "total"]);
        // after FROM / JOIN: the tables first
        let l = labels("SELECT * FROM |", "sql");
        assert_eq!(&l[..2], ["orders", "people"]);
        let l = labels("SELECT * FROM people, o|", "sql");
        assert_eq!(&l[..2], ["orders", "people"]);
        // elsewhere: the named tables' columns, the tables, the words
        let l = labels("SELECT n| FROM people", "sql-sqlite");
        assert_eq!(&l[..5], ["city", "id", "name", "orders", "people"]);
        assert!(l.contains(&"PRAGMA".to_string()) && l.contains(&"SELECT".to_string()) && l.contains(&"JULIANDAY".to_string()));
        assert!(!l.contains(&"SHOW".to_string()));
        let l = labels("SELECT | FROM orders", "sql-mysql");
        assert_eq!(&l[..3], ["id", "person_id", "total"]);
        assert!(l.contains(&"SHOW".to_string()) && l.contains(&"DATE_FORMAT".to_string()) && !l.contains(&"PRAGMA".to_string()));
    }

    #[test]
    fn dialects_colour_their_own_words() {
        let langs = Languages::builtin();
        let (any, lite, my) = (langs.get("sql").unwrap(), langs.get("sql-sqlite").unwrap(), langs.get("sql-mysql").unwrap());
        assert!(any.keyword_kind("PRAGMA").is_some() && any.keyword_kind("SHOW").is_some());
        assert!(lite.keyword_kind("pragma").is_some() && lite.keyword_kind("SHOW").is_none());
        assert!(my.keyword_kind("SHOW").is_some() && my.keyword_kind("PRAGMA").is_none());
        assert!(lite.keyword_kind("SELECT").is_some() && my.keyword_kind("select").is_some());
        assert_eq!(langs.for_path("q.sql").id, "sql");
        assert_eq!((Dialect::named("MariaDB"), Dialect::named("sqlite"), Dialect::named("")), (Dialect::Mysql, Dialect::Sqlite, Dialect::Any));
        assert!(is_sql("sql-mysql") && is_sql("SQL") && !is_sql("sqlx"));
    }
}
