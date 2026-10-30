//! The pseudo-language `xx`: every text RapidR translates, accented and
//! about a third longer, in brackets — `Save your work` becomes
//! `[Šàvé ÿöûŕ ŵöŕķ·····]`. Shown in it, any plain English left is text
//! no catalog translates (a string that never went through `TR$`), and
//! any text cut off or spilling out is a layout that won't hold a longer
//! language (Spanish runs a quarter to a third longer than English).
//!
//! What a translation must keep is kept ([`crate::markers`]): `{name}`
//! placeholders, `%1`, `` `code` ``, the `&` mnemonic's letter (so Alt+key
//! still works), what follows a tab (a menu's shortcut), and words that
//! are code or paths rather than words (`*.rr`, `$INCLUDE`, `RButton`,
//! `Ctrl+N`, `UTF-8`, `https://…`). The accented letters are all in
//! Latin-1 and Latin Extended-A, which RapidR's fonts draw.

/// The pseudo-language's tag.
pub const TAG: &str = "xx";

/// The padding a pseudo text gets: a dot this many per hundred characters.
const LONGER_PERCENT: usize = 35;

/// The middle dot that pads a pseudo text.
pub const PAD: char = '·';

fn accent(c: char) -> char {
    match c {
        'a' => 'à',
        'c' => 'ç',
        'd' => 'ð',
        'e' => 'é',
        'g' => 'ĝ',
        'h' => 'ĥ',
        'i' => 'í',
        'j' => 'ĵ',
        'k' => 'ķ',
        'l' => 'ĺ',
        'n' => 'ñ',
        'o' => 'ö',
        'p' => 'þ',
        'r' => 'ŕ',
        's' => 'š',
        't' => 'ţ',
        'u' => 'û',
        'w' => 'ŵ',
        'y' => 'ÿ',
        'z' => 'ž',
        'A' => 'À',
        'C' => 'Ç',
        'D' => 'Ð',
        'E' => 'É',
        'G' => 'Ĝ',
        'H' => 'Ĥ',
        'I' => 'Í',
        'J' => 'Ĵ',
        'K' => 'Ķ',
        'L' => 'Ĺ',
        'N' => 'Ñ',
        'O' => 'Ö',
        'P' => 'Þ',
        'R' => 'Ŕ',
        'S' => 'Š',
        'T' => 'Ţ',
        'U' => 'Û',
        'W' => 'Ŵ',
        'Y' => 'Ý',
        'Z' => 'Ž',
        c => c,
    }
}

/// Whether `c` is one of the pseudo-language's accented letters (what a
/// test looks for to tell translated text from plain English).
pub fn is_pseudo_char(c: char) -> bool {
    "àçðéĝĥíĵķĺñöþŕšţûŵÿžÀÇÐÉĜĤÍĴĶĹÑÖÞŔŠŢÛŴÝŽ".contains(c) || c == PAD
}

/// A word that is code, a path, a key or a number rather than a word of
/// the language: kept as it is.
fn kept_word(w: &str) -> bool {
    let letters = w.chars().filter(|c| c.is_ascii_alphabetic()).count();
    if letters == 0 {
        return true;
    }
    if w.contains("://") || w.contains(['*', '/', '\\', '_', '$', '@', '=', '<', '>', '#', '+']) {
        return true;
    }
    // (`.rr`, `RAPIDQ.INC`, `hello.rr`: a dot inside or before letters)
    let inner_dot = w.trim_end_matches(['.', ',', ';', ':', '!', '?', ')', '"', '\'']).contains('.');
    if inner_dot {
        return true;
    }
    if w.chars().any(|c| c.is_ascii_digit()) {
        return true;
    }
    // (RButton, OnClick, MaxTokens: a capital after a small letter)
    let cs: Vec<char> = w.chars().filter(char::is_ascii_alphabetic).collect();
    if cs.windows(2).any(|p| p[0].is_ascii_lowercase() && p[1].is_ascii_uppercase()) {
        return true;
    }
    // (RButton, QForm: two capitals, then small letters; RAPIDQ, PATH: all
    // capitals — names, not words)
    (cs.len() > 2 && cs[0].is_ascii_uppercase() && cs[1].is_ascii_uppercase() && cs[2].is_ascii_lowercase()) || (cs.len() > 1 && cs.iter().all(char::is_ascii_uppercase))
}

