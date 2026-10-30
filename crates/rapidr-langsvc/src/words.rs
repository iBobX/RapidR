//! RapidR's own words around the registry's docs, in the person's language:
//! the Help pages' labels ("Property of RButton", "Insert into the code"),
//! the hover's and completion's kind words ("local variable", "method of …")
//! — what the language service writes itself, as opposed to the docs the
//! registry carries (`rapidr_lang::docs_i18n`) and the compiler's messages
//! (`rapidr_diagnostics::messages`, translated where a person reads them,
//! never in what the model reads).
//!
//! The service is wasm-safe and light (no `rapidr-value`), so it holds no
//! catalogs of its own: whoever shows its text installs how to translate
//! once ([`install`], `rapidr_i18n::host`): RapidR Studio and `rapidr lsp`
//! give it RapidR's own catalog (`rapidr_value::i18n::tr_ui`,
//! `crates/rapidr-value/lang/es.po`, the one file for every word of RapidR's
//! that Rust writes). Without a translator every word is the English one it
//! is written with, which is what the tests and the AI tools see.

pub use rapidr_i18n::host::{install, tr_ui, tr_ui_fill, SetLanguage, Translate};

/// The language the words and the registry's docs speak (a tag: `es`,
/// `es-AR`, `en`): the docs' ([`rapidr_lang::docs_i18n::set_doc_language`])
/// and, when the host installed how, the catalogs'. A server calls it when
/// it learns its client's language.
pub fn set_language(tag: &str) {
    rapidr_lang::docs_i18n::set_doc_language(tag);
    rapidr_i18n::host::set_language(tag);
}
