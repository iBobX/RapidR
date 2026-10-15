//! JSON in and out: a frame from an array of records, JSON Lines or
//! columns; a frame written as an array of records.

use super::super::parse_num;
use super::{DType, Frame};

/// A JSON value, numbers kept as written.
#[derive(Clone, Debug)]
enum Json {
    Null,
    Bool(bool),
    Num(String),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

struct JsonParser<'a> {
    s: &'a str,
    i: usize,
}

impl JsonParser<'_> {
    fn ws(&mut self) {
        while self.s[self.i..].starts_with([' ', '\t', '\n', '\r']) {
            self.i += 1;
        }
    }

    fn eat(&mut self, c: char) -> bool {
        self.ws();
        if self.s[self.i..].starts_with(c) {
            self.i += c.len_utf8();
            true
        } else {
            false
        }
    }

    fn value(&mut self, depth: usize) -> Option<Json> {
        if depth > 64 {
            return None;
        }
        self.ws();
        let rest = &self.s[self.i..];
        if rest.starts_with('{') {
            self.i += 1;
            let mut fields = Vec::new();
            if self.eat('}') {
                return Some(Json::Obj(fields));
            }
            loop {
                self.ws();
                let Some(Json::Str(k)) = self.string() else { return None };
                if !self.eat(':') {
                    return None;
                }
                fields.push((k, self.value(depth + 1)?));
                if self.eat(',') {
                    continue;
                }
                return self.eat('}').then_some(Json::Obj(fields));
            }
        }
        if rest.starts_with('[') {
            self.i += 1;
            let mut items = Vec::new();
            if self.eat(']') {
                return Some(Json::Arr(items));
            }
            loop {
                items.push(self.value(depth + 1)?);
                if self.eat(',') {
                    continue;
                }
                return self.eat(']').then_some(Json::Arr(items));
            }
        }
        if rest.starts_with('"') {
            return self.string();
        }
        for (word, v) in [("true", Json::Bool(true)), ("false", Json::Bool(false)), ("null", Json::Null)] {
            if rest.starts_with(word) {
                self.i += word.len();
                return Some(v);
            }
        }
        let n = rest.find(|c: char| !(c.is_ascii_digit() || "+-.eE".contains(c))).unwrap_or(rest.len());
        if n == 0 {
            return None;
        }
        self.i += n;
        Some(Json::Num(rest[..n].to_string()))
    }

    fn string(&mut self) -> Option<Json> {
        if !self.s[self.i..].starts_with('"') {
            return None;
        }
        self.i += 1;
        let mut out = String::new();
        let mut chars = self.s[self.i..].char_indices();
        while let Some((at, c)) = chars.next() {
            match c {
                '"' => {
                    self.i += at + 1;
                    return Some(Json::Str(out));
                }
                '\\' => {
                    let (_, e) = chars.next()?;
                    match e {
                        'n' => out.push('\n'),
                        't' => out.push('\t'),
                        'r' => out.push('\r'),
                        'b' => out.push('\u{8}'),
                        'f' => out.push('\u{c}'),
                        'u' => {
                            let hex: String = (0..4).filter_map(|_| chars.next().map(|(_, c)| c)).collect();
                            let mut cp = u32::from_str_radix(&hex, 16).ok()?;
                            if (0xD800..0xDC00).contains(&cp) {
                                // (a surrogate pair)
                                let rest = chars.as_str();
                                if let Some(lo) = rest.strip_prefix("\\u").and_then(|r| r.get(..4)).and_then(|h| u32::from_str_radix(h, 16).ok()) {
                                    if (0xDC00..0xE000).contains(&lo) {
                                        cp = 0x10000 + ((cp - 0xD800) << 10) + (lo - 0xDC00);
                                        for _ in 0..6 {
                                            chars.next();
                                        }
                                    }
                                }
                            }
                            out.push(char::from_u32(cp).unwrap_or('\u{fffd}'));
                        }
                        other => out.push(other),
                    }
                }
                _ => out.push(c),
            }
        }
        None
    }
}

fn parse_json(text: &str) -> Option<Json> {
    let mut p = JsonParser { s: text, i: 0 };
    let v = p.value(0)?;
    p.ws();
    (p.i == text.len()).then_some(v)
}

