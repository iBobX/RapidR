//! QCGI — RapidQ's CGI object (the manual's Appendix A: Chris Warrington's
//! QCGI.INC 1.6, a RapidQ library; RC.EXE has no QCGI of its own). Made,
//! it takes the CGI variables from the environment (`crate::environ`); its
//! first `Get` (or `Parse`) reads the request's name / value pairs — a GET's
//! QUERY_STRING, a POST's body from standard input — and `Get` hands one
//! back. RapidR's own implementation of that behaviour, compatible with
//! programs written for the library: what it does was observed by running
//! them with RapidQ itself (RC.EXE and QCGI.INC: docs/io-media-plan.md, the
//! `cgi_*` conformance cases):
//!
//! - **Parse** reads at most MaxInput characters: a POST's CONTENT_LENGTH
//!   of them (or MaxInput) from standard input, a GET's QUERY_STRING as it
//!   is when Parse runs (cut to MaxInput); any other method (HEAD, none)
//!   gives no pairs. It runs once.
//! - **Pairs** are split at `&`; a `=` switches from the name to the value
//!   (a second `=` is dropped). With AutoConvert (the default) `%xx` turns
//!   into its character when it's a printable one (space … `~`), else the
//!   `%` stays as it is; `+` is a space, but a `+` before another `+` or a
//!   space stays a `+`. Names are kept in capitals (Get's name too); the
//!   same name again replaces the value; a last pair without a name is
//!   left out; at most 256 pairs.
//! - **Get(Name, Value)** returns 1 and sets Value when the pair is there,
//!   else 0 and leaves Value alone. (The compilers turn the call into
//!   `Value = CGI.__get(Name, Value)`, which keeps whether it was found for
//!   `CGI.__found`: rapidr_ast::library.)
//! - **MaxInput** (32767) takes only a value above 0; **AutoConvert** (1)
//!   is 1 or, set to anything else, 0.
//! - The CGI variables are read-only properties (functions in RapidQ):
//!   Accept (HTTP_ACCEPT), AuthType, ContentLength and ServerPort (numbers:
//!   the text's leading number, as VAL reads it), ContentType, Cookie
//!   (HTTP_COOKIE), GatewayInterface, PathInfo, PathTranslated, Referer
//!   (HTTP_REFERER), RemoteAddr, RemoteHost, RemoteIdent, RemoteUser,
//!   RequestMethod, ScriptName, ServerSoftware, ServerName, ServerProtocol,
//!   QueryString, UserAgent (HTTP_USER_AGENT) — as they were when the
//!   object was made.

use crate::{v_int, v_str, Value};

/// How many pairs a request can hold (RapidQ's limit).
const MAX_PAIRS: usize = 256;
/// MaxInput when the program sets none.
pub const INPUT_DEFAULT: i64 = 32767;

/// The read-only properties and the environment variable each is.
const VARIABLES: &[(&str, &str)] = &[
    ("accept", "HTTP_ACCEPT"),
    ("authtype", "AUTH_TYPE"),
    ("contentlength", "CONTENT_LENGTH"),
    ("contenttype", "CONTENT_TYPE"),
    ("cookie", "HTTP_COOKIE"),
    ("gatewayinterface", "GATEWAY_INTERFACE"),
    ("pathinfo", "PATH_INFO"),
    ("pathtranslated", "PATH_TRANSLATED"),
    ("referer", "HTTP_REFERER"),
    ("remoteaddr", "REMOTE_ADDR"),
    ("remotehost", "REMOTE_HOST"),
    ("remoteident", "REMOTE_IDENT"),
    ("remoteuser", "REMOTE_USER"),
    ("requestmethod", "REQUEST_METHOD"),
    ("scriptname", "SCRIPT_NAME"),
    ("serversoftware", "SERVER_SOFTWARE"),
    ("servername", "SERVER_NAME"),
    ("serverport", "SERVER_PORT"),
    ("serverprotocol", "SERVER_PROTOCOL"),
    ("querystring", "QUERY_STRING"),
    ("useragent", "HTTP_USER_AGENT"),
];

/// Whether `prop` (lowercase) is one of QCGI's read-only CGI variables.
pub fn is_variable(prop: &str) -> bool {
    VARIABLES.iter().any(|(p, _)| *p == prop)
}

