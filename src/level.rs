#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Help,
    Note,
    FailureNote,
}

impl DiagnosticLevel {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            DiagnosticLevel::Error | DiagnosticLevel::FailureNote => "error",
            DiagnosticLevel::Warning => "warning",
            DiagnosticLevel::Help => "help",
            DiagnosticLevel::Note => "note",
        }
    }

    #[cfg(feature = "std")]
    #[must_use]
    pub const fn as_style(&self) -> &'static str {
        use crate::styles::{STYLE_ERROR, STYLE_HELP_NOTE, STYLE_WARNING};

        match self {
            DiagnosticLevel::Error | DiagnosticLevel::FailureNote => STYLE_ERROR,
            DiagnosticLevel::Warning => STYLE_WARNING,
            DiagnosticLevel::Help | DiagnosticLevel::Note => STYLE_HELP_NOTE,
        }
    }
}
