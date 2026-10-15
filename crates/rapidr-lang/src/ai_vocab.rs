// The AI facets' vocabularies (docs/ai-architecture.md §2.2): one copy,
// read by build.rs (`include!`, to validate data/*.toml) and by the library
// (src/ai.rs: `Effect`, `Risk`). Plain consts only — no crate paths here.

/// The effects an entry may declare (`effects = ["file-write"]`), in
/// [`Effect`]'s order.
pub const EFFECT_NAMES: &[&str] =
    &["none", "ui", "wait", "state", "clipboard", "file-read", "network", "device", "registry", "db-read", "file-write", "db-write", "printer", "exit", "process", "native"];

/// Each effect's risk: its index in [`RISK_NAMES`].
pub const EFFECT_RISK: &[u8] = &[0, 1, 1, 2, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5];

/// The risk levels, lowest first.
pub const RISK_NAMES: &[&str] = &["read", "ui", "state", "external", "irreversible", "system"];

/// The units a numeric property may give (`unit = "px"`).
pub const UNITS: &[&str] = &["px", "pt", "ms", "s", "deg", "percent", "bgr", "chars", "bytes", "index", "count", "hz", "bps"];
