use std::fmt;

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