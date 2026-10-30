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

const PLACE_OPEN: &str = "{{at:";
const PLACE_CLOSE: &str = "}}";

/// `text` with every [`place_mark`] replaced by what `place` says of its
/// offset (`None`: "an earlier line").
pub fn resolve_place_marks(text: &str, mut place: impl FnMut(usize) -> Option<String>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find(PLACE_OPEN) {
        let after = &rest[i + PLACE_OPEN.len()..];
        let Some(j) = after.find(PLACE_CLOSE) else { break };
        let Ok(offset) = after[..j].parse::<usize>() else {
            out.push_str(&rest[..i + PLACE_OPEN.len()]);
            rest = after;
            continue;
        };
        out.push_str(&rest[..i]);
        out.push_str(&place(offset).unwrap_or_else(|| "an earlier line".to_string()));
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
    }
}
