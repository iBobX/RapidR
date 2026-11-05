use std::fmt;

/// The compiler's messages in the user's language: stable IDs, English
/// templates, translations compiled in (`lang/<language>.po`).
pub mod messages;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TextSpan {
    pub start: usize,
    pub end: usize,
}

impl TextSpan {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub const fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }

    pub const fn is_empty(self) -> bool {
        self.start >= self.end
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
}

impl SourceLocation {
    pub const fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    Error,
    Warning,
    Note,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Error => f.write_str("error"),
            Severity::Warning => f.write_str("warning"),
            Severity::Note => f.write_str("note"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub span: TextSpan,
    pub location: SourceLocation,
    pub file_path: Option<String>,
}

impl Diagnostic {
    pub fn new(
        severity: Severity,
        message: impl Into<String>,
        span: TextSpan,
        location: SourceLocation,
        file_path: Option<String>,
    ) -> Self {
        Self {
            severity,
            message: message.into(),
            span,
            location,
            file_path,
        }
    }

    pub fn error(
        message: impl Into<String>,
        span: TextSpan,
        location: SourceLocation,
        file_path: Option<String>,
    ) -> Self {
        Self::new(Severity::Error, message, span, location, file_path)
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(file_path) = &self.file_path {
            write!(f, "{file_path}:")?;
        }
        write!(
            f,
            "{}:{}: {}: {}",
            self.location.line, self.location.column, self.severity, self.message
        )
    }
}

/// A mark in a compiler message for another place of the program — byte
/// `offset` of the preprocessed source (an earlier declaration) — which
/// whoever shows the message writes as `file:line` with the preprocessor's
/// line map (`rapidr_preprocessor::PreprocessResult::resolve_places`, the
/// language service): `Identifier a already used … — already declared in
/// globals.rr:12`. The compiler knows bytes, not files.
pub fn place_mark(offset: usize) -> String {
    format!("{PLACE_OPEN}{offset}{PLACE_CLOSE}")
}

/// A mark like [`place_mark`] for a place the message is about itself —
/// the line of a block left open, written in the message so that it says
/// where in words too (`SUB StartApp (ui_logic.rr:487) has no END SUB`): it
/// is always written as `file:line` (never as an earlier declaration of the
/// same line, an `$INCLUDE`d-twice file: [`resolve_place_marks`]).
pub fn own_place_mark(offset: usize) -> String {
    format!("{OWN_OPEN}{offset}{PLACE_CLOSE}")
}

const PLACE_OPEN: &str = "{{at:";
const OWN_OPEN: &str = "{{place:";
const PLACE_CLOSE: &str = "}}";

/// Whether `text` has a mark of either kind ([`place_mark`], [`own_place_mark`]).
pub fn has_place_marks(text: &str) -> bool {
    text.contains(PLACE_OPEN) || text.contains(OWN_OPEN)
}

/// `text` with every [`place_mark`] replaced by what `place` says of its
/// offset (`None`: "an earlier line"). [`own_place_mark`]s are left.
pub fn resolve_place_marks(text: &str, place: impl FnMut(usize) -> Option<String>) -> String {
    resolve_marks(text, PLACE_OPEN, "an earlier line", place)
}

/// `text` with every [`own_place_mark`] replaced by what `place` says of
/// its offset (`None`: "this line").
pub fn resolve_own_place_marks(text: &str, place: impl FnMut(usize) -> Option<String>) -> String {
    resolve_marks(text, OWN_OPEN, "this line", place)
}

fn resolve_marks(text: &str, open: &str, unknown: &str, mut place: impl FnMut(usize) -> Option<String>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find(open) {
        let after = &rest[i + open.len()..];
        let Some(j) = after.find(PLACE_CLOSE) else { break };
        let Ok(offset) = after[..j].parse::<usize>() else {
            out.push_str(&rest[..i + open.len()]);
            rest = after;
            continue;
        };
        out.push_str(&rest[..i]);
        out.push_str(&place(offset).unwrap_or_else(|| unknown.to_string()));
        rest = &after[j + PLACE_CLOSE.len()..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod place_tests {
    use super::*;

    #[test]
    fn marks_are_replaced() {
        let m = format!("Identifier a already used — already declared in {}", place_mark(42));
        assert_eq!(resolve_place_marks(&m, |o| Some(format!("g.rr:{o}"))), "Identifier a already used — already declared in g.rr:42");
        assert_eq!(resolve_place_marks(&m, |_| None), "Identifier a already used — already declared in an earlier line");
        assert_eq!(resolve_place_marks("no marks {{at:x}} here", |_| None), "no marks {{at:x}} here");
        let own = format!("SUB Go ({}) has no END SUB", own_place_mark(7));
        assert!(has_place_marks(&own) && has_place_marks(&m) && !has_place_marks("plain"));
        assert_eq!(resolve_place_marks(&own, |_| Some("x".into())), own);
        assert_eq!(resolve_own_place_marks(&own, |o| Some(format!("b.rr:{o}"))), "SUB Go (b.rr:7) has no END SUB");
    }
}
