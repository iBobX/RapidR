//! The texts a RapidR program translates, read from its sources: every
//! `TR$("text" [, "context"] …)` and `TRN$("one", "many", n [, "context"] …)`
//! whose text is written in the call (a string, or strings joined with
//! `+`), and the arguments of the routines the sources name as taking text
//! the program translates later — RapidR Studio's command table keeps
//! `AddCmd`'s titles in English and shows `TR$(title)`:
//!
//! ```text
//! ' i18n-keywords: AddCmd:2 AddSubmenu:3 AddTopMenu:2
//! ```
//!
//! (`Name:N`: the Nth argument, from 1, is a text to translate.) A `TR$`
//! of a variable is looked up at run time like any other, so it isn't
//! listed: its texts come from elsewhere (a keyword's argument).

/// A text found: what a template lists.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub context: String,
    pub id: String,
    pub plural: Option<String>,
    /// Where: the file as given, and the line (from 1).
    pub file: String,
    pub line: usize,
}

/// A routine whose argument `arg` (from 1) is a text to translate.
#[derive(Clone, Debug, PartialEq)]
pub struct Keyword {
    /// Lower case (`addcmd`).
    pub name: String,
    pub arg: usize,
}

/// The keywords source `text` declares (`' i18n-keywords: AddCmd:2 …`).
pub fn keywords_in(text: &str) -> Vec<Keyword> {
    let mut out = Vec::new();
    for line in text.lines() {
        let l = line.trim_start();
        let Some(rest) = l.strip_prefix('\'').map(str::trim_start).and_then(|r| r.strip_prefix("i18n-keywords:")) else { continue };
        for spec in rest.split_whitespace() {
            if let Some((name, arg)) = spec.split_once(':') {
                if let Ok(arg) = arg.parse::<usize>() {
                    if arg > 0 {
                        out.push(Keyword { name: name.to_ascii_lowercase(), arg });
                    }
                }
            }
        }
    }
    out
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Str(String),
    Word(String),
    Punct(char),
    /// The end of a statement: a line's end or `:`.
    End,
}

/// Tokens of BASIC source, each with its line: strings (no escapes, as
/// RapidQ's: a string ends at its quote or the line's end), words (with a
/// `$` / `%` / `&` / `!` / `#` suffix), punctuation; comments (`'`, `REM`)
/// dropped; a line ending in ` _` goes on on the next.
fn tokens(text: &str) -> Vec<(Tok, usize)> {
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut line = 1;
    let mut statement_start = true;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '\n' => {
                out.push((Tok::End, line));
                line += 1;
                i += 1;
                statement_start = true;
            }
            '\r' | ' ' | '\t' => i += 1,
            '\'' => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            '"' => {
                let start = i + 1;
                let mut j = start;
                while j < chars.len() && chars[j] != '"' && chars[j] != '\n' {
                    j += 1;
                }
                out.push((Tok::Str(chars[start..j].iter().collect()), line));
                i = if j < chars.len() && chars[j] == '"' { j + 1 } else { j };
                statement_start = false;
            }
            '_' if {
                // ` _` then the line's end (or a comment): a continuation
                let mut k = i + 1;
                while k < chars.len() && (chars[k] == ' ' || chars[k] == '\t' || chars[k] == '\r') {
                    k += 1;
                }
                (k >= chars.len() || chars[k] == '\n' || chars[k] == '\'') && (i == 0 || chars[i - 1] == ' ' || chars[i - 1] == '\t')
            } =>
            {
                // skip to the line's end, and past it
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                i += 1;
                line += 1;
            }
            ':' => {
                out.push((Tok::End, line));
                i += 1;
                statement_start = true;
            }
            c if c.is_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '.') {
                    i += 1;
                }
                if i < chars.len() && "$%&!#".contains(chars[i]) {
                    i += 1;
                }
                let w: String = chars[start..i].iter().collect();
                if statement_start && w.eq_ignore_ascii_case("rem") {
                    while i < chars.len() && chars[i] != '\n' {
                        i += 1;
                    }
                    continue;
                }
                statement_start = false;
                out.push((Tok::Word(w), line));
            }
            c => {
                out.push((Tok::Punct(c), line));
                i += 1;
                statement_start = false;
            }
        }
    }
    out
}

