//! Programs in every language (RapidR's): `TR$`, `TRN$` and
//! `Application.Language`, one implementation for every runtime
//! (docs/program-languages-plan.md §1–2, lane T1).
//!
//! - **Catalogs** are PO files, one per language, in a `lang` folder beside
//!   the program (`lang/es.po`, `lang/pt-BR.po`, `lang/zh-Hans.po`): the
//!   folder of the program file the RapidR Runtime runs (`rapidr run
//!   prog.bas`), of a built executable, or the web page's. RapidR reads
//!   them with its own small parser ([`parse_po`]); no gettext code.
//! - **The language** is the system's until the program sets
//!   `Application.Language`; `es-AR` looks in `es-AR.po`, then `es.po`,
//!   then gives the source text ([`fallback_chain`]).
//! - **Plurals** follow the catalog's `Plural-Forms` header ([`Plural`]),
//!   English's rule (`n != 1`) when it has none or for the source text.
//! - **Placeholders** `{name}` are filled by name ([`fill`]); `{{` and `}}`
//!   are literal braces; a name nobody gave stays as written.
//!
//! A missing folder, file or entry is never an error: the text is the
//! program's own. A bad line in a file is skipped; the good entries load.

use crate::{v_str, Value};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

// ------------------------------------------------------------ plural rules --

/// A `Plural-Forms` header: how many forms a language has and which one a
/// count takes (gettext's C-like expression of `n`).
#[derive(Clone, Debug, PartialEq)]
pub struct Plural {
    pub nplurals: usize,
    expr: Expr,
}

