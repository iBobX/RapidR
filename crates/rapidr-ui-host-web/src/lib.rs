//! RapidR's web host for the UI kernel (docs/web-host-plan.md): the web
//! counterpart of `rapidr-ui-host-winit`, so a form is the same retained
//! tree, the same input routing, the same display list and the same pixels
//! on the web as on the desktop.
//!
//! - [`host`] (Stage W3): the program's forms as windows on the page — a
//!   kernel-drawn frame ([`frame`]), the client area drawn by the shared
//!   CPU renderer (`rapidr-ui-render`) at `devicePixelRatio`, the
//!   accessibility mirror over it ([`mirror`], from [`aria`]'s
//!   descriptions), the pointer, keys, input methods, the clipboard and
//!   autofill routed into the kernel through `rapidr_ui_app::desktop`'s
//!   `Desktop` (the desktop host's). The web runtime drives it (its
//!   `kernel` feature, `?host=kernel`).
//! - The W0 spike (feature `spike`): `SpikeForm`, three fixture forms in
//!   `MemStore`s and the spike's page (`tests/web_host_spike.html`).
//!
//! Everything compiles for the desktop too (wasm-bindgen's exports are
//! inert there), so `cargo check` of the workspace covers it.

// (the spike's page passes a DOM event's fields one by one: no objects
// across the wasm boundary)
#![allow(clippy::too_many_arguments)]

pub mod aria;
pub mod frame;
pub mod host;
pub mod mirror;

#[cfg(feature = "spike")]
pub mod forms;
#[cfg(feature = "gpu")]
pub mod gpu_web;
#[cfg(feature = "spike")]
mod spike;
#[cfg(feature = "spike")]
pub use spike::*;