/// An argument's text when it is written whole in the call: a string, or
/// strings joined with `+`.
fn literal(arg: &[(Tok, usize)]) -> Option<String> {
    let mut out = String::new();
    let mut want_str = true;
    for (t, _) in arg {
        match (t, want_str) {
            (Tok::Str(s), true) => {
                out.push_str(s);
                want_str = false;
            }
            (Tok::Punct('+'), false) => want_str = true,
            _ => return None,
        }
    }
    (!want_str).then_some(out)
}

/// The arguments of a call starting at token `i` (after the routine's
/// name): `(a, b)`, or a statement's `a, b` up to its end.
fn arguments(toks: &[(Tok, usize)], i: usize) -> Vec<Vec<(Tok, usize)>> {
    let mut args = vec![Vec::new()];
    let paren = matches!(toks.get(i), Some((Tok::Punct('('), _)));
    let mut depth = 0i32;
    let mut k = if paren { i + 1 } else { i };
    while k < toks.len() {
        let (t, _) = &toks[k];
        match t {
            Tok::End => break,
            Tok::Punct('(') => depth += 1,
            Tok::Punct(')') => {
                if depth == 0 && paren {
                    break;
                }
                depth -= 1;
            }
            Tok::Punct(',') if depth == 0 => {
                args.push(Vec::new());
                k += 1;
                continue;
            }
            _ => {}
        }
        args.last_mut().unwrap().push(toks[k].clone());
        k += 1;
    }
    if args.len() == 1 && args[0].is_empty() {
        args.clear();
    }
    args
}

/// The texts source `text` (named `file` in the references) translates:
/// its `TR$` / `TRN$` calls with written text, and `keywords`' arguments.
pub fn extract(text: &str, file: &str, keywords: &[Keyword]) -> Vec<Found> {
    let toks = tokens(text);
    let mut out = Vec::new();
    for (i, (t, line)) in toks.iter().enumerate() {
        let Tok::Word(w) = t else { continue };
        let lw = w.to_ascii_lowercase();
        // (a declaration names the routine: `SUB AddCmd(Id AS STRING …)`)
        let declared = i > 0 && matches!(&toks[i - 1].0, Tok::Word(p) if ["sub", "function", "declare"].contains(&p.to_ascii_lowercase().as_str()));
        if declared {
            continue;
        }
        let found = |context: String, id: String, plural: Option<String>| Found { context, id, plural, file: file.to_string(), line: *line };
        match lw.as_str() {
            "tr$" => {
                if !matches!(toks.get(i + 1), Some((Tok::Punct('('), _))) {
                    continue;
                }
                let args = arguments(&toks, i + 1);
                let Some(id) = args.first().and_then(|a| literal(a)) else { continue };
                let rest = args.len() - 1;
                let context = if rest % 2 == 1 { args.get(1).and_then(|a| literal(a)).unwrap_or_default() } else { String::new() };
                if !id.is_empty() {
                    out.push(found(context, id, None));
                }
            }
            "trn$" => {
                if !matches!(toks.get(i + 1), Some((Tok::Punct('('), _))) {
                    continue;
                }
                let args = arguments(&toks, i + 1);
                let (Some(one), Some(many)) = (args.first().and_then(|a| literal(a)), args.get(1).and_then(|a| literal(a))) else { continue };
                let rest = args.len().saturating_sub(3);
                let context = if rest % 2 == 1 { args.get(3).and_then(|a| literal(a)).unwrap_or_default() } else { String::new() };
                out.push(found(context, one, Some(many)));
            }
            _ => {
                for k in keywords.iter().filter(|k| k.name == lw) {
                    // (a statement's call: the name first in it)
                    let first = i == 0 || matches!(toks[i - 1].0, Tok::End) || matches!(&toks[i - 1].0, Tok::Word(p) if p.eq_ignore_ascii_case("then") || p.eq_ignore_ascii_case("else") || p.eq_ignore_ascii_case("call"));
                    let called = first || matches!(toks.get(i + 1), Some((Tok::Punct('('), _)));
                    if !called {
                        continue;
                    }
                    let args = arguments(&toks, i + 1);
                    if let Some(id) = args.get(k.arg - 1).and_then(|a| literal(a)) {
                        if !id.is_empty() && id.chars().any(char::is_alphabetic) {
                            out.push(found(String::new(), id, None));
                        }
                    }
                }
            }
        }
    }
    out
}