/// Reads `n` characters of the request's body (standard input).
pub type BodyReader = fn(usize) -> String;

fn std_body(n: usize) -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::io::Read;
        let mut buf = vec![0u8; n];
        let mut got = 0;
        let mut stdin = std::io::stdin().lock();
        while got < n {
            match stdin.read(&mut buf[got..]) {
                Ok(0) | Err(_) => break,
                Ok(k) => got += k,
            }
        }
        buf.truncate(got);
        buf.iter().map(|&b| char::from(b)).collect()
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = n;
        String::new()
    }
}

thread_local! {
    static BODY: std::cell::Cell<BodyReader> = const { std::cell::Cell::new(std_body) };
}

/// Replaces where a POST's body is read from (standard input by default; a
/// page has none).
pub fn set_body_reader(reader: BodyReader) {
    BODY.with(|b| b.set(reader));
}

/// The number VAL reads at the start of `s` (RapidQ's VAL stops at the
/// first character that can't continue a number: "12abc" is 12).
pub fn leading_number(s: &str) -> f64 {
    let t = s.trim_start();
    let b = t.as_bytes();
    let mut i = 0;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        i += 1;
    }
    let digits = |i: &mut usize| {
        let start = *i;
        while *i < b.len() && b[*i].is_ascii_digit() {
            *i += 1;
        }
        *i > start
    };
    let mut any = digits(&mut i);
    if i < b.len() && b[i] == b'.' {
        i += 1;
        any |= digits(&mut i);
    }
    if !any {
        return 0.0;
    }
    let mantissa_end = i;
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        let mut j = i + 1;
        if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
            j += 1;
        }
        if digits(&mut j) {
            i = j;
        }
    }
    t[..i].parse::<f64>().or_else(|_| t[..mantissa_end].parse::<f64>()).unwrap_or(0.0)
}

#[derive(Debug, Clone)]
pub struct Cgi {
    /// The variables as they were when the object was made.
    vars: Vec<String>,
    pub max_input: i64,
    pub auto_convert: i64,
    /// The pairs, names in capitals, kept sorted by name so a lookup is a
    /// binary search.
    pairs: Vec<(String, String)>,
    parsed: bool,
    /// Whether the last `__get` found its name (`__found`).
    found: bool,
}

impl Cgi {
    /// A QCGI made now: the CGI variables read from the environment.
    pub fn new() -> Self {
        Cgi {
            vars: VARIABLES.iter().map(|(_, env)| crate::environ::get(env)).collect(),
            max_input: INPUT_DEFAULT,
            auto_convert: 1,
            pairs: Vec::new(),
            parsed: false,
            found: false,
        }
    }

    fn var(&self, prop: &str) -> Option<&str> {
        VARIABLES.iter().position(|(p, _)| *p == prop).map(|i| self.vars[i].as_str())
    }

    fn number(&self, prop: &str) -> i64 {
        crate::numeric::round_to_int(leading_number(self.var(prop).unwrap_or("")))
    }

    pub fn get(&self, prop: &str) -> Option<Value> {
        Some(match prop {
            "maxinput" => v_int(self.max_input),
            "autoconvert" => v_int(self.auto_convert),
            "__found" => v_int(i64::from(self.found)),
            "contentlength" | "serverport" => v_int(self.number(prop)),
            _ => v_str(self.var(prop)?),
        })
    }

    /// `None`: not a property of QCGI; `Some(false)`: read-only.
    pub fn set(&mut self, prop: &str, val: &Value) -> Option<bool> {
        match prop {
            "maxinput" => {
                let v = val.to_i64();
                if v > 0 {
                    self.max_input = v;
                }
                Some(true)
            }
            "autoconvert" => {
                self.auto_convert = i64::from(val.to_i64() == 1);
                Some(true)
            }
            _ if is_variable(prop) => Some(false),
            _ => None,
        }
    }

