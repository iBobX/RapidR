//! RapidR Studio's code editor model (docs/ide-plan.md §4, stage I2), GUI-free
//! and built for the browser too. The kernel's code editor view draws it;
//! users' programs reach it through RCODEEDITOR.
//!
//! - [`buffer`]: the text in a rope (ropey 1.6) behind our own [`Buffer`]
//!   trait; byte offsets, UTF-16 columns on demand, line endings kept.
//! - [`selection`], [`transaction`]: carets and selections (merged when they
//!   overlap); every edit a change set with the selections around it.
//! - [`history`]: the undo tree (linear undo / redo, branches kept), typing
//!   grouped by word or a 400 ms pause.
//! - [`document`]: all of it together, and the editing commands (typing,
//!   auto-closing pairs, auto-indent, Backspace, Tab, comments, paste,
//!   multiple carets, moves).
//! - [`search`]: find and replace (literal, word, case, regex, in selection,
//!   incremental, replacements with groups).
//! - [`lang`]: declarative language definitions (TOML) and their state
//!   machine tokenizer; eleven built in.
//! - [`highlight`]: incremental colouring that stops once a line ends as
//!   it did before.
//! - [`structure`]: folding and bracket matching from the tokens.
//! - [`snippet`]: snippet bodies expanded with their tab stops.
//! - [`service`]: what an editor asks a language service (completion,
//!   hover, diagnostics, keyword case …), and the one a runtime installs.

pub mod buffer;
pub mod document;
pub mod highlight;
pub mod history;
pub mod lang;
pub mod search;
pub mod selection;
pub mod snippet;
pub mod service;
pub mod structure;
pub mod transaction;

pub use buffer::{Buffer, LineEnding, Position, RopeBuffer};
pub use document::{CharClass, Direction, Document};
pub use highlight::{Highlighter, StateId, ROOT};
pub use history::{EditKind, History, GROUP_PAUSE_MS};
pub use lang::{LangError, Language, Languages, Token, TokenKind};
pub use search::{ReplaceError, SearchError, SearchQuery, Searcher};
pub use selection::{Selection, Selections};
pub use structure::{FoldKind, FoldRange};
pub use transaction::{Assoc, Change, ChangeSet, EditError, Transaction};
