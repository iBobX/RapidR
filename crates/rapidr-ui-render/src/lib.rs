//! RapidR's UI kernel drawn (`docs/web-host-plan.md` §3.4): a form's
//! [`DisplayList`](rapidr_ui_kernel::DisplayList), in RapidQ's logical
//! pixels, drawn at the device's resolution by the one piece of code both
//! hosts use, so a form has the same pixels on the desktop and in the
//! browser.
//!
//! - [`canvas`]: the display list's ops, texts and editors as the few
//!   primitives a renderer draws ([`Canvas`](canvas::Canvas)), on the
//!   device's pixel grid.
//! - [`cpu`]: vello_cpu into a pixmap — every capture (deterministic, no
//!   GPU or window server), a desktop window where wgpu finds no GPU
//!   (through softbuffer), every frame on the web (`putImageData`). Builds
//!   for `wasm32-unknown-unknown`, with or without wasm SIMD.
//! - [`gpu`] (feature `gpu`): the same list as a vello scene for wgpu — the
//!   desktop's windows, WebGPU in a browser that has it.
//! - [`images`]: the pictures a list names, converted once per revision and
//!   kept between frames.
//!
//! Nothing here knows a window system or the DOM: the hosts
//! (`rapidr-ui-host-winit`, `rapidr-ui-host-web`) make the surfaces and put
//! the frames on them.

pub mod canvas;
pub mod chart;
pub mod cpu;
#[cfg(feature = "gpu")]
pub mod gpu;
pub mod images;
