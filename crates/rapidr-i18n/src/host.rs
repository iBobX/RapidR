//! RapidR's own words in the crates that hold no catalogs: the language
//! service (`rapidr-langsvc`), the AI layer (`rapidr-ai`) and the language
//! server (`rapidr-lsp`) are wasm-safe and light — they do not depend on
//! `rapidr-value`, where RapidR's own catalogs are compiled in
//! (`rapidr_value::i18n::tr_ui`, `crates/rapidr-value/lang/<language>.po`).
//! Whoever shows their text says how to translate, once ([`install`]):
//! RapidR Studio and the `rapidr` command give them
//! `rapidr_value::i18n::tr_ui`. Without a translator every word is the
//! English one it is written with, which is what the tests, a library
//! user and the model that reads a tool's answer see.
//!
//! The words are written `tr_ui("…")` / `tr_ui_fill("…", …)` in those
//! crates, as in `rapidr-value`, so `rapidr i18n runtime` finds them and
//! one catalog holds every word RapidR's Rust code writes.

use std::sync::OnceLock;

/// What translates one of RapidR's own words (English in, the language's out).
pub type Translate = fn(&str) -> String;

/// What is told the language changed (its tag).
pub type SetLanguage = fn(&str);

static TRANSLATE: OnceLock<Translate> = OnceLock::new();
static SET_LANGUAGE: OnceLock<SetLanguage> = OnceLock::new();

/// Gives the crates the translator (and, for a server that learns its
/// client's language late, how the catalogs are told it). Once; the first
/// call wins.
pub fn install(translate: Translate, set_language: Option<SetLanguage>) {
    let _ = TRANSLATE.set(translate);
    if let Some(f) = set_language {
        let _ = SET_LANGUAGE.set(f);
    }
}

/// Tells the catalogs the language (a tag: `es`, `es-AR`, `en`), when the
/// host said how ([`install`]).
pub fn set_language(tag: &str) {
    if let Some(f) = SET_LANGUAGE.get() {
        f(tag);
    }
}

/// `text` (English) in the person's language; the English itself when no
/// translator was installed or the catalog has no translation.
pub fn tr_ui(text: &str) -> String {
    match TRANSLATE.get() {
        Some(f) => f(text),
        None => text.to_string(),
    }
}

/// [`tr_ui`] with its `{name}` placeholders filled: `tr_ui_fill("Property
/// of {comp}", &[("comp", "RButton")])`.
pub fn tr_ui_fill(text: &str, values: &[(&str, &str)]) -> String {
    crate::fill(&tr_ui(text), |name| values.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, v)| v.to_string()))
}