#[derive(Clone, Debug, PartialEq)]
enum Expr {
    N,
    Num(u64),
    Not(Box<Expr>),
    Bin(Op, Box<Expr>, Box<Expr>),
    Cond(Box<Expr>, Box<Expr>, Box<Expr>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Op {
    Or,
    And,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

impl Default for Plural {
    /// English's (and the source text's): one form for 1, another for the rest.
    fn default() -> Self {
        Plural { nplurals: 2, expr: Expr::Bin(Op::Ne, Box::new(Expr::N), Box::new(Expr::Num(1))) }
    }
}

impl Plural {
    /// Parses a header's value: `nplurals=3; plural=(n==1 ? 0 : …);`. None
    /// when it isn't one (the caller keeps English's rule).
    pub fn parse(header: &str) -> Option<Plural> {
        let mut nplurals = None;
        let mut expr = None;
        for part in header.split(';') {
            let Some((key, value)) = part.split_once('=') else { continue };
            match key.trim().to_ascii_lowercase().as_str() {
                "nplurals" => nplurals = value.trim().parse::<usize>().ok(),
                "plural" => expr = parse_expr(value.trim()),
                _ => {}
            }
        }
        let nplurals = nplurals.filter(|n| (1..=32).contains(n))?;
        Some(Plural { nplurals, expr: expr? })
    }

    /// The form a count takes, 0 to `nplurals - 1` (an index the
    /// expression gives past the last is the last).
    pub fn index(&self, n: u64) -> usize {
        let i = eval(&self.expr, n);
        (i.min(self.nplurals as u64 - 1)) as usize
    }
}

fn eval(e: &Expr, n: u64) -> u64 {
    match e {
        Expr::N => n,
        Expr::Num(v) => *v,
        Expr::Not(a) => u64::from(eval(a, n) == 0),
        Expr::Cond(c, a, b) => {
            if eval(c, n) != 0 {
                eval(a, n)
            } else {
                eval(b, n)
            }
        }
        Expr::Bin(op, a, b) => {
            let x = eval(a, n);
            // (&& and || don't evaluate their right side when the left decides)
            match op {
                Op::And => return u64::from(x != 0 && eval(b, n) != 0),
                Op::Or => return u64::from(x != 0 || eval(b, n) != 0),
                _ => {}
            }
            let y = eval(b, n);
            match op {
                Op::Eq => u64::from(x == y),
                Op::Ne => u64::from(x != y),
                Op::Lt => u64::from(x < y),
                Op::Le => u64::from(x <= y),
                Op::Gt => u64::from(x > y),
                Op::Ge => u64::from(x >= y),
                Op::Add => x.wrapping_add(y),
                Op::Sub => x.wrapping_sub(y),
                Op::Mul => x.wrapping_mul(y),
                Op::Div => x.checked_div(y).unwrap_or(0),
                Op::Rem => x.checked_rem(y).unwrap_or(0),
                Op::And | Op::Or => unreachable!(),
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Tok {
    N,
    Num(u64),
    Op(&'static str),
}

fn tokens(s: &str) -> Option<Vec<Tok>> {
    const OPS: [&str; 19] = ["||", "&&", "==", "!=", "<=", ">=", "<", ">", "+", "-", "*", "/", "%", "!", "?", ":", "(", ")", ";"];
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c.is_ascii_whitespace() {
            i += 1;
        } else if c == b'n' {
            out.push(Tok::N);
            i += 1;
        } else if c.is_ascii_digit() {
            let start = i;
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
            out.push(Tok::Num(s[start..i].parse().ok()?));
        } else {
            let op = OPS.iter().find(|op| s[i..].starts_with(**op))?;
            if *op == ";" {
                break;
            }
            out.push(Tok::Op(op));
            i += op.len();
        }
    }
    Some(out)
}

/// gettext's plural expression, by precedence climbing (C's precedence:
/// `?:` lowest, then `||`, `&&`, `== !=`, `< <= > >=`, `+ -`, `* / %`,
/// `!`). None when it doesn't parse, or nests deeper than any real rule.
fn parse_expr(s: &str) -> Option<Expr> {
    let toks = tokens(s)?;
    let mut p = ExprParser { toks: &toks, pos: 0, depth: 0 };
    let e = p.ternary()?;
    (p.pos == toks.len()).then_some(e)
}

struct ExprParser<'a> {
    toks: &'a [Tok],
    pos: usize,
    depth: usize,
}

impl ExprParser<'_> {
    fn peek_op(&self) -> Option<&'static str> {
        match self.toks.get(self.pos) {
            Some(Tok::Op(op)) => Some(op),
            _ => None,
        }
    }

    fn eat(&mut self, op: &str) -> bool {
        if self.peek_op() == Some(op) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn ternary(&mut self) -> Option<Expr> {
        self.depth += 1;
        if self.depth > 64 {
            return None;
        }
        let cond = self.binary(0)?;
        let e = if self.eat("?") {
            let a = self.ternary()?;
            if !self.eat(":") {
                return None;
            }
            let b = self.ternary()?;
            Expr::Cond(Box::new(cond), Box::new(a), Box::new(b))
        } else {
            cond
        };
        self.depth -= 1;
        Some(e)
    }

    fn binary(&mut self, level: usize) -> Option<Expr> {
        const LEVELS: [&[(&str, Op)]; 6] = [
            &[("||", Op::Or)],
            &[("&&", Op::And)],
            &[("==", Op::Eq), ("!=", Op::Ne)],
            &[("<", Op::Lt), ("<=", Op::Le), (">", Op::Gt), (">=", Op::Ge)],
            &[("+", Op::Add), ("-", Op::Sub)],
            &[("*", Op::Mul), ("/", Op::Div), ("%", Op::Rem)],
        ];
        if level == LEVELS.len() {
            return self.unary();
        }
        let mut left = self.binary(level + 1)?;
        while let Some(op) = self.peek_op().and_then(|t| LEVELS[level].iter().find(|(s, _)| *s == t).map(|(_, o)| *o)) {
            self.pos += 1;
            let right = self.binary(level + 1)?;
            left = Expr::Bin(op, Box::new(left), Box::new(right));
        }
        Some(left)
    }

    fn unary(&mut self) -> Option<Expr> {
        if self.eat("!") {
            self.depth += 1;
            if self.depth > 64 {
                return None;
            }
            let e = Expr::Not(Box::new(self.unary()?));
            self.depth -= 1;
            return Some(e);
        }
        match self.toks.get(self.pos)? {
            Tok::N => {
                self.pos += 1;
                Some(Expr::N)
            }
            Tok::Num(v) => {
                self.pos += 1;
                Some(Expr::Num(*v))
            }
            Tok::Op("(") => {
                self.pos += 1;
                let e = self.ternary()?;
                self.eat(")").then_some(e)
            }
            Tok::Op(_) => None,
        }
    }
}

// --------------------------------------------------------------- catalogs --

/// One language's translations: (context, source text) → its forms (one,
/// or one per plural form), and its plural rule.
#[derive(Debug, Default)]
pub struct Catalog {
    pub plural: Plural,
    /// Context ("" for none) → source text → forms.
    entries: HashMap<String, HashMap<String, Vec<String>>>,
}

impl Catalog {
    /// The forms of `id` in `context` (None when it has no translation).
    pub fn forms(&self, context: &str, id: &str) -> Option<&[String]> {
        self.entries.get(context)?.get(id).map(Vec::as_slice)
    }

    /// How many entries it has.
    pub fn len(&self) -> usize {
        self.entries.values().map(HashMap::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// `id`'s translation (its first form), when it has a non-empty one.
    pub fn text(&self, context: &str, id: &str) -> Option<&str> {
        self.forms(context, id)?.first().map(String::as_str).filter(|s| !s.is_empty())
    }

    /// `id`'s translation for count `n`, by this language's plural rule
    /// (when that form is there and not empty).
    pub fn plural_text(&self, context: &str, id: &str, n: u64) -> Option<&str> {
        let forms = self.forms(context, id)?;
        let i = self.plural.index(n).min(forms.len().checked_sub(1)?);
        forms.get(i).map(String::as_str).filter(|s| !s.is_empty())
    }
}

#[derive(Default)]
struct PoEntry {
    context: Option<String>,
    id: Option<String>,
    plural_id: Option<String>,
    forms: Vec<String>,
    fuzzy: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Field {
    Context,
    Id,
    PluralId,
    Str(usize),
}

impl PoEntry {
    fn field(&mut self, f: Field) -> &mut String {
        match f {
            Field::Context => self.context.get_or_insert_with(String::new),
            Field::Id => self.id.get_or_insert_with(String::new),
            Field::PluralId => self.plural_id.get_or_insert_with(String::new),
            Field::Str(i) => {
                if self.forms.len() <= i {
                    self.forms.resize(i + 1, String::new());
                }
                &mut self.forms[i]
            }
        }
    }
}

/// A PO file's text as a catalog. Tolerant: a line it can't read is
/// skipped (and a string continuing it is not taken), an entry without a
/// source text or a translation is left out, entries marked `#, fuzzy` are
/// not used (as gettext's tools do); never a panic. The header entry
/// (`msgid ""`) gives the plural rule.
pub fn parse_po(text: &str) -> Catalog {
    let mut cat = Catalog::default();
    let mut entry = PoEntry::default();
    let mut field: Option<Field> = None;
    let text = text.strip_prefix('\u{FEFF}').unwrap_or(text);
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(comment) = line.strip_prefix('#') {
            // (the comments before an entry are its: the one before ends)
            if entry.id.is_some() || !entry.forms.is_empty() {
                finish(&mut cat, std::mem::take(&mut entry));
            }
            field = None;
            if let Some(flags) = comment.strip_prefix(',') {
                if flags.split(',').any(|f| f.trim() == "fuzzy") {
                    entry.fuzzy = true;
                }
            }
            continue;
        }
        if line.starts_with('"') {
            match (field, parse_string(line)) {
                (Some(f), Some(s)) => entry.field(f).push_str(&s),
                _ => field = None,
            }
            continue;
        }
        let (keyword, rest) = line.split_once(|c: char| c.is_whitespace() || c == '"').map_or((line, ""), |(k, _)| (k, &line[k.len()..]));
        let f = match keyword {
            "msgctxt" => Field::Context,
            "msgid" => Field::Id,
            "msgid_plural" => Field::PluralId,
            "msgstr" => Field::Str(0),
            k => match k.strip_prefix("msgstr[").and_then(|r| r.strip_suffix(']')).and_then(|i| i.trim().parse::<usize>().ok()) {
                Some(i) if i < 32 => Field::Str(i),
                _ => {
                    field = None;
                    continue;
                }
            },
        };
        // A new entry starts at its context, or at its source text when the
        // one before has one (or a translation) already.
        let starts = match f {
            Field::Context => entry.id.is_some() || entry.context.is_some() || !entry.forms.is_empty(),
            Field::Id => entry.id.is_some() || !entry.forms.is_empty(),
            _ => false,
        };
        if starts {
            finish(&mut cat, std::mem::take(&mut entry));
        }
        let Some(s) = parse_string(rest.trim()) else {
            // (an unreadable source text: what follows it, until the next
            // entry, is nobody's)
            field = None;
            continue;
        };
        *entry.field(f) = s;
        field = Some(f);
    }
    finish(&mut cat, entry);
    cat
}

fn finish(cat: &mut Catalog, e: PoEntry) {
    let Some(id) = e.id else { return };
    let context = e.context.unwrap_or_default();
    if id.is_empty() && context.is_empty() {
        // The header: `Plural-Forms: nplurals=2; plural=(n != 1);`.
        let header = e.forms.first().map(String::as_str).unwrap_or("");
        for line in header.lines() {
            if let Some((key, value)) = line.split_once(':') {
                if key.trim().eq_ignore_ascii_case("plural-forms") {
                    if let Some(p) = Plural::parse(value) {
                        cat.plural = p;
                    }
                }
            }
        }
        return;
    }
    if e.fuzzy || e.forms.iter().all(String::is_empty) {
        return;
    }
    cat.entries.entry(context).or_default().insert(id, e.forms);
}

/// A PO string: `"..."` with C's escapes (`\n \t \r \" \\`, and `\a \b
/// \f \v`, octal `\ooo`, hex `\xHH`); None when it isn't one (no closing
/// quote, or something after it other than a comment).
fn parse_string(s: &str) -> Option<String> {
    let body = s.strip_prefix('"')?;
    let mut out = String::with_capacity(body.len());
    let mut chars = body.char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => {
                let after = body[i + 1..].trim();
                return (after.is_empty() || after.starts_with('#')).then_some(out);
            }
            '\\' => {
                let (_, e) = chars.next()?;
                match e {
                    'n' => out.push('\n'),
                    't' => out.push('\t'),
                    'r' => out.push('\r'),
                    'a' => out.push('\u{7}'),
                    'b' => out.push('\u{8}'),
                    'f' => out.push('\u{c}'),
                    'v' => out.push('\u{b}'),
                    '0'..='7' => {
                        let mut v = e.to_digit(8)?;
                        for _ in 0..2 {
                            match chars.clone().next() {
                                Some((_, d @ '0'..='7')) => {
                                    v = v * 8 + d.to_digit(8)?;
                                    chars.next();
                                }
                                _ => break,
                            }
                        }
                        out.push(char::from_u32(v)?);
                    }
                    'x' => {
                        let mut v = 0u32;
                        let mut digits = 0;
                        while let Some((_, d)) = chars.clone().next().filter(|(_, d)| d.is_ascii_hexdigit() && digits < 2) {
                            v = v * 16 + d.to_digit(16)?;
                            digits += 1;
                            chars.next();
                        }
                        if digits == 0 {
                            return None;
                        }
                        out.push(char::from_u32(v)?);
                    }
                    other => out.push(other),
                }
            }
            c => out.push(c),
        }
    }
    None
}

// ----------------------------------------------------------- placeholders --

/// `text` with each `{name}` replaced by `value(name)`: `{{` and `}}` are
/// literal braces; a name `value` doesn't know (None) stays as written.
pub fn fill(text: &str, value: impl Fn(&str) -> Option<String>) -> String {
    if !text.contains(['{', '}']) {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len() + 16);
    let mut rest = text;
    while let Some(i) = rest.find(['{', '}']) {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        if tail.starts_with("{{") || tail.starts_with("}}") {
            out.push(tail.as_bytes()[0] as char);
            rest = &tail[2..];
        } else if let Some(after) = tail.strip_prefix('}') {
            out.push('}');
            rest = after;
        } else {
            // `{name}`: up to the next brace, which must close it.
            match tail[1..].find(['{', '}']) {
                Some(j) if tail.as_bytes()[1 + j] == b'}' => {
                    let name = &tail[1..1 + j];
                    match value(name.trim()) {
                        Some(v) => out.push_str(&v),
                        None => out.push_str(&tail[..j + 2]),
                    }
                    rest = &tail[j + 2..];
                }
                _ => {
                    out.push('{');
                    rest = &tail[1..];
                }
            }
        }
    }
    out.push_str(rest);
    out
}

/// The value of placeholder `name` among `pairs` (name, value, name,
/// value …; names in any case).
fn pair_value(pairs: &[Value], name: &str) -> Option<String> {
    pairs.chunks(2).find(|p| p[0].to_string_val().trim().eq_ignore_ascii_case(name)).map(|p| p.get(1).map(Value::to_string_val).unwrap_or_default())
}

// -------------------------------------------------------------- languages --

/// A language tag as RapidR writes it: `es-AR`, `zh-Hans-CN`, `pt-BR` —
/// from `es_ar`, `ES-ar`, a system's `es_AR.UTF-8@euro`. "" for none
/// (`C`, `POSIX`, empty).
pub fn normalize_tag(tag: &str) -> String {
    let tag = tag.trim();
    let tag = tag.split(['.', '@']).next().unwrap_or("");
    if tag.eq_ignore_ascii_case("c") || tag.eq_ignore_ascii_case("posix") {
        return String::new();
    }
    let mut out = String::new();
    for (i, part) in tag.split(['-', '_']).filter(|p| !p.is_empty()).enumerate() {
        if !part.chars().all(|c| c.is_ascii_alphanumeric()) {
            break;
        }
        if i > 0 {
            out.push('-');
        }
        if i == 0 {
            out.push_str(&part.to_ascii_lowercase());
        } else if part.len() == 4 && part.chars().all(|c| c.is_ascii_alphabetic()) {
            // a script: Hans, Latn
            out.push_str(&part[..1].to_ascii_uppercase());
            out.push_str(&part[1..].to_ascii_lowercase());
        } else if (part.len() == 2 && part.chars().all(|c| c.is_ascii_alphabetic())) || (part.len() == 3 && part.chars().all(|c| c.is_ascii_digit())) {
            // a region: AR, BR, 419
            out.push_str(&part.to_ascii_uppercase());
        } else {
            out.push_str(&part.to_ascii_lowercase());
        }
    }
    out
}

/// The catalogs a language looks in, most specific first: `es-AR` →
/// `es-AR`, `es`; `zh-Hans-CN` → `zh-Hans-CN`, `zh-Hans`, `zh`. Chinese
/// named by its region alone takes its script's file too (`zh-CN` →
/// `zh-CN`, `zh-Hans`, `zh`; `zh-TW` → … `zh-Hant` …).
pub fn fallback_chain(tag: &str) -> Vec<String> {
    let tag = normalize_tag(tag);
    let mut out = Vec::new();
    let mut t = tag.as_str();
    while !t.is_empty() {
        out.push(t.to_string());
        t = t.rfind('-').map_or("", |i| &t[..i]);
    }
    let parts: Vec<&str> = tag.split('-').collect();
    if parts.len() == 2 && parts[0] == "zh" {
        let script = match parts[1] {
            "CN" | "SG" | "MY" => Some("zh-Hans"),
            "TW" | "HK" | "MO" => Some("zh-Hant"),
            _ => None,
        };
        if let Some(s) = script {
            out.insert(1, s.to_string());
        }
    }
    out
}

/// What reads a catalog file by its name in the program's `lang` folder
/// (`lang/es.po`): its bytes, None when there is none.
pub type Reader = fn(&str) -> Option<Vec<u8>>;

#[derive(Default)]
struct State {
    /// What the program set (normalized); None: the system's.
    chosen: Option<String>,
    /// The system's language, once asked.
    system: Option<String>,
    /// The current language's catalogs, most specific first; None until
    /// the first translation after the language was set.
    chain: Option<Rc<Vec<Catalog>>>,
    /// The language the program was last told about (OnLanguageChange):
    /// None until it listens.
    told: Option<String>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
    /// How the runtime asks the system's language (the web's: the
    /// browser's), and reads the program's files (the web's: the page's).
    static SYSTEM_PROBE: Cell<Option<fn() -> String>> = const { Cell::new(None) };
    static READER: Cell<Option<Reader>> = const { Cell::new(None) };
}

/// A runtime without a process environment or file system (the web's)
/// says how to ask the system's language and how to read the program's
/// files (`lang/es.po`), before the program runs.
pub fn set_platform(system_language: fn() -> String, reader: Reader) {
    SYSTEM_PROBE.with(|p| p.set(Some(system_language)));
    READER.with(|r| r.set(Some(reader)));
}

/// The language the program speaks now: what it set as
/// `Application.Language`, else the system's (normalized: `es-AR`).
pub fn language() -> String {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        if let Some(l) = &s.chosen {
            return l.clone();
        }
        s.system.get_or_insert_with(system_language).clone()
    })
}

/// `Application.Language = tag`: the program speaks that language from
/// now on (its catalogs read again); "" goes back to the system's.
pub fn set_language(tag: &str) {
    let tag = normalize_tag(tag);
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.chosen = (!tag.is_empty()).then_some(tag);
        s.chain = None;
    });
}

/// The program listens for the language's changes (its
/// Application.OnLanguageChange handler was bound): from the language it
/// has now.
pub fn listen() {
    let now = language();
    STATE.with(|s| s.borrow_mut().told = Some(now));
}

/// Whether the language changed since the program was last told (since
/// its OnLanguageChange handler was bound, or this last said so). Never
/// before it listens; a change undone before it's asked is none.
pub fn take_change() -> bool {
    let told = STATE.with(|s| s.borrow().told.clone());
    let Some(told) = told else { return false };
    let now = language();
    if told == now {
        return false;
    }
    STATE.with(|s| s.borrow_mut().told = Some(now));
    true
}

/// The system's language: `LANGUAGE`'s first, `LC_ALL`, `LC_MESSAGES`
/// (the user's explicit choice), then the system's own setting (macOS'
/// preferred languages, Windows' user locale), then `LANG`; English when
/// none says. The web's: the browser's (`set_platform`).
fn system_language() -> String {
    if let Some(probe) = SYSTEM_PROBE.with(Cell::get) {
        let t = normalize_tag(&probe());
        return if t.is_empty() { "en".to_string() } else { t };
    }
    let env = |var: &str| std::env::var(var).ok().map(|v| normalize_tag(v.split(':').next().unwrap_or(""))).filter(|t| !t.is_empty());
    if let Some(t) = env("LANGUAGE").or_else(|| env("LC_ALL")).or_else(|| env("LC_MESSAGES")) {
        return t;
    }
    if let Some(t) = os_language().map(|l| normalize_tag(&l)).filter(|t| !t.is_empty()) {
        return t;
    }
    env("LANG").unwrap_or_else(|| "en".to_string())
}

#[cfg(target_os = "macos")]
fn os_language() -> Option<String> {
    use std::ffi::{c_char, c_void, CStr};
    type CfRef = *const c_void;
    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFLocaleCopyPreferredLanguages() -> CfRef;
        fn CFArrayGetCount(array: CfRef) -> isize;
        fn CFArrayGetValueAtIndex(array: CfRef, index: isize) -> CfRef;
        fn CFStringGetCString(string: CfRef, buffer: *mut c_char, size: isize, encoding: u32) -> u8;
        fn CFRelease(cf: CfRef);
    }
    const UTF8: u32 = 0x0800_0100;
    // SAFETY: CoreFoundation's documented calls: the array is ours (Copy)
    // and released once; its first string is read into a buffer of the
    // size given, NUL-terminated on success.
    unsafe {
        let langs = CFLocaleCopyPreferredLanguages();
        if langs.is_null() {
            return None;
        }
        let mut out = None;
        if CFArrayGetCount(langs) > 0 {
            let first = CFArrayGetValueAtIndex(langs, 0);
            let mut buf = [0 as c_char; 128];
            if !first.is_null() && CFStringGetCString(first, buf.as_mut_ptr(), buf.len() as isize, UTF8) != 0 {
                out = Some(CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned());
            }
        }
        CFRelease(langs);
        out
    }
}