    pub fn call(&mut self, method: &str, args: &[Value]) -> Option<Value> {
        let arg = |i: usize| args.get(i).cloned().unwrap_or(Value::Null);
        match method {
            "parse" => {
                self.parse();
                Some(Value::Null)
            }
            // `Value = CGI.__get(Name, Value)` (rapidr_ast::library): the
            // pair's value, or Value as it was.
            "__get" => {
                let found = self.lookup(&arg(0).to_string_val());
                self.found = found.is_some();
                Some(found.map(Value::String).unwrap_or_else(|| value_or(arg(1))))
            }
            // (`CGI.Get(Name)` not lowered — a value not a variable: only
            // whether it's there)
            "get" => {
                self.found = self.lookup(&arg(0).to_string_val()).is_some();
                Some(v_int(i64::from(self.found)))
            }
            // (the CGI variables read with parentheses, `CGI.ScriptName()`)
            _ if is_variable(method) && args.is_empty() => self.get(method),
            _ => None,
        }
    }

    /// The value of pair `name` (any case), the request read first.
    fn lookup(&mut self, name: &str) -> Option<String> {
        if !self.parsed {
            self.parse();
        }
        let name = name.to_uppercase();
        self.pairs.binary_search_by(|(n, _)| n.as_str().cmp(name.as_str())).ok().map(|i| self.pairs[i].1.clone())
    }

    /// Reads the request's pairs (once).
    pub fn parse(&mut self) {
        if self.parsed {
            return;
        }
        let max = self.max_input.max(0) as usize;
        let method = self.var("requestmethod").unwrap_or("").to_uppercase();
        let qs: Vec<char> = match method.as_str() {
            "POST" => {
                let length = self.number("contentlength");
                let n = if length > self.max_input { max } else { length.max(0) as usize };
                let reader = BODY.with(std::cell::Cell::get);
                reader(n).chars().collect()
            }
            // (the variable as it is now, not as it was when the object was
            // made: RapidQ's Parse sees a QUERY_STRING changed since)
            "GET" => crate::environ::get("QUERY_STRING").chars().take(max).collect(),
            _ => Vec::new(),
        };
        self.parse_pairs(&qs);
        self.parsed = true;
    }

    fn parse_pairs(&mut self, qs: &[char]) {
        if qs.is_empty() {
            return;
        }
        let (mut name, mut value) = (String::new(), String::new());
        let mut in_value = false;
        // (the `num` characters after `index`, or nothing when the text
        // ends sooner)
        let ahead = |index: usize, num: usize| -> Option<&[char]> { qs.get(index + 1..index + 1 + num) };
        let mut i = 0;
        while i < qs.len() {
            let c = qs[i];
            let mut append = |s: &str| if in_value { value.push_str(s) } else { name.push_str(s) };
            match c {
                '&' => {
                    if !self.insert(name.to_uppercase(), std::mem::take(&mut value)) {
                        // (no room for more: the rest is left out)
                        return;
                    }
                    name.clear();
                    in_value = false;
                }
                '=' => in_value = true,
                '%' if self.auto_convert == 1 => match ahead(i, 2).and_then(dehex) {
                    Some(ch) => {
                        append(&ch.to_string());
                        i += 2;
                    }
                    None => append("%"),
                },
                '+' if self.auto_convert == 1 => {
                    let next = ahead(i, 1).map(|n| n[0]);
                    append(if matches!(next, Some('+' | ' ')) { "+" } else { " " });
                }
                other => append(&other.to_string()),
            }
            i += 1;
        }
        if !name.is_empty() {
            self.insert(name.to_uppercase(), value);
        }
    }

    /// Stores one pair: a new name goes in at its sorted place, a name
    /// already there gets the new value; `false` when the table is full
    /// (256 pairs).
    fn insert(&mut self, name: String, value: String) -> bool {
        if self.pairs.len() + 1 > MAX_PAIRS {
            return false;
        }
        match self.pairs.binary_search_by(|(n, _)| n.as_str().cmp(name.as_str())) {
            Ok(i) => self.pairs[i].1 = value,
            Err(i) => self.pairs.insert(i, (name, value)),
        }
        true
    }
}

impl Default for Cgi {
    fn default() -> Self {
        Self::new()
    }
}

/// Get's Value as it was (a missing argument: "").
fn value_or(v: Value) -> Value {
    if matches!(v, Value::Null) {
        v_str("")
    } else {
        v
    }
}

