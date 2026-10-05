//! The program's side of the UI kernel, for every host
//! (docs/web-host-plan.md §3.1, Stage W2): what a runtime does between the
//! kernel and the program, whichever host draws the windows — the desktop's
//! winit or headless host now, the browser's later. Moved out of
//! runtime-core's `ui/` (`kernel.rs`, `kernel_lists.rs`, `kernel/menus.rs`,
//! `testhooks.rs`, `file_dialog.rs`, `choose_dialogs.rs`) without a change
//! in behaviour.
//!
//! - **The program** is a [`Program`]: its components (the property
//!   store), its events, its layout and the clock. runtime-core implements
//!   it over its registry; the web runtime will over its own.
//! - **The windows** are a [`Windows`]: what the glue asks of the host —
//!   start, carry out the queued window commands now, what only it knows
//!   (the screen, a window's scale, where a component is), a pop-up menu, a
//!   test's input. Window commands and "something changed" wait in queues
//!   the host's next turn takes ([`windows`]).
//! - **Nothing here blocks.** The waits that wait for the user — a native
//!   build's ShowModal loop, INPUT$, a dialog's answer — are the runtime's,
//!   around these functions (runtime-core's `ui/kernel.rs`: `step`, the pump,
//!   the tracking tick); what a VM serves itself is bookkept in [`waits`],
//!   the dialogs (shown, answered, their answers mapped to the builtins'
//!   results) in [`dialogs`]. Program code runs only where the runtime
//!   calls in, never inside a host callback.
//!
//! - **The forms as the hosts keep them** ([`desktop`]: each one's kernel
//!   side, stacking, the modal list, the window commands and the host's
//!   events, the input entry points the user and a test script share) —
//!   moved from the desktop host in Stage W3 so the web host shares it.
//!
//! Modules: [`dispatch`] (a kernel event as the program's), [`forms`]
//! (show / hide / close, OnLoad / OnShow / the first OnPaint, the modal
//! list, a user's resize and move, WindowState, toggle buttons),
//! [`timers`], [`waits`], [`lists`] (the lists lane's events and
//! owner-draw), [`menus`], the test hooks' environment ([`testhooks`]) and
//! the [`script`] that plays it, the dialogs the program waits for
//! ([`dialogs`]) and their requests and answers ([`file_dialog`],
//! [`choose_dialogs`]).

pub mod program;
pub mod windows;
// (Stage W3) The forms as the hosts keep them: moved from the desktop host
// so the web host shares it.
pub mod desktop;

pub mod dispatch;
pub mod forms;
pub mod lists;
pub mod menus;
pub mod timers;
pub mod waits;

pub mod script;
pub mod testhooks;
// (the dialogs lane's: Open / Save dialogs; colour and font dialogs too)
pub mod choose_dialogs;
pub mod dialogs;
pub mod file_dialog;

#[cfg(test)]
mod tests;

pub use program::Program;
pub use windows::{Icon, ScriptInput, WindowOp, Windows};
