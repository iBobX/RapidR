//! PO catalogs: RapidR's own small reader and writer of gettext's plain
//! text format (no gettext code), and the plural rules their
//! `Plural-Forms` header gives.

use std::collections::HashMap;

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
    /// Each translation's source (its first form → context, source text),
    /// made when first asked ([`Catalog::source_of`]).
    reverse: std::cell::OnceCell<HashMap<String, (String, String)>>,
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

    /// Whether it has source text `id` in any context.
    pub fn has_source(&self, id: &str) -> bool {
        self.entries.values().any(|m| m.contains_key(id))
    }

    /// A context source text `id` has (none, "", first; else the first
    /// in order), when it has `id`.
    pub fn context_of(&self, id: &str) -> Option<&str> {
        if self.entries.get("").is_some_and(|m| m.contains_key(id)) {
            return Some("");
        }
        let mut found: Vec<&str> = self.entries.iter().filter(|(_, m)| m.contains_key(id)).map(|(c, _)| c.as_str()).collect();
        found.sort_unstable();
        found.first().copied()
    }

    /// The (context, source text) whose translation is `translated` (its
    /// first form), when one is: how a text shown in this language is read
    /// back to its source when the language changes (the first source in
    /// order wins when two translate alike).
    pub fn source_of(&self, translated: &str) -> Option<(&str, &str)> {
        let reverse = self.reverse.get_or_init(|| {
            let mut m: HashMap<String, (String, String)> = HashMap::new();
            let mut all: Vec<(&String, &String, &Vec<String>)> = self.entries.iter().flat_map(|(c, ids)| ids.iter().map(move |(id, f)| (c, id, f))).collect();
            // (the same answer every run: sources in order)
            all.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));
            for (c, id, forms) in all {
                if let Some(first) = forms.first().filter(|s| !s.is_empty()) {
                    m.entry(first.clone()).or_insert_with(|| (c.clone(), id.clone()));
                }
            }
            m
        });
        reverse.get(translated).map(|(c, id)| (c.as_str(), id.as_str()))
    }

    /// Every entry: (context, source text, forms).
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str, &[String])> {
        self.entries.iter().flat_map(|(c, ids)| ids.iter().map(move |(id, f)| (c.as_str(), id.as_str(), f.as_slice())))
    }
}

/// An entry of a PO (or POT) file as written: what the tools read and
/// write (a template's entries have no translations). The runtime reads
/// files into a [`Catalog`] ([`parse_po`]).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Message {
    /// `msgctxt`, "" for none.
    pub context: String,
    /// `msgid`: the source text ("" with no context: the header).
    pub id: String,
    /// `msgid_plural`.
    pub plural: Option<String>,
    /// `msgstr` / `msgstr[n]`: the translation's forms (none in a template).
    pub forms: Vec<String>,
    /// `#.` comments: notes for translators (from the code).
    pub notes: Vec<String>,
    /// `#:` references: where the text is used (`file:line`).
    pub refs: Vec<String>,
    /// `#,` flags (`fuzzy`, `review`).
    pub flags: Vec<String>,
    /// `#` comments: the translator's own.
    pub comments: Vec<String>,
}

impl Message {
    pub fn has_flag(&self, flag: &str) -> bool {
        self.flags.iter().any(|f| f == flag)
    }

    /// A translation is there: some form not empty.
    pub fn translated(&self) -> bool {
        self.forms.iter().any(|f| !f.is_empty())
    }

    /// The header's entry (`msgid ""`, no context).
    pub fn is_header(&self) -> bool {
        self.id.is_empty() && self.context.is_empty()
    }