/// RapidR's own text in Rust sources: every `tr_ui("…")` and
/// `tr_ui_fill("…", …)` with a string literal (Rust's escapes read), for
/// the runtime's template (`rapidr i18n runtime`).
pub fn extract_rust(text: &str, file: &str) -> Vec<Found> {
    // (the code only: not its tests — from the first `#[cfg(test)]` on —
    // nor its comments, which may show a call; line numbers kept)
    let code_end = text.find("#[cfg(test)]").unwrap_or(text.len());
    let text: String = text[..code_end].lines().map(|l| if l.trim_start().starts_with("//") { "" } else { l }).collect::<Vec<_>>().join("\n");
    let text = text.as_str();
    let mut out = Vec::new();
    for marker in ["tr_ui(\"", "tr_ui_fill(\""] {
        let mut from = 0;
        while let Some(i) = text[from..].find(marker) {
            let at = from + i;
            from = at + marker.len();
            // (a name ending in tr_ui, `my_tr_ui(`, isn't it)
            if at > 0 && text[..at].chars().next_back().is_some_and(|c| c.is_alphanumeric() || c == '_') {
                continue;
            }
            let mut s = String::new();
            let mut chars = text[from..].char_indices();
            let mut ok = false;
            while let Some((_, c)) = chars.next() {
                match c {
                    '"' => {
                        ok = true;
                        break;
                    }
                    '\\' => match chars.next().map(|(_, e)| e) {
                        Some('n') => s.push('\n'),
                        Some('t') => s.push('\t'),
                        Some('"') => s.push('"'),
                        Some('\\') => s.push('\\'),
                        Some('\'') => s.push('\''),
                        Some('\n') => {
                            // (a line continued: the next line's spaces skipped)
                            while chars.clone().next().is_some_and(|(_, c)| c.is_whitespace()) {
                                chars.next();
                            }
                        }
                        Some(o) => s.push(o),
                        None => break,
                    },
                    c => s.push(c),
                }
            }
            if ok && !s.is_empty() {
                let line = text[..at].matches('\n').count() + 1;
                out.push(Found { context: String::new(), id: s, plural: None, file: file.to_string(), line });
            }
        }
    }
    out.sort_by_key(|f| f.line);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_tr_calls() {
        let src = "x = TR$(\"Save\")\nCaption = TR$(\"Open\", \"menu\") ' TR$(\"not this\")\n\
                   l.Caption = TR$(\"Hello, {name}!\", \"name\", n$)\n\
                   s = TRN$(\"{n} file\", \"{n} files\", k)\n\
                   t = TR$(\"a long \" + _\n  \"text\")\n\
                   u = TR$(v$)\nREM TR$(\"nor this\")\n";
        let f = extract(src, "a.rr", &[]);
        let ids: Vec<(&str, &str)> = f.iter().map(|f| (f.context.as_str(), f.id.as_str())).collect();
        assert_eq!(ids, [("", "Save"), ("menu", "Open"), ("", "Hello, {name}!"), ("", "{n} file"), ("", "a long text")]);
        assert_eq!(f[3].plural.as_deref(), Some("{n} files"));
        assert_eq!(f[4].line, 5);
    }

    #[test]
    fn finds_rapidr_s_own_text_in_rust() {
        let src = "let a = tr_ui(\"Search components\");\nlet b = i18n::tr_ui_fill(\"Remove {name} from \\\"it\\\"?\", &[(\"name\", n)]);\nlet c = my_tr_ui(\"no\");\n";
        let ids: Vec<String> = extract_rust(src, "x.rs").into_iter().map(|f| f.id).collect();
        assert_eq!(ids, ["Search components", "Remove {name} from \"it\"?"]);
    }

    #[test]
    fn finds_keyword_arguments() {
        let src = "' i18n-keywords: AddCmd:2 AddTopMenu:2\nSUB AddCmd(Id AS STRING, Title AS STRING)\nEND SUB\n\
                   AddCmd \"file.open\", \"&Open...\", \"File\"\nIF x THEN AddCmd \"a\", \"&Close\"\nAddTopMenu(\"File\", \"&File\")\nAddCmd \"b\", t$\n";
        let kw = keywords_in(src);
        assert_eq!(kw.len(), 2);
        let ids: Vec<String> = extract(src, "c.inc", &kw).into_iter().map(|f| f.id).collect();
        assert_eq!(ids, ["&Open...", "&Close", "&File"]);
    }
}
