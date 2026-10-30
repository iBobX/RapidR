//! What a translation must keep of its source text, and the check that it
//! does (`rapidr i18n check`): the program fills `{name}` and `%1`, shows
//! `` `code` `` as code (names are never translated: RButton, Caption,
//! OnClick), finds the mnemonic after the one `&`, and lines a menu's
//! shortcut up after a tab. A translation that loses one of them breaks
//! the program, not just its wording.

/// The markers of a text, in the order found.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Markers {
    /// `{name}` placeholders (`{{` / `}}` are literal braces, not one).
    pub placeholders: Vec<String>,
    /// `%1` … `%9`, `%s`, `%d`.
    pub percents: Vec<String>,
    /// `` `code` `` spans, with their text.
    pub code: Vec<String>,
    /// Single `&`s (a mnemonic each; `&&` is a literal `&`).
    pub mnemonics: usize,
    /// Tab characters.
    pub tabs: usize,
}

/// The markers in `text`.
pub fn markers(text: &str) -> Markers {
    let mut m = Markers::default();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '{' if chars.get(i + 1) == Some(&'{') => i += 2,
            '}' if chars.get(i + 1) == Some(&'}') => i += 2,
            '{' => {
                match chars[i + 1..].iter().position(|&d| d == '}' || d == '{') {
                    Some(j) if chars[i + 1 + j] == '}' => {
                        m.placeholders.push(chars[i + 1..i + 1 + j].iter().collect::<String>().trim().to_ascii_lowercase());
                        i += j + 2;
                    }
                    _ => i += 1,
                }
            }
            '`' => match chars[i + 1..].iter().position(|&d| d == '`') {
                Some(j) => {
                    m.code.push(chars[i + 1..i + 1 + j].iter().collect());
                    i += j + 2;
                }
                None => i += 1,
            },
            '%' if chars.get(i + 1).is_some_and(|d| d.is_ascii_digit() || *d == 's' || *d == 'd') => {
                m.percents.push(chars[i..i + 2].iter().collect());
                i += 2;
            }
            '&' if chars.get(i + 1) == Some(&'&') => i += 2,
            '&' => {
                if chars.get(i + 1).is_some_and(|d| !d.is_whitespace()) {
                    m.mnemonics += 1;
                }
                i += 1;
            }
            '\t' => {
                m.tabs += 1;
                i += 1;
            }
            _ => i += 1,
        }
    }
    m
}

/// The mnemonic letter of a caption (`&Open` → `o`), lower case; None
/// without one.
pub fn mnemonic(text: &str) -> Option<char> {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i + 1 < chars.len() {
        if chars[i] == '&' {
            if chars[i + 1] == '&' {
                i += 2;
                continue;
            }
            return chars[i + 1].to_lowercase().next();
        }
        i += 1;
    }
    None
}

fn sorted(mut v: Vec<String>) -> Vec<String> {
    v.sort();
    v
}

/// What `translation` lost or added of `source`'s markers, in plain words
/// (empty: it keeps them all).
pub fn check(source: &str, translation: &str) -> Vec<String> {
    let (s, t) = (markers(source), markers(translation));
    let mut out = Vec::new();
    if sorted(s.placeholders.clone()) != sorted(t.placeholders.clone()) {
        out.push(format!("placeholders differ: {} in the source, {} in the translation", list(&s.placeholders, "{", "}"), list(&t.placeholders, "{", "}")));
    }
    if sorted(s.percents.clone()) != sorted(t.percents.clone()) {
        out.push(format!("% markers differ: {} in the source, {} in the translation", list(&s.percents, "", ""), list(&t.percents, "", "")));
    }
    if sorted(s.code.clone()) != sorted(t.code.clone()) {
        out.push(format!("code must stay as written: {} in the source, {} in the translation", list(&s.code, "`", "`"), list(&t.code, "`", "`")));
    }
    if s.mnemonics.min(1) != t.mnemonics.min(1) || t.mnemonics > 1 {
        out.push(format!("the & mnemonic: {} in the source, {} in the translation (one, or none, as the source)", s.mnemonics, t.mnemonics));
    }
    if s.tabs != t.tabs {
        out.push(format!("tabs: {} in the source, {} in the translation", s.tabs, t.tabs));
    }
    let edge = |x: &str| (x.starts_with(char::is_whitespace), x.ends_with(char::is_whitespace), x.ends_with('\n'));
    if edge(source) != edge(translation) {
        out.push("the spaces or line break at the start or end differ from the source's".to_string());
    }
    // (a menu's or button's "..." — three dots, which RapidR Studio strips
    // to make tool-bar tips and palette entries — stays three dots)
    let head = |x: &str| x.split('\t').next().unwrap_or("").trim_end().to_string();
    if head(source).ends_with("...") && !head(translation).ends_with("...") {
        out.push("ends with \"...\" in the source: keep the three dots (not \"…\")".to_string());
    }
    if let Some(k) = mnemonic(translation) {
        // (not when the source has the very same "mnemonic": the `&=` of a
        // hint that names that operator isn't a mnemonic at all)
        if !k.is_ascii_alphanumeric() && mnemonic(source) != Some(k) {
            out.push(format!("the & mnemonic is on '{k}': choose a letter without an accent (it can't be typed with Alt on every keyboard)"));
        }
    }
    out
}