fn json_text(v: &Json) -> String {
    match v {
        Json::Null => "null".into(),
        Json::Bool(b) => b.to_string(),
        Json::Num(n) => n.clone(),
        Json::Str(s) => json_string(s),
        Json::Arr(items) => format!("[{}]", items.iter().map(json_text).collect::<Vec<_>>().join(",")),
        Json::Obj(fields) => format!("{{{}}}", fields.iter().map(|(k, v)| format!("{}:{}", json_string(k), json_text(v))).collect::<Vec<_>>().join(",")),
    }
}

fn json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn json_cell(v: Json) -> Option<String> {
    match v {
        Json::Null => None,
        Json::Bool(b) => Some(b.to_string()),
        Json::Num(n) => Some(n),
        Json::Str(s) => Some(s),
        other => Some(json_text(&other)),
    }
}

/// A frame from JSON: an array of records (`[{"a": 1}, …]`), JSON Lines
/// (one record a line), or columns (`{"a": [1, 2], …}`).
pub fn parse_json_frame(text: &str) -> Option<Frame> {
    let records: Vec<Json> = match parse_json(text.trim_start_matches('\u{feff}')) {
        Some(Json::Arr(items)) => items,
        Some(Json::Obj(fields)) if fields.iter().all(|(_, v)| matches!(v, Json::Arr(_) | Json::Obj(_))) && !fields.is_empty() => {
            let mut f = Frame::default();
            for (name, col) in fields {
                let cells: Vec<Option<String>> = match col {
                    Json::Arr(items) => items.into_iter().map(json_cell).collect(),
                    Json::Obj(items) => items.into_iter().map(|(_, v)| json_cell(v)).collect(),
                    _ => Vec::new(),
                };
                let c = f.add_column(&name);
                f.set_column(c, cells);
            }
            return Some(f);
        }
        Some(obj @ Json::Obj(_)) => vec![obj],
        Some(_) => return None,
        None => {
            let mut items = Vec::new();
            for line in text.lines().filter(|l| !l.trim().is_empty()) {
                items.push(parse_json(line.trim())?);
            }
            items
        }
    };
    let mut f = Frame::default();
    for record in records {
        let mut row: Vec<Option<String>> = vec![None; f.width()];
        match record {
            Json::Obj(fields) => {
                for (k, v) in fields {
                    let c = match f.columns.iter().position(|c| *c == k) {
                        Some(c) => c,
                        None => {
                            let c = f.add_column(&k);
                            row.push(None);
                            c
                        }
                    };
                    row[c] = json_cell(v);
                }
            }
            Json::Arr(items) => {
                for (c, v) in items.into_iter().enumerate() {
                    if c >= f.width() {
                        f.grow(f.height, c + 1);
                        row.resize(f.width(), None);
                    }
                    row[c] = json_cell(v);
                }
            }
            other => {
                f.grow(f.height, 1);
                row.resize(f.width(), None);
                row[0] = json_cell(other);
            }
        }
        f.push_row(&row);
    }
    // (cells pushed one by one were kept as text: typed now where lossless)
    f.retype();
    Some(f)
}

/// The frame as JSON: an array of records, a line each, numbers and
/// booleans as such in columns of those types.
pub fn to_json(f: &Frame) -> String {
    let types: Vec<DType> = (0..f.width()).map(|c| f.dtype(c)).collect();
    let names: Vec<String> = f.columns.iter().map(|c| json_string(c)).collect();
    let rows: Vec<String> = (0..f.height)
        .map(|r| {
            let fields: Vec<String> = (0..f.width())
                .map(|c| {
                    let v = match (f.cell(r, c).as_deref(), types[c]) {
                        (None, _) => "null".to_string(),
                        (Some(s), DType::Int) => s.trim().parse::<i64>().map_or("null".into(), |n| n.to_string()),
                        (Some(s), DType::Float) => parse_num(s).map_or("null".into(), |x| format!("{x}")),
                        (Some(s), DType::Bool) => s.to_ascii_lowercase(),
                        (Some(s), DType::Str) => json_string(s),
                    };
                    format!("{}:{}", names[c], v)
                })
                .collect();
            format!("{{{}}}", fields.join(","))
        })
        .collect();
    if rows.is_empty() {
        "[]\n".to_string()
    } else {
        format!("[\n{}\n]\n", rows.join(",\n"))
    }
}

