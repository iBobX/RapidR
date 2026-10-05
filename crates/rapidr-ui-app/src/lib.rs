//! The program's side of the UI kernel, for every host
//! (docs/web-host-plan.md §3.1, Stage W2): what a runtime does between the
//! kernel and the program, whichever host draws the windows — the desktop's
//! winit or headless host now, the browser's later.
//!
//! The program is a [`Program`]: its components (the property store), its
//! events, its layout and the clock. runtime-core implements it over its
//! registry; the web runtime will over its own.
//!
//! Modules: the test hooks' environment ([`testhooks`]); the dialogs'
//! requests and answers ([`file_dialog`], [`choose_dialogs`]).

pub mod program;

pub mod testhooks;
// (the dialogs lane's: Open / Save dialogs; colour and font dialogs too)
pub mod choose_dialogs;
pub mod file_dialog;

pub use program::Program;
