//! RapidR in every language: the pieces every translated part of RapidR
//! shares — programs' `TR$` (rapidr-value's `i18n`), RapidR Studio's own
//! text (`ide/lang/*.po`), the compiler's messages
//! (`rapidr_diagnostics::messages`) and the tools translators use
//! (`rapidr i18n`). No dependencies, no gettext code (GPL / LGPL): RapidR
//! reads and writes PO files with its own small parser.
//!
//! - [`po`]: PO / POT files — read ([`po::parse_po`] into a [`po::Catalog`],
//!   [`po::read_messages`] as written), written ([`po::write_po`]), plural
//!   rules ([`po::Plural`]).
//! - [`tags`]: language tags (`es-AR` → `es` → the source), the system's
//!   language, languages' own names.
//! - [`fill`]: `{name}` placeholders.
//! - [`pseudo`]: the pseudo-language `xx` — every text accented and a
//!   third longer — that shows text no catalog translates, and text that
//!   doesn't fit.
//! - [`markers`]: what a translation must keep of its source (`{name}`,
//!   `%1`, `` `code` ``, one `&` mnemonic, tabs).
//! - [`extract`]: the texts a RapidR program translates (`TR$`, `TRN$`, and
//!   the routines its sources name as taking text), for a template (POT).
//! - [`template`]: a text against its `{name}` template, the values taken out.
//! - [`host`]: RapidR's own words for the crates that hold no catalogs (the
//!   language service, the AI layer): the host installs how to translate.
//! - [`tools`]: templates, updates, coverage and checks, as `rapidr i18n`
//!   runs them.

pub mod extract;
mod fill;
pub mod host;
pub mod markers;
pub mod po;
pub mod pseudo;
pub mod tags;
pub mod template;
pub mod tools;

pub use fill::fill;
pub use po::{parse_po, Catalog, Message, Plural};
pub use tags::{fallback_chain, normalize_tag};

#[cfg(test)]
mod tests;
