//! The registry's docs in the user's language: translations kept beside
//! the registry (`data/i18n/<language>/*.toml`, a `[docs]` table of `KEY =
//! "text"`), compiled into [`crate::DOC_TRANSLATIONS`]. A key is an
//! entry's RapidR name in upper case — a component (`RBUTTON`), its member
//! (`RBUTTON.CAPTION`), a built-in (`MID$`), a statement (`PRINT`), a
//! directive (`$INCLUDE`), a keyword, a type, a constant group. Code in the
//! docs (`` `Caption` ``, examples, syntax lines) is never translated.
//!
//! Whoever shows docs says the language once ([`set_doc_language`]: RapidR
//! Studio as its language changes, `rapidr lsp` from the CLI's) and asks
//! [`doc`] for each text: the translation, else the English.

use std::sync::RwLock;

static DOC_LANGUAGE: RwLock<String> = RwLock::new(String::new());

/// The language docs are shown in from now on ("" or `en`: English).
pub fn set_doc_language(tag: &str) {
    if let Ok(mut l) = DOC_LANGUAGE.write() {
        *l = tag.trim().to_string();
    }
}

/// The language docs are shown in ("" for English).
pub fn doc_language() -> String {
    DOC_LANGUAGE.read().map(|l| l.clone()).unwrap_or_default()
}

/// The languages tried for `tag`: `es-UY`, then `es`.
fn chain(tag: &str) -> Vec<String> {
    let t = tag.trim().replace('_', "-");
    let mut out = Vec::new();
    let mut cur = t.as_str();
    while !cur.is_empty() {
        out.push(cur.to_string());
        cur = cur.rfind('-').map_or("", |i| &cur[..i]);
    }
    out
}

/// `key`'s doc translated into `tag` (or a language it falls back to),
/// when it is.
pub fn translated_doc(key: &str, tag: &str) -> Option<&'static str> {
    let key = key.to_ascii_uppercase();
    for lang in chain(tag) {
        if lang.eq_ignore_ascii_case("en") {
            return None;
        }
        if let Some(&(_, _, t))= crate::DOC_TRANSLATIONS.iter().find(|(l, k, _)| l.eq_ignore_ascii_case(&lang) && *k == key) {
            return Some(t);
        }
    }
    None
}

/// `key`'s doc in the doc language ([`set_doc_language`]): the
/// translation, else `english`.
pub fn doc<'a>(key: &str, english: &'a str) -> &'a str {
    let lang = doc_language();
    if lang.is_empty() {
        return english;
    }
    translated_doc(key, &lang).unwrap_or(english)
}

/// Every entry that has a doc: (KEY, the English doc) — what a language's
/// translations are counted against.
pub fn doc_keys() -> Vec<(String, &'static str)> {
    let mut out = Vec::new();
    for c in crate::COMPONENTS.iter().chain(crate::GLOBALS.iter()).chain(crate::ITEMS.iter()) {
        let name = c.name.to_ascii_uppercase();
        if !c.doc.is_empty() {
            out.push((name.clone(), c.doc));
        }
        for p in c.properties.iter().filter(|p| !p.doc.is_empty()) {
            out.push((format!("{name}.{}", p.name.to_ascii_uppercase()), p.doc));
        }
        for m in c.methods.iter().filter(|m| !m.doc.is_empty()) {
            out.push((format!("{name}.{}", m.name.to_ascii_uppercase()), m.doc));
        }
        for e in c.events.iter().filter(|e| !e.doc.is_empty()) {
            out.push((format!("{name}.{}", e.name.to_ascii_uppercase()), e.doc));
        }
    }
    out.extend(crate::BUILTINS.iter().filter(|b| !b.doc.is_empty()).map(|b| (b.name.to_ascii_uppercase(), b.doc)));
    out.extend(crate::STATEMENTS.iter().filter(|s| !s.doc.is_empty()).map(|s| (s.name.to_ascii_uppercase(), s.doc)));
    out.extend(crate::DIRECTIVES.iter().filter(|d| !d.doc.is_empty()).map(|d| (d.name.to_ascii_uppercase(), d.doc)));
    out.extend(crate::KEYWORDS.iter().filter(|k| !k.doc.is_empty()).map(|k| (k.name.to_ascii_uppercase(), k.doc)));
    out.extend(crate::TYPE_NAMES.iter().filter(|t| !t.doc.is_empty()).map(|t| (t.name.to_ascii_uppercase(), t.doc)));
    out.extend(crate::CONSTANT_GROUPS.iter().filter(|g| !g.doc.is_empty()).map(|g| (g.name.to_ascii_uppercase(), g.doc)));
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out.dedup_by(|a, b| a.0 == b.0);
    out
}

/// The languages the docs have translations in.
pub fn doc_languages() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = crate::DOC_TRANSLATIONS.iter().map(|(l, _, _)| *l).collect();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_translation_names_an_entry() {
        let keys: std::collections::HashSet<String> = doc_keys().into_iter().map(|(k, _)| k).collect();
        for (lang, key, text) in crate::DOC_TRANSLATIONS {
            assert!(keys.contains(*key), "data/i18n/{lang}: {key} is no entry with a doc");
            assert!(!text.trim().is_empty(), "data/i18n/{lang}: {key} is empty");
        }
    }

    #[test]
    fn docs_follow_the_language() {
        let (lang, key, text) = crate::DOC_TRANSLATIONS.first().expect("the sample's translations (data/i18n/es)");
        assert_eq!(translated_doc(key, &format!("{lang}-UY")), Some(*text));
        assert_eq!(translated_doc(key, "en"), None);
        assert_eq!(translated_doc("NO SUCH THING", lang), None);
    }
}