/// `%xx`'s two hex digits as the character they name, when that's a
/// printable one (space … `~`); `None` leaves the `%` as it is.
fn dehex(two: &[char]) -> Option<char> {
    let hi = two.first()?.to_digit(16)?;
    let lo = two.get(1)?.to_digit(16)?;
    let code = hi * 16 + lo;
    (32..=126).contains(&code).then(|| char::from(code as u8))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(qs: &str, auto: i64) -> Cgi {
        let mut c = Cgi { vars: vec![String::new(); VARIABLES.len()], max_input: INPUT_DEFAULT, auto_convert: auto, pairs: Vec::new(), parsed: true, found: false };
        c.parse_pairs(&qs.chars().collect::<Vec<_>>());
        c
    }

    fn value(c: &mut Cgi, name: &str) -> Option<String> {
        c.lookup(name)
    }

    // (RapidQ's own answers: RC.EXE running QCGI.INC 1.6)
    #[test]
    fn pairs_as_the_library_reads_them() {
        let mut c = parsed("name=John+Smith&Age=42&x=%41%42%7e&y=a++b&z=%0Aq&w=%4&e=&noeq&name=Jane&p=%2B%20%zz&t=a=b=c&=lone&&last=%41", 1);
        assert_eq!(value(&mut c, "name").as_deref(), Some("Jane"));
        assert_eq!(value(&mut c, "Age").as_deref(), Some("42"));
        assert_eq!(value(&mut c, "x").as_deref(), Some("AB~"));
        assert_eq!(value(&mut c, "y").as_deref(), Some("a+ b"));
        assert_eq!(value(&mut c, "z").as_deref(), Some("%0Aq"));
        assert_eq!(value(&mut c, "w").as_deref(), Some("%4"));
        assert_eq!(value(&mut c, "e").as_deref(), Some(""));
        assert_eq!(value(&mut c, "noeq").as_deref(), Some(""));
        assert_eq!(value(&mut c, "p").as_deref(), Some("+ %zz"));
        assert_eq!(value(&mut c, "t").as_deref(), Some("abc"));
        assert_eq!(value(&mut c, "").as_deref(), Some(""));
        assert_eq!(value(&mut c, "last").as_deref(), Some("A"));
        assert_eq!(value(&mut c, "missing"), None);
        let mut c = parsed("a=%7E%7f%1F%20%41%c3%A9%4G%G4%%41", 1);
        assert_eq!(value(&mut c, "a").as_deref(), Some("~%7f%1F A%c3%A9%4G%G4%A"));
        let mut c = parsed("a=+&b=++&c=+ x&d=x+&e=+++", 1);
        let got: Vec<_> = ["a", "b", "c", "d", "e"].iter().map(|n| value(&mut c, n).unwrap()).collect();
        assert_eq!(got, [" ", "+ ", "+ x", "x ", "++ "]);
        let mut c = parsed("=x&=y&k=1", 1);
        assert_eq!(value(&mut c, "").as_deref(), Some("y"));
        let mut c = parsed("k=1&=x", 1);
        assert_eq!(value(&mut c, ""), None);
        let mut c = parsed("x=%41+%42&y=1", 0);
        assert_eq!(value(&mut c, "x").as_deref(), Some("%41+%42"));
        let mut c = parsed("my+name=1&f%41o=2&x%3Dy=3", 1);
        assert_eq!(value(&mut c, "my name").as_deref(), Some("1"));
        assert_eq!(value(&mut c, "FAO").as_deref(), Some("2"));
        assert_eq!(value(&mut c, "x=y").as_deref(), Some("3"));
    }

    #[test]
    fn at_most_256_pairs() {
        let qs: Vec<String> = (0..300).map(|i| format!("k{i:03}={i}")).collect();
        let mut c = parsed(&qs.join("&"), 1);
        assert_eq!(c.pairs.len(), 256);
        assert_eq!(value(&mut c, "k255").as_deref(), Some("255"));
        assert_eq!(value(&mut c, "k256"), None);
    }

    #[test]
    fn numbers_and_settings() {
        assert_eq!(leading_number("12abc"), 12.0);
        assert_eq!(leading_number(" -3.5e2x"), -350.0);
        assert_eq!(leading_number("x1"), 0.0);
        assert_eq!(leading_number("1e"), 1.0);
        let mut c = parsed("", 1);
        c.set("maxinput", &v_int(0));
        c.set("autoconvert", &v_int(7));
        assert_eq!((c.max_input, c.auto_convert), (INPUT_DEFAULT, 0));
        assert_eq!(c.set("cookie", &v_str("x")), Some(false));
    }
}
