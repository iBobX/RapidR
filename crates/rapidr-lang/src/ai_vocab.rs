// The AI facets' vocabularies (docs/ai-architecture.md §2.2): one copy,
// read by build.rs (`include!`, to validate data/*.toml) and by the library
// (src/ai.rs: `Effect`, `Risk`). Plain consts and std-only functions — no
// crate paths here.

/// The effects an entry may declare (`effects = ["file-write"]`), in
/// [`Effect`]'s order.
pub const EFFECT_NAMES: &[&str] =
    &["none", "ui", "wait", "state", "clipboard", "file-read", "network", "device", "registry", "db-read", "file-write", "db-write", "printer", "exit", "process", "native"];

/// Each effect's risk: its index in [`RISK_NAMES`].
pub const EFFECT_RISK: &[u8] = &[0, 1, 1, 2, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5];

/// The risk levels, lowest first.
pub const RISK_NAMES: &[&str] = &["read", "ui", "state", "external", "irreversible", "system"];

/// The units a numeric property may give (`unit = "px"`).
pub const UNITS: &[&str] = &["px", "pt", "ms", "s", "deg", "percent", "bgr", "chars", "bytes", "mb", "index", "count", "hz", "bps"];

/// The kinds a value may have (`kind = "color"`, docs/code-helpers-plan.md
/// §2): what it *is* beyond its type, so the code editor's helpers show and
/// edit it and an AI writes a valid one. Two take an argument:
/// `component:<TYPE>[|<TYPE>…]` (a link to another component;
/// `component:any`) and `choice:<set>` (one of a set of constants: `values`
/// — the property's own —, `bool` — True / False —, or the `set` of a
/// constant group in constants.toml).
pub const KINDS: &[&str] = &["color", "font", "file:image", "file:sound", "file:any", "shortcut", "cursor", "range", "handler"];

/// A builtin's parameters as its `syntax` names them, without type suffixes
/// or brackets: `MID$(Text$, Position, Num)` → Text, Position, Num;
/// `PLAYWAV FileName$|Resource_Handle, SndOption` → FileName, SndOption.
pub fn syntax_param_names(syntax: &str) -> Vec<String> {
    let name_end = syntax.find(['(', ' ', '[']).unwrap_or(syntax.len());
    let inner = match syntax.find('(') {
        Some(open) if syntax[name_end..open].chars().all(|c| c == '[') => {
            let mut depth = 0;
            let mut close = syntax.len();
            for (i, c) in syntax.char_indices().skip_while(|(i, _)| *i < open) {
                match c {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            close = i;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            &syntax[open + 1..close]
        }
        _ => &syntax[name_end..],
    };
    let mut out = Vec::new();
    let mut depth = 0;
    let mut piece = String::new();
    for c in inner.chars().chain(std::iter::once(',')) {
        match c {
            '(' => {
                depth += 1;
                piece.push(c);
            }
            ')' => {
                depth -= 1;
                piece.push(c);
            }
            ',' if depth == 0 => {
                let body = piece.trim().trim_matches(['[', ']', ' ']);
                let name: String = body.chars().skip_while(|c| *c == '#').take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
                if name.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
                    out.push(name);
                }
                piece.clear();
            }
            _ => piece.push(c),
        }
    }
    out
}
