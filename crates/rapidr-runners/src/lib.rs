//! Runners for every system, fetched on demand, checked, kept
//! (docs/build-for-every-platform-plan.md §3 step 2 and §5).
//!
//! Building an app for another system needs that system's runner (the
//! program the app's bytecode is appended to), and each RapidR install ships
//! only its own. [`runner_bytes`] finds the one asked for:
//!
//! 1. in this install,
//! 2. in the cache of runners downloaded before, or
//! 3. on the host, where a download is used only if it passes [`verify`]:
//!    the list's minisign signature (the key built into RapidR), this RapidR's
//!    version, the file's size and SHA-256.
//!
//! The split for the browser: **everything that decides whether a runner is
//! good is bytes in, answer out** — [`blake2b`], [`minisign`], [`manifest`],
//! [`verify`], [`bundle`], [`names`], [`urls`] — and compiles to wasm32
//! (`--no-default-features`). The web Studio fetches with the browser's
//! `fetch`, keeps the files in Cache Storage, and calls [`verify::check_all`].
//! The download ([`net`]), the cache ([`store`]) and the install's files
//! ([`resolve`]) are the `native` feature.
//!
//! A person's words for every way it can go wrong are the `Display` of
//! [`verify::Refusal`] and [`resolve::Error`].

pub mod blake2b;
pub mod bundle;
pub mod manifest;
pub mod minisign;
pub mod names;
pub mod urls;
pub mod verify;

#[cfg(feature = "native")]
pub mod net;
#[cfg(feature = "native")]
pub mod resolve;
#[cfg(feature = "native")]
pub mod store;

pub use bundle::Bundle;
pub use manifest::{Entry, Manifest};
pub use names::NOTICES_FILE;
pub use verify::{Refusal, Trust};

#[cfg(feature = "native")]
pub use resolve::{resolve, runner_bytes, Config, Error, Progress, Resolved, Source, Step};

/// RapidR's version: the runners a build uses are always this one's.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