    fn field(&mut self, f: Field, seen: &mut Seen) -> &mut String {
        match f {
            Field::Context => {
                seen.context = true;
                &mut self.context
            }
            Field::Id => {
                seen.id = true;
                &mut self.id
            }
            Field::PluralId => self.plural.get_or_insert_with(String::new),
            Field::Str(i) => {
                if self.forms.len() <= i {
                    self.forms.resize(i + 1, String::new());
                }
                &mut self.forms[i]
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Field {
    Context,
    Id,
    PluralId,
    Str(usize),
}

/// What an entry being read has had so far.
#[derive(Default, Clone, Copy)]
struct Seen {
    context: bool,
    id: bool,
}

/// A PO file's entries as written, the header (`msgid ""`) first when it
/// has one. Tolerant: a line it can't read is skipped (and a string
/// continuing it is not taken), an entry without a source text is left out;
/// never a panic.
pub fn read_messages(text: &str) -> Vec<Message> {
    let mut out = Vec::new();
    let mut entry = Message::default();
    let mut seen = Seen::default();
    let mut field: Option<Field> = None;
    let text = text.strip_prefix('\u{FEFF}').unwrap_or(text);
    let done = |out: &mut Vec<Message>, e: Message, s: Seen| {
        if s.id {
            out.push(e);
        }
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(comment) = line.strip_prefix('#') {
            // (the comments before an entry are its: the one before ends)
            if seen.id || !entry.forms.is_empty() {
                done(&mut out, std::mem::take(&mut entry), std::mem::take(&mut seen));
            }
            field = None;
            if let Some(flags) = comment.strip_prefix(',') {
                entry.flags.extend(flags.split(',').map(|f| f.trim().to_string()).filter(|f| !f.is_empty()));
            } else if let Some(note) = comment.strip_prefix('.') {
                entry.notes.push(note.trim().to_string());
            } else if let Some(refs) = comment.strip_prefix(':') {
                entry.refs.extend(refs.split_whitespace().map(String::from));
            } else if !comment.starts_with('|') && !comment.starts_with('~') {
                entry.comments.push(comment.strip_prefix(' ').unwrap_or(comment).to_string());
            }
            continue;
        }
        if line.starts_with('"') {
            match (field, parse_string(line)) {
                (Some(f), Some(s)) => entry.field(f, &mut seen).push_str(&s),
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
            Field::Context => seen.id || seen.context || !entry.forms.is_empty(),
            Field::Id => seen.id || !entry.forms.is_empty(),
            _ => false,
        };
        if starts {
            done(&mut out, std::mem::take(&mut entry), std::mem::take(&mut seen));
        }
        let Some(s) = parse_string(rest.trim()) else {
            // (an unreadable source text: what follows it, until the next
            // entry, is nobody's)
            field = None;
            continue;
        };
        *entry.field(f, &mut seen) = s;
        field = Some(f);
    }
    done(&mut out, entry, seen);
    out
}

/// A PO file's text as a catalog. Tolerant ([`read_messages`]); entries
/// marked `#, fuzzy` are not used (as gettext's tools do), nor ones without
/// a translation. The header entry (`msgid ""`) gives the plural rule.
pub fn parse_po(text: &str) -> Catalog {
    catalog_of(read_messages(text))
}

/// The catalog of entries read ([`parse_po`]'s rules).
pub fn catalog_of(messages: impl IntoIterator<Item = Message>) -> Catalog {
    let mut cat = Catalog::default();
    for e in messages {
        if e.is_header() {
            // `Plural-Forms: nplurals=2; plural=(n != 1);`.
            if let Some(p) = header_value(&e, "plural-forms").and_then(|v| Plural::parse(&v)) {
                cat.plural = p;
            }
            continue;
        }
        if e.has_flag("fuzzy") || e.forms.iter().all(String::is_empty) {
            continue;
        }
        cat.entries.entry(e.context).or_default().insert(e.id, e.forms);
    }
    cat
}

/// A header field's value (`Plural-Forms`, `Language`), by name in any case.
pub fn header_value(header: &Message, name: &str) -> Option<String> {
    let text = header.forms.first()?;
    text.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        key.trim().eq_ignore_ascii_case(name).then(|| value.trim().to_string())
    })
}

/// `s` as a PO string (`"…"`, C's escapes); a text with line breaks as
/// gettext writes it: `""` then a string per line.
pub fn quote(s: &str) -> String {
    let esc = |s: &str| {
        let mut o = String::with_capacity(s.len() + 2);
        for c in s.chars() {
            match c {
                '"' => o.push_str("\\\""),
                '\\' => o.push_str("\\\\"),
                '\n' => o.push_str("\\n"),
                '\t' => o.push_str("\\t"),
                '\r' => o.push_str("\\r"),
                c => o.push(c),
            }
        }
        format!("\"{o}\"")
    };
    let trimmed = s.strip_suffix('\n').unwrap_or(s);
    if !trimmed.contains('\n') {
        return esc(s);
    }
    let mut out = String::from("\"\"");
    let mut rest = s;
    while !rest.is_empty() {
        let (line, after) = match rest.find('\n') {
            Some(i) => rest.split_at(i + 1),
            None => (rest, ""),
        };
        out.push('\n');
        out.push_str(&esc(line));
        rest = after;
    }
    out
}

/// Entries as a PO file's text, in the order given (the header first when
/// it is among them).
pub fn write_po(messages: &[Message]) -> String {
    let mut out = String::new();
    for (i, m) in messages.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        for c in &m.comments {
            if c.is_empty() {
                out.push_str("#\n");
            } else {
                out.push_str(&format!("# {c}\n"));
            }
        }
        for n in &m.notes {
            out.push_str(&format!("#. {n}\n"));
        }
        if !m.refs.is_empty() {
            out.push_str(&format!("#: {}\n", m.refs.join(" ")));
        }
        if !m.flags.is_empty() {
            out.push_str(&format!("#, {}\n", m.flags.join(", ")));
        }
        if !m.context.is_empty() {
            out.push_str(&format!("msgctxt {}\n", quote(&m.context)));
        }
        out.push_str(&format!("msgid {}\n", quote(&m.id)));
        match &m.plural {
            Some(p) => {
                out.push_str(&format!("msgid_plural {}\n", quote(p)));
                let n = m.forms.len().max(2);
                for k in 0..n {
                    out.push_str(&format!("msgstr[{k}] {}\n", quote(m.forms.get(k).map(String::as_str).unwrap_or(""))));
                }
            }
            None => out.push_str(&format!("msgstr {}\n", quote(m.forms.first().map(String::as_str).unwrap_or("")))),
        }
    }
    out
}

/// A PO string: `"..."` with C's escapes (`\n \t \r \" \\`, and `\a \b
/// \f \v`, octal `\ooo`, hex `\xHH`); None when it isn't one (no closing
/// quote, or something after it other than a comment).
pub(crate) fn parse_string(s: &str) -> Option<String> {
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
