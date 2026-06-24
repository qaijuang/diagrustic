use crate::level::DiagnosticLevel;

pub(crate) const RESET: &str = "\x1b[0m";

const BOLD: &str = "\x1b[1m";
pub(crate) const STYLE_ERROR: &str = "\x1b[1m\x1b[91m";
pub(crate) const STYLE_WARNING: &str = "\x1b[1m\x1b[33m";
pub(crate) const STYLE_HELP_NOTE: &str = "\x1b[1m\x1b[92m";
const ADDITION: &str = "\x1b[92m";

#[cfg(windows)]
const GUTTER_STYLE: &str = "\x1b[1m\x1b[96m";

#[cfg(not(windows))]
const GUTTER_STYLE: &str = "\x1b[1m\x1b[94m";

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum StyleKind {
    Level(DiagnosticLevel),
    HeaderMsg,
    Gutter,
    PrimaryLabel(DiagnosticLevel),
    SecondaryLabel,
    Addition,
}

impl StyleKind {
    pub(crate) const fn prefix(self) -> &'static str {
        match self {
            Self::Level(level) | Self::PrimaryLabel(level) => level.as_style(),
            Self::HeaderMsg => BOLD,
            Self::Gutter | Self::SecondaryLabel => GUTTER_STYLE,
            Self::Addition => ADDITION,
        }
    }
}
