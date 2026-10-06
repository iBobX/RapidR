//! RapidR's data-science components shared by every runtime (decision D7,
//! docs/ide-plan.md): RNUM ([`num`]) and RPLOT ([`plot`]) — one
//! implementation for native, interpreted and web programs. RDATAFRAME is
//! the `rapidr-frame` crate's (polars), which the desktop links and the
//! web loads as a module of its own.

pub mod colors;
pub mod num;
pub mod plot;