#[cfg(windows)]
fn os_language() -> Option<String> {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetUserDefaultLocaleName(name: *mut u16, size: i32) -> i32;
    }
    let mut buf = [0u16; 85];
    // SAFETY: the buffer and its size in characters (LOCALE_NAME_MAX_LENGTH).
    let n = unsafe { GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as i32) };
    (n > 1).then(|| String::from_utf16_lossy(&buf[..n as usize - 1]))
}

#[cfg(not(any(target_os = "macos", windows)))]
fn os_language() -> Option<String> {
    None
}

/// A catalog file's bytes, by name in the program's `lang` folder: the
/// runtime's reader (the web's), else the folder beside the program file
/// or executable (`rapidr_value::command_line::files_folder`).
fn read(name: &str) -> Option<Vec<u8>> {
    if let Some(reader) = READER.with(Cell::get) {
        return reader(&format!("lang/{name}"));
    }
    let folder = crate::command_line::files_folder()?;
    std::fs::read(folder.join("lang").join(name)).ok()
}

/// The catalog for exactly language `tag`, when the program has its file
/// (`es-AR.po`, or `es_AR.po`, in either case).
fn load(tag: &str) -> Option<Catalog> {
    let lower = tag.to_ascii_lowercase();
    let mut names: Vec<String> = Vec::with_capacity(4);
    for name in [tag.to_string(), tag.replace('-', "_"), lower.replace('-', "_"), lower] {
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names.iter().find_map(|name| read(&format!("{name}.po"))).map(|bytes| parse_po(&crate::objects::codec::decode_text(&bytes)))
}

/// The current language's catalogs (read once per language).
fn chain() -> Rc<Vec<Catalog>> {
    if let Some(c) = STATE.with(|s| s.borrow().chain.clone()) {
        return c;
    }
    let chain: Rc<Vec<Catalog>> = Rc::new(fallback_chain(&language()).iter().filter_map(|t| load(t)).collect());
    STATE.with(|s| s.borrow_mut().chain = Some(chain.clone()));
    chain
}

/// `text`'s translation in `context` in the current language, else `text`.
pub fn translate(context: &str, text: &str) -> String {
    let chain = chain();
    chain.iter().find_map(|c| c.text(context, text)).unwrap_or(text).to_string()
}

/// The translation of `singular` / `plural` for count `n` in `context`:
/// the first catalog that has it, by its language's plural rule; else the
/// source text by English's (`singular` for 1).
pub fn translate_plural(context: &str, singular: &str, plural: &str, n: u64) -> String {
    let chain = chain();
    match chain.iter().find_map(|c| c.plural_text(context, singular, n)) {
        Some(t) => t.to_string(),
        None if n == 1 => singular.to_string(),
        None => plural.to_string(),
    }
}

/// `TR$(text$ [, context$] [, name$, value …])`: after the text, an odd
/// number of arguments starts with the context; the rest are placeholder
/// pairs, name then value.
pub fn rp_tr(args: &[Value]) -> Value {
    let text = args.first().map(Value::to_string_val).unwrap_or_default();
    let rest = args.get(1..).unwrap_or(&[]);
    let (context, pairs) = split_context(rest);
    let translated = translate(&context, &text);
    v_str(&fill(&translated, |name| pair_value(pairs, name)))
}

/// `TRN$(singular$, plural$, count [, context$] [, name$, value …])`: the
/// form for `count` in the current language, `{n}` the count.
pub fn rp_trn(args: &[Value]) -> Value {
    let arg = |i: usize| args.get(i).map(Value::to_string_val).unwrap_or_default();
    let count = args.get(2).cloned().unwrap_or(Value::Integer(0));
    let n = count.to_f64().abs();
    let n = if n.is_finite() { n.trunc().min(u64::MAX as f64) as u64 } else { 0 };
    let (context, pairs) = split_context(args.get(3..).unwrap_or(&[]));
    let translated = translate_plural(&context, &arg(0), &arg(1), n);
    v_str(&fill(&translated, |name| if name.eq_ignore_ascii_case("n") { Some(count.to_string_val()) } else { pair_value(pairs, name) }))
}

fn split_context(rest: &[Value]) -> (String, &[Value]) {
    if rest.len() % 2 == 1 {
        (rest[0].to_string_val(), &rest[1..])
    } else {
        (String::new(), rest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{v_dbl, v_int};

    #[test]
    fn plural_rules() {
        let en = Plural::default();
        assert_eq!((en.index(0), en.index(1), en.index(2)), (1, 0, 1));
        let es = Plural::parse(" nplurals=2; plural=(n != 1);").unwrap();
        assert_eq!((es.index(1), es.index(5)), (0, 1));
        let zh = Plural::parse("nplurals=1; plural=0;").unwrap();
        assert_eq!((zh.index(1), zh.index(7)), (0, 0));
        // Polish: 1, 2-4 (not 12-14), the rest
        let pl = Plural::parse("nplurals=3; plural=(n==1 ? 0 : n%10>=2 && n%10<=4 && (n%100<10 || n%100>=20) ? 1 : 2);").unwrap();
        let forms: Vec<usize> = [1, 2, 4, 5, 12, 14, 22, 25, 101, 102].iter().map(|&n| pl.index(n)).collect();
        assert_eq!(forms, [0, 1, 1, 2, 2, 2, 1, 2, 2, 1]);
        // Arabic: six forms, nested conditions
        let ar = Plural::parse("nplurals=6; plural=n==0 ? 0 : n==1 ? 1 : n==2 ? 2 : n%100>=3 && n%100<=10 ? 3 : n%100>=11 ? 4 : 5;").unwrap();
        let forms: Vec<usize> = [0, 1, 2, 3, 11, 100].iter().map(|&n| ar.index(n)).collect();
        assert_eq!(forms, [0, 1, 2, 3, 4, 5]);
        // French: 0 and 1 singular
        let fr = Plural::parse("nplurals=2; plural=(n > 1);").unwrap();
        assert_eq!((fr.index(0), fr.index(1), fr.index(2)), (0, 0, 1));
        // `!`, arithmetic, division by zero, an index past the last form
        let odd = Plural::parse("nplurals=2; plural=!(n - n) * 7 + n / 0;").unwrap();
        assert_eq!(odd.index(3), 1);
        // not a rule
        for bad in ["nplurals=2;", "plural=n!=1;", "nplurals=0; plural=0;", "nplurals=2; plural=n !=;", "nplurals=2; plural=(n;", "nplurals=2; plural=x;"] {
            assert_eq!(Plural::parse(bad), None, "{bad}");
        }
        let deep = format!("nplurals=2; plural={}n{};", "(".repeat(500), ")".repeat(500));
        assert_eq!(Plural::parse(&deep), None);
    }

    const ES: &str = r#"# Spanish
msgid ""
msgstr ""
"Content-Type: text/plain; charset=UTF-8\n"
"Plural-Forms: nplurals=2; plural=(n != 1);\n"

#: main.bas:3
msgid "Save file"
msgstr "Guardar archivo"

msgctxt "menu"
msgid "Open"
msgstr "Abrir"

msgid "Open"
msgstr "Abierto"

msgid "{n} file"
msgid_plural "{n} files"
msgstr[0] "{n} archivo"
msgstr[1] "{n} archivos"

msgid ""
"Two "
"lines"
msgstr "Dos\n"
"líneas \"citadas\"\t\\"

#, fuzzy
msgid "Draft"
msgstr "Borrador"

msgid "Untranslated"
msgstr ""

#~ msgid "Old"
#~ msgstr "Viejo"
"#;

    #[test]
    fn reads_a_po_file() {
        let c = parse_po(ES);
        assert_eq!(c.plural.nplurals, 2);
        assert_eq!(c.text("", "Save file"), Some("Guardar archivo"));
        assert_eq!(c.text("menu", "Open"), Some("Abrir"));
        assert_eq!(c.text("", "Open"), Some("Abierto"));
        assert_eq!(c.plural_text("", "{n} file", 1), Some("{n} archivo"));
        assert_eq!(c.plural_text("", "{n} file", 5), Some("{n} archivos"));
        assert_eq!(c.text("", "Two lines"), Some("Dos\nlíneas \"citadas\"\t\\"));
        assert_eq!(c.text("", "Draft"), None, "fuzzy entries are not used");
        assert_eq!(c.text("", "Untranslated"), None);
        assert_eq!(c.text("", "Old"), None);
        assert_eq!(c.len(), 5);
    }

    #[test]
    fn escapes() {
        assert_eq!(parse_string(r#""a\nb\tc\"d\\e\x41\101\q""#).as_deref(), Some("a\nb\tc\"d\\eAAq"));
        assert_eq!(parse_string(r#""ok" # note"#).as_deref(), Some("ok"));
        assert_eq!(parse_string(r#""open"#), None);
        assert_eq!(parse_string(r#""x" y"#), None);
        assert_eq!(parse_string(r#""\x""#), None);
    }

    #[test]
    fn broken_files_keep_their_good_entries() {
        let broken = "garbage line\nmsgid \"Good\"\nmsgstr \"Bueno\"\n%%%\n\"stray\"\nmsgid \"Bad\nmsgstr \"never\"\nmsgid \"Fine\"\nmsgstr \"Bien\"\nmsgstr[99] \"x\"\nmsgid \"Half\"\n\u{0}\u{ff}\nmsgid \"Also\"\nmsgstr \"Tambi\u{e9}n\"\n\"";
        let c = parse_po(broken);
        assert_eq!(c.text("", "Good"), Some("Bueno"));
        assert_eq!(c.text("", "Fine"), Some("Bien"));
        assert_eq!(c.text("", "Also"), Some("También"));
        assert_eq!(c.text("", "Half"), None);
        // nothing at all, only noise, a BOM, Windows line ends
        assert!(parse_po("").is_empty());
        assert!(parse_po("\"\"\"\nmsgstr[\nmsgid\n#,").is_empty());
        let c = parse_po("\u{FEFF}msgid \"A\"\r\nmsgstr \"B\"\r\n");
        assert_eq!(c.text("", "A"), Some("B"));
        // a header without plural forms: English's rule
        let c = parse_po("msgid \"\"\nmsgstr \"Language: es\\n\"\n");
        assert_eq!(c.plural, Plural::default());
    }

    #[test]
    fn plural_forms_past_the_rule_take_the_last() {
        let c = parse_po("msgid \"\"\nmsgstr \"Plural-Forms: nplurals=3; plural=n;\\n\"\nmsgid \"a\"\nmsgid_plural \"as\"\nmsgstr[0] \"zero\"\nmsgstr[1] \"one\"\n");
        assert_eq!(c.plural_text("", "a", 0), Some("zero"));
        assert_eq!(c.plural_text("", "a", 1), Some("one"));
        assert_eq!(c.plural_text("", "a", 9), Some("one"));
    }

    #[test]
    fn tags_and_fallback() {
        assert_eq!(normalize_tag("es_AR.UTF-8@euro"), "es-AR");
        assert_eq!(normalize_tag("ES-ar"), "es-AR");
        assert_eq!(normalize_tag("zh_hans_cn"), "zh-Hans-CN");
        assert_eq!(normalize_tag("es-419"), "es-419");
        assert_eq!(normalize_tag("C"), "");
        assert_eq!(normalize_tag("POSIX"), "");
        assert_eq!(normalize_tag("  "), "");
        assert_eq!(fallback_chain("es-AR"), ["es-AR", "es"]);
        assert_eq!(fallback_chain("zh-Hans-CN"), ["zh-Hans-CN", "zh-Hans", "zh"]);
        assert_eq!(fallback_chain("zh_CN"), ["zh-CN", "zh-Hans", "zh"]);
        assert_eq!(fallback_chain("zh-TW"), ["zh-TW", "zh-Hant", "zh"]);
        assert_eq!(fallback_chain("pt"), ["pt"]);
        assert!(fallback_chain("").is_empty());
    }

    #[test]
    fn placeholders() {
        let v = |n: &str| match n.to_ascii_lowercase().as_str() {
            "name" => Some("Ana".to_string()),
            "n" => Some("5".to_string()),
            _ => None,
        };
        assert_eq!(fill("Hola, {name}!", v), "Hola, Ana!");
        assert_eq!(fill("{n} de {n}", v), "5 de 5");
        assert_eq!(fill("{{name}} {Name} {other} {", v), "{name} Ana {other} {");
        assert_eq!(fill("a } b }} c {x{name}", v), "a } b } c {xAna");
        assert_eq!(fill("no braces", v), "no braces");
        assert_eq!(fill("{ name }", v), "Ana");
    }

    thread_local!(static FILES: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new()));

    fn test_reader(name: &str) -> Option<Vec<u8>> {
        FILES.with(|f| f.borrow().get(name).map(|s| s.as_bytes().to_vec()))
    }

    fn with_files(files: &[(&str, &str)]) {
        FILES.with(|f| *f.borrow_mut() = files.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect());
        set_platform(|| "en-US".to_string(), test_reader);
        STATE.with(|s| *s.borrow_mut() = State::default());
    }

    #[test]
    fn translates_in_the_language_set() {
        with_files(&[
            ("lang/es.po", ES),
            ("lang/es_AR.po", "msgid \"Open\"\nmsgstr \"Abrí\"\n"),
            ("lang/zh-Hans.po", "msgid \"\"\nmsgstr \"Plural-Forms: nplurals=1; plural=0;\\n\"\nmsgid \"{n} file\"\nmsgid_plural \"{n} files\"\nmsgstr[0] \"{n} 个文件\"\nmsgid \"Save file\"\nmsgstr \"保存文件\"\n"),
        ]);
        let tr = |args: &[Value]| rp_tr(args).to_string_val();
        let trn = |args: &[Value]| rp_trn(args).to_string_val();
        assert_eq!(language(), "en-US");
        assert_eq!(tr(&[v_str("Save file")]), "Save file");
        set_language("es");
        assert_eq!(language(), "es");
        assert_eq!(tr(&[v_str("Save file")]), "Guardar archivo");
        assert_eq!(tr(&[v_str("Open"), v_str("menu")]), "Abrir");
        assert_eq!(tr(&[v_str("Open")]), "Abierto");
        assert_eq!(trn(&[v_str("{n} file"), v_str("{n} files"), v_int(1)]), "1 archivo");
        assert_eq!(trn(&[v_str("{n} file"), v_str("{n} files"), v_int(5)]), "5 archivos");
        assert_eq!(tr(&[v_str("Hello, {name}!"), v_str("name"), v_str("Ana")]), "Hello, Ana!");
        // es-AR: its own file first, then es
        set_language("es_ar");
        assert_eq!(language(), "es-AR");
        assert_eq!(tr(&[v_str("Open")]), "Abrí");
        assert_eq!(tr(&[v_str("Save file")]), "Guardar archivo");
        // pt: no file, the source text; English's plural rule
        set_language("pt");
        assert_eq!(tr(&[v_str("Save file")]), "Save file");
        assert_eq!(trn(&[v_str("{n} file"), v_str("{n} files"), v_int(1)]), "1 file");
        assert_eq!(trn(&[v_str("{n} file"), v_str("{n} files"), v_dbl(2.5)]), "2.5 files");
        // zh-Hans: one form for every count
        set_language("zh-Hans");
        assert_eq!(trn(&[v_str("{n} file"), v_str("{n} files"), v_int(1)]), "1 个文件");
        assert_eq!(trn(&[v_str("{n} file"), v_str("{n} files"), v_int(9)]), "9 个文件");
        set_language("zh_CN");
        assert_eq!(tr(&[v_str("Save file")]), "保存文件");
        // context and placeholders together; a count as a placeholder too
        set_language("es");
        assert_eq!(tr(&[v_str("Open"), v_str("menu"), v_str("x"), v_int(1)]), "Abrir");
        assert_eq!(trn(&[v_str("{n} file"), v_str("{n} files"), v_int(-1), v_str("who"), v_str("Ana")]), "-1 archivo");
        // back to the system's
        set_language("");
        assert_eq!(language(), "en-US");
    }

    thread_local!(static RAW_FILES: RefCell<HashMap<String, Vec<u8>>> = RefCell::new(HashMap::new()));

    fn raw_reader(name: &str) -> Option<Vec<u8>> {
        RAW_FILES.with(|f| f.borrow().get(name).cloned())
    }

    /// Robert's Solar System 3D (2.121.3): the translation files an earlier
    /// Studio wrote were Windows-1252 (a single-byte `é`, `·`), and TR$
    /// turned each of those into U+FFFD. A `.po` is read as UTF-8 (a BOM
    /// dropped) or, when it isn't, as Windows-1252 — no character is lost on
    /// the way in.
    #[test]
    fn a_po_file_is_read_as_utf8_or_windows_1252() {
        RAW_FILES.with(|f| {
            let mut f = f.borrow_mut();
            f.clear();
            f.insert("lang/es.po".into(), b"msgid \"Playing\"\nmsgstr \"Reproduciendo \xB7 Espa\xF1ol\"\n".to_vec());
            f.insert("lang/zh-Hans.po".into(), "msgid \"Language\"\nmsgstr \"语言 · 中文(简体)\"\n".as_bytes().to_vec());
            f.insert("lang/fr.po".into(), "\u{feff}msgid \"Hello\"\nmsgstr \"Bonjour, ça va?\"\n".as_bytes().to_vec());
        });
        set_platform(|| "en-US".to_string(), raw_reader);
        STATE.with(|s| *s.borrow_mut() = State::default());
        let tr = |text: &str| rp_tr(&[v_str(text)]).to_string_val();
        set_language("es");
        assert_eq!(tr("Playing"), "Reproduciendo · Español");
        set_language("zh-Hans");
        assert_eq!(tr("Language"), "语言 · 中文(简体)");
        set_language("fr");
        assert_eq!(tr("Hello"), "Bonjour, ça va?");
        assert!(!tr("Hello").contains('\u{fffd}'));
        set_language("");
    }

    #[test]
    fn language_changes_are_told_once() {
        with_files(&[]);
        assert!(!take_change(), "never before the program listens");
        set_language("es");
        assert!(!take_change());
        listen();
        assert!(!take_change());
        set_language("pt-br");
        assert!(take_change());
        assert!(!take_change());
        set_language("es");
        set_language("pt-BR");
        assert!(!take_change(), "a change undone is none");
    }

    #[test]
    fn a_program_without_catalogs_gets_its_own_text() {
        with_files(&[]);
        set_language("de");
        assert_eq!(rp_tr(&[v_str("Hi {who}"), v_str("who"), v_int(3)]).to_string_val(), "Hi 3");
        assert_eq!(rp_tr(&[]).to_string_val(), "");
        assert_eq!(rp_trn(&[v_str("one"), v_str("many")]).to_string_val(), "many");
    }
}
