use crate::level::DiagnosticLevel;

pub(crate) const RESET: &str = "\x1b[0m";

const BOLD: &str = "\x1b[1m";
pub(crate) const STYLE_ERROR: &str = "\x1b[1m\x1b[91m";
#[cfg(windows)]
pub(crate) const STYLE_WARNING: &str = "\x1b[1m\x1b[93m";

#[cfg(not(windows))]
pub(crate) const STYLE_WARNING: &str = "\x1b[1m\x1b[33m";
pub(crate) const STYLE_HELP: &str = "\x1b[1m\x1b[96m";
pub(crate) const STYLE_NOTE: &str = "\x1b[1m\x1b[92m";
const ADDITION: &str = "\x1b[92m";
const REMOVAL: &str = "\x1b[91m";

#[cfg(windows)]
const GUTTER_STYLE: &str = "\x1b[1m\x1b[96m";

#[cfg(not(windows))]
const GUTTER_STYLE: &str = "\x1b[1m\x1b[94m";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StyleKind {
    Level(DiagnosticLevel),
    HeaderMsg,
    Gutter,
    PrimaryLabel(DiagnosticLevel),
    SecondaryLabel,
    Addition,
    Removal,
}

impl StyleKind {
    pub(crate) const fn prefix(self) -> &'static str {
        match self {
            Self::Level(level) | Self::PrimaryLabel(level) => level.as_style(),
            Self::HeaderMsg => BOLD,
            Self::Gutter | Self::SecondaryLabel => GUTTER_STYLE,
            Self::Addition => ADDITION,
            Self::Removal => REMOVAL,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::alloc::Global;

    use super::*;
    use crate::builder::DiagnosticBuilder;
    use crate::source_map::SourceMap;
    use crate::{ColorChoice, DiagnosticFormat, EmitDiagnostic, EmitterConfig, TerminalEmitter};

    fn levels() -> String {
        let mut source_map = SourceMap::default();
        let file_id = source_map.add_file("levels.rs", "let signal = 1;");
        let span = source_map.span(file_id, 4..10);

        let mut output = Vec::new();
        for (level, message) in [
            (DiagnosticLevel::Error, "error level"),
            (DiagnosticLevel::Warning, "warning level"),
            (DiagnosticLevel::Help, "help level"),
            (DiagnosticLevel::Note, "note level"),
        ] {
            let diag = DiagnosticBuilder::new(level)
                .set_primary(message)
                .add_span(span)
                .span_label(span, "level marker")
                .build();
            {
                let config =
                    EmitterConfig { format: DiagnosticFormat::Human, color: ColorChoice::Always };
                let mut emitter = TerminalEmitter::with_config(&mut output, config, &Global);
                emitter.emit(&diag, &source_map).unwrap();
            }
        }
        String::from_utf8(output).unwrap()
    }

    #[cfg(windows)]
    #[test]
    fn ansi_styles() {
        let output = levels();

        assert!(output.contains("\x1b[1m\x1b[93mwarning\x1b[0m"));
        assert!(output.contains("\x1b[1m\x1b[96m--> \x1b[0m"));
        assert!(!output.contains("\x1b[1m\x1b[33mwarning\x1b[0m"));
        assert!(!output.contains("\x1b[1m\x1b[94m--> \x1b[0m"));

        assert!(output.contains("\x1b[1m\x1b[91merror\x1b[0m"));
        assert!(output.contains("\x1b[1m\x1b[92mnote\x1b[0m"));
        assert!(output.contains("\x1b[1m\x1b[96mhelp\x1b[0m"));
    }

    #[cfg(not(windows))]
    #[test]
    fn ansi_styles() {
        let output = levels();

        assert!(output.contains("\x1b[1m\x1b[33mwarning\x1b[0m"));
        assert!(output.contains("\x1b[1m\x1b[94m--> \x1b[0m"));
        assert!(!output.contains("\x1b[1m\x1b[93mwarning\x1b[0m"));
        assert!(!output.contains("\x1b[1m\x1b[96m--> \x1b[0m"));

        assert!(output.contains("\x1b[1m\x1b[91merror\x1b[0m"));
        assert!(output.contains("\x1b[1m\x1b[92mnote\x1b[0m"));
        assert!(output.contains("\x1b[1m\x1b[96mhelp\x1b[0m"));
    }
}