/// [`check`], and the rules of language `tag` (RapidR's glossaries,
/// docs/i18n/glossary-<lang>.md): Spanish opens its questions and
/// exclamations (`¿…?`, `¡…!`).
pub fn check_for(tag: &str, source: &str, translation: &str) -> Vec<String> {
    let mut out = check(source, translation);
    let lang = crate::tags::normalize_tag(tag);
    if lang == "es" || lang.starts_with("es-") {
        let t = translation.trim_end();
        if t.ends_with('?') && !t.contains('¿') {
            out.push("a question opens with ¿ in Spanish".to_string());
        }
        if t.ends_with('!') && !t.contains('¡') {
            out.push("an exclamation opens with ¡ in Spanish".to_string());
        }
    }
    out
}

fn list(items: &[String], open: &str, close: &str) -> String {
    if items.is_empty() {
        "none".to_string()
    } else {
        items.iter().map(|i| format!("{open}{i}{close}")).collect::<Vec<_>>().join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_markers() {
        let m = markers("&Open {file} at %1 with `TR$`\tCtrl+O && more {{x}}");
        assert_eq!(m.placeholders, ["file"]);
        assert_eq!(m.percents, ["%1"]);
        assert_eq!(m.code, ["TR$"]);
        assert_eq!((m.mnemonics, m.tabs), (1, 1));
        assert_eq!(mnemonic("Save &As..."), Some('a'));
        assert_eq!(mnemonic("Fish && Chips"), None);
    }

    #[test]
    fn checks_translations() {
        assert!(check("&Open {file}", "&Abrir {file}").is_empty());
        assert!(check("Hello, {name}!", "¡Hola, {Name}!").is_empty());
        assert_eq!(check("Hello, {name}!", "¡Hola, {nombre}!").len(), 1);
        assert_eq!(check("&Open", "Abrir").len(), 1);
        assert_eq!(check("&Open", "&Ab&rir").len(), 1);
        assert_eq!(check("Set `Caption`", "Pon `Título`").len(), 1);
        assert_eq!(check("Open\tCtrl+O", "Abrir").len(), 1);
        assert_eq!(check("Name: ", "Nombre:").len(), 1);
        assert!(check("Line %1", "Línea %1").is_empty());
        // (the glossary's rules)
        assert!(check("Save &As...", "Guardar co&mo...").is_empty());
        assert_eq!(check("Save &As...", "Guardar co&mo…").len(), 1);
        assert_eq!(check("&Edit", "&Édición").len(), 1);
        // (an operator's `&=` in a sentence: no mnemonic to move)
        assert!(check("the shortcuts are += &=", "los atajos son += &=").is_empty());
        assert!(check_for("es", "Save changes?", "¿Guardar los cambios?").is_empty());
        assert_eq!(check_for("es", "Save changes?", "Guardar los cambios?").len(), 1);
        assert_eq!(check_for("es", "Hello, {name}!", "Hola, {name}!").len(), 1);
        assert!(check_for("pt", "Hello, {name}!", "Olá, {name}!").is_empty());
    }
}
