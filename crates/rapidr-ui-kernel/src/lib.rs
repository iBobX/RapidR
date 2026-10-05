//! RapidR's UI kernel (ROADMAP Phase 1B, `docs/desktop-host-plan.md` §1):
//! RapidQ's forms as RapidR draws them itself, with no GUI toolkit under it —
//! without a window, a GPU or an OS. Everything here is data in, data out:
//!
//! - **The property store is the only source of truth.** The kernel reads
//!   components through [`Store`] (runtime-core implements it over its
//!   component registry; [`MemStore`] serves tests and kernel-drawn
//!   dialogs) and the shared models in `rapidr_value::objects` (TrackBar,
//!   TabControl, TextEdit …), which the web runtime draws too.
//! - **A retained tree per form** ([`FormUi`]): built from
//!   `Store::children`, absolute rectangles from Left / Top (below an
//!   in-window main menu), plus UI-only state — focus, hover, the pressed
//!   button, the mouse capture, parley editors.
//! - **Input in, events out.** The host passes the mouse, keys and input
//!   methods in ([`FormUi::mouse_down`], [`FormUi::key_down`],
//!   [`FormUi::ime_preedit`] …); the kernel routes them (hit test, capture,
//!   hover, focus and Tab order, default and cancel buttons) into the
//!   models and queues [`KernelEvent`]s. **No program code is ever called
//!   here**: runtime-core dispatches the events after the host's pump
//!   returns (OnClick, OnChange, OnKeyDown …).
//! - **Drawing is a [`DisplayList`]** of positioned
//!   `rapidr_value::objects::ops::Op`s (the shared models' own op
//!   vocabulary) and parley editor layouts, in logical pixels; the host
//!   renders it with vello (GPU or CPU).
//! - **Accessibility is a tree of `rapidr_value::objects::a11y::AccessNode`**
//!   with stable ids ([`FormUi::access_tree`]); screen readers' requests
//!   come back as the same input a user's would be
//!   ([`FormUi::access_action`]).
//!
//! - **Every colour, metric and glyph style is the theme's**
//!   ([`theme`], `rapidr_value::theme`): Windows' classic look (RapidQ's,
//!   the default, drawn op for op as before themes), a modern flat one, a
//!   dark one and Windows' high contrast — the same components in the same
//!   places, drawn differently (`paint.rs`).
//!
//! Components are a table ([`components::kind_of`]): label, button,
//! single-line edit, track bar and tab control so far.

pub mod a11y;
pub mod components;
pub mod dialogs;
pub mod display;
mod focus;
pub mod input;
pub mod paint;
pub mod store;
pub mod text;
pub mod tick;
pub mod tree;

#[cfg(test)]
mod tests;

pub use a11y::AccessValue;
pub use components::{kind_of, ComponentKind, Cx};
pub use display::{DisplayList, Item, Picture, TextItem};
pub use input::{Clipboard, KernelEvent, MemClipboard, Mods};
pub use rapidr_value::objects::ops::{Op, Place, Rect};
/// The themes the kernel draws with (the shared models' too).
pub use rapidr_value::theme;
pub use store::{MemStore, Store};
pub use text::{Ink, TextSystem};
pub use tree::{FormUi, Node, NodeUi};