/// `text` in the pseudo-language ([module docs](self)). Text without a
/// letter (`-`, `...`, `|`) stays as it is, as does an empty one.
pub fn pseudo(text: &str) -> String {
    if !text.chars().any(|c| c.is_alphabetic()) {
        return text.to_string();
    }
    // (a menu's shortcut after its tab stays: `Open…\tCtrl+O`)
    let (body, tail) = match text.find('\t') {
        Some(i) => text.split_at(i),
        None => (text, ""),
    };
    let mut out = String::with_capacity(body.len() * 2 + 4);
    let chars: Vec<char> = body.chars().collect();
    let mut i = 0;
    let mut letters = 0usize;
    while i < chars.len() {
        let c = chars[i];
        // {name}: kept whole
        if c == '{' {
            if let Some(j) = chars[i..].iter().position(|&d| d == '}') {
                out.extend(&chars[i..=i + j]);
                i += j + 1;
                continue;
            }
        }
        // `code`: kept whole
        if c == '`' {
            if let Some(j) = chars[i + 1..].iter().position(|&d| d == '`') {
                out.extend(&chars[i..=i + 1 + j]);
                i += j + 2;
                continue;
            }
        }
        // %1, %s: kept
        if c == '%' && i + 1 < chars.len() && (chars[i + 1].is_ascii_alphanumeric()) {
            out.push(c);
            out.push(chars[i + 1]);
            i += 2;
            continue;
        }
        // && is a literal &; &x is the mnemonic: x kept plain
        if c == '&' {
            out.push('&');
            if i + 1 < chars.len() {
                out.push(chars[i + 1]);
                if chars[i + 1].is_alphabetic() {
                    letters += 1;
                }
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }
        if c.is_whitespace() {
            out.push(c);
            i += 1;
            continue;
        }
        // a word: up to the next space
        let start = i;
        while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '{' && chars[i] != '`' && chars[i] != '&' {
            i += 1;
        }
        let word: String = chars[start..i].iter().collect();
        letters += word.chars().filter(|c| c.is_alphabetic()).count();
        if kept_word(&word) {
            out.push_str(&word);
        } else {
            out.extend(word.chars().map(accent));
        }
    }
    let pad = (body.chars().count() * LONGER_PERCENT).div_ceil(100).max(usize::from(letters > 0));
    // (the brackets around the text, inside the spaces it starts or ends with)
    let trailing_ws: String = out.chars().rev().take_while(|c| c.is_whitespace()).collect::<Vec<_>>().into_iter().rev().collect();
    let core = out[..out.len() - trailing_ws.len()].to_string();
    let leading_ws: String = core.chars().take_while(|c| c.is_whitespace()).collect();
    let core = &core[leading_ws.len()..];
    format!("{leading_ws}[{core}{}]{trailing_ws}{tail}", PAD.to_string().repeat(pad))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accents_and_pads() {
        assert_eq!(pseudo("Save your work"), "[Šàvé ÿöûŕ ŵöŕķ·····]");
        assert_eq!(pseudo(""), "");
        assert_eq!(pseudo("-"), "-");
        assert_eq!(pseudo("..."), "...");
        assert_eq!(pseudo("  Ready "), "  [Ŕéàðÿ···] ");
    }

    #[test]
    fn keeps_what_translations_keep() {
        assert_eq!(pseudo("&Open Project..."), "[&Oþéñ Þŕöĵéçţ...······]");
        assert_eq!(pseudo("Hello, {name}!"), "[Ĥéĺĺö, {name}!·····]");
        assert_eq!(pseudo("Line %1 of %2"), "[Ĺíñé %1 öf %2·····]");
        assert_eq!(pseudo("Use `TR$` here"), "[Ûšé `TR$` ĥéŕé·····]");
        assert_eq!(pseudo("Open\tCtrl+O"), "[Öþéñ··]\tCtrl+O");
        assert!(pseudo("Files (*.rr)").contains("*.rr"));
        assert!(pseudo("Set RButton's Caption").contains("RButton's"));
        assert!(pseudo("the classic RAPIDQ.INC").contains("RAPIDQ.INC"));
        assert!(pseudo("See https://rapidr.dev now").contains("https://rapidr.dev"));
        assert_eq!(pseudo("Fish && Chips"), "[Fíšĥ && Çĥíþš·····]");
    }

    #[test]
    fn its_letters_are_known() {
        let p = pseudo("The quick brown fox jumps over the lazy dog");
        assert!(p.chars().any(is_pseudo_char));
        assert!(!"Plain English".chars().any(is_pseudo_char));
    }
}
