use alloc::alloc::Allocator;

use crate::diagnostic::Diagnostic;
use crate::source_map::SourceMap;

pub trait EmitDiagnostic<A: Allocator, E> {
    #[allow(clippy::missing_errors_doc)]
    fn emit(&mut self, diag: &Diagnostic<'_, A>, source_map: &SourceMap<'_, A>) -> Result<(), E>;
}

#[cfg(feature = "std")]
pub mod terminal {
    use std::alloc::Global;
    use std::io::{Error, ErrorKind, Write};
    use std::range::Range;
    use std::str::FromStr;
    use std::string::ToString;
    use std::vec::Vec;
    use std::{format, io};

    use super::{Allocator, Diagnostic, EmitDiagnostic, SourceMap};
    use crate::level::DiagnosticLevel;
    use crate::span::Span;
    use crate::styles::{RESET, StyleKind};
    use crate::sub_diag::SubDiagnostic;
    use crate::suggestion::Suggestion;
    use crate::sys;

    #[derive(Debug, Default)]
    pub enum DiagnosticFormat {
        #[default]
        Human,
        Short,
        Json,
    }

    impl FromStr for DiagnosticFormat {
        type Err = ();

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s.to_ascii_lowercase().as_str() {
                "human" => Ok(DiagnosticFormat::Human),
                "short" => Ok(DiagnosticFormat::Short),
                "json" => Ok(DiagnosticFormat::Json),
                _ => Err(()),
            }
        }
    }

    impl DiagnosticFormat {
        fn from_env() -> Option<Self> {
            option_env!("DIAGRUSTIC_FORMAT").and_then(|s| s.parse().ok())
        }
    }

    #[derive(Debug, Default)]
    pub enum ColorChoice {
        #[default]
        Auto,
        Always,
        Never,
    }

    impl FromStr for ColorChoice {
        type Err = ();

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s.to_ascii_lowercase().as_str() {
                "auto" => Ok(ColorChoice::Auto),
                "always" => Ok(ColorChoice::Always),
                "never" => Ok(ColorChoice::Never),
                _ => Err(()),
            }
        }
    }

    impl ColorChoice {
        fn from_env() -> Option<Self> {
            option_env!("DIAGRUSTIC_COLOR").and_then(|s| s.parse().ok())
        }
    }

    #[derive(Debug)]
    pub struct EmitterConfig {
        pub format: DiagnosticFormat,
        pub color: ColorChoice,
    }

    impl Default for EmitterConfig {
        fn default() -> Self {
            Self {
                format: DiagnosticFormat::from_env().unwrap_or_default(),
                color: ColorChoice::from_env().unwrap_or_default(),
            }
        }
    }

    impl EmitterConfig {
        #[must_use]
        pub fn build(mut self) -> Self {
            if matches!(self.color, ColorChoice::Auto) {
                self.color = if sys::stderr_supports_color() {
                    ColorChoice::Always
                } else {
                    ColorChoice::Never
                };
            }
            self
        }
    }

    #[derive(Copy, Clone)]
    struct DisplayLabel<'a> {
        span: Span,
        message: Option<&'a str>,
        is_primary: bool,
    }

    #[derive(Copy, Clone)]
    struct LabelPosition<'a> {
        label: DisplayLabel<'a>,
        range: Range<usize>,
    }

    pub struct TerminalEmitter<'alloc, W: Write, A: Allocator> {
        renderer: HumanRenderer<'alloc, W, A>,
    }

    impl Default for TerminalEmitter<'_, io::Stderr, Global> {
        fn default() -> Self {
            Self::new_in(io::stderr(), &Global)
        }
    }

    impl<'alloc, W: Write, A: Allocator> TerminalEmitter<'alloc, W, A> {
        pub fn new_in(writer: W, alloc: &'alloc A) -> Self {
            Self::with_config(writer, EmitterConfig::default(), alloc)
        }

        pub fn with_config(writer: W, config: EmitterConfig, alloc: &'alloc A) -> Self {
            let renderer = HumanRenderer::new_in(writer, config, alloc);
            Self { renderer }
        }
    }

    impl<W: Write, A: Allocator> EmitDiagnostic<A, Error> for TerminalEmitter<'_, W, A> {
        fn emit(
            &mut self,
            diag: &Diagnostic<'_, A>,
            source_map: &SourceMap<'_, A>,
        ) -> Result<(), Error> {
            self.renderer.emit(diag, source_map)
        }
    }

    struct HumanRenderer<'alloc, W: Write, A: Allocator> {
        writer: W,
        config: EmitterConfig,
        alloc: &'alloc A,
    }

    impl Default for HumanRenderer<'_, io::Stderr, Global> {
        fn default() -> Self {
            Self::new_in(io::stderr(), EmitterConfig::default(), &Global)
        }
    }

    impl<W: Write, A: Allocator> EmitDiagnostic<A, Error> for HumanRenderer<'_, W, A> {
        fn emit(
            &mut self,
            diag: &Diagnostic<'_, A>,
            source_map: &SourceMap<'_, A>,
        ) -> Result<(), Error> {
            self.emit(diag, source_map)
        }
    }

    impl<W: Write, A: Allocator> HumanRenderer<'_, W, A> {
        fn emit(
            &mut self,
            diag: &Diagnostic<'_, A>,
            source_map: &SourceMap<'_, A>,
        ) -> Result<(), Error> {
            if !matches!(self.config.format, DiagnosticFormat::Human) {
                return Err(Error::new(
                    ErrorKind::Unsupported,
                    "diagnostic format is not implemented",
                ));
            }

            self.print_header(diag)?;

            let labels = self.collect_labels(diag);
            if let Some(primary_label) =
                labels.iter().find(|label| label.is_primary).or_else(|| labels.first())
            {
                self.print_source_window(source_map, primary_label.span, &labels, diag.level)?;
            }

            for child in &diag.children {
                self.print_trailing_sub_diag(child)?;
            }

            for suggestion in &diag.suggestions {
                self.print_suggestion(source_map, suggestion)?;
            }

            writeln!(self.writer)
        }

        fn print_source_window(
            &mut self,
            source_map: &SourceMap<'_, A>,
            primary_span: Span,
            labels: &[DisplayLabel<'_>],
            level: DiagnosticLevel,
        ) -> Result<(), Error> {
            let Some(filename) = source_map.filename(primary_span.file_id()) else {
                return Ok(());
            };
            let Some((line, col)) =
                source_map.line_col(primary_span.file_id(), primary_span.start())
            else {
                return Ok(());
            };
            let Some(source_line) = source_map.line(primary_span.file_id(), line) else {
                return Ok(());
            };

            let line_width = line.to_string().len();
            write!(self.writer, " ")?;
            self.write_styled(StyleKind::Gutter, "--> ")?;
            writeln!(self.writer, "{filename}:{line}:{col}")?;
            self.write_gutter(line_width, "")?;
            self.write_source_line(line_width, line, source_line)?;

            let mut positions = self.label_positions(source_map, primary_span, line, labels);
            positions.sort_by_key(|pos| (pos.range.start, !pos.label.is_primary));
            self.print_label_markers(line_width, &positions, level)?;
            self.print_secondary_label_messages(line_width, &positions)?;
            self.write_gutter(line_width, "")
        }

        fn print_label_markers(
            &mut self,
            line_width: usize,
            positions: &[LabelPosition<'_>],
            level: DiagnosticLevel,
        ) -> Result<(), Error> {
            if positions.is_empty() {
                return Ok(());
            }

            self.write_gutter_prefix(line_width)?;
            let mut current_col = 1;
            for position in positions {
                let marker = if position.label.is_primary { '^' } else { '-' };
                if position.range.end <= current_col {
                    continue;
                }
                let start = position.range.start.max(current_col);
                write!(self.writer, "{}", " ".repeat(start.saturating_sub(current_col)))?;
                let style = if position.label.is_primary {
                    StyleKind::PrimaryLabel(level)
                } else {
                    StyleKind::SecondaryLabel
                };
                self.write_styled_repeat(style, marker, position.range.end - start)?;
                current_col = position.range.end;
            }

            if let Some(message) = positions
                .iter()
                .find_map(|pos| pos.label.is_primary.then_some(pos.label.message).flatten())
            {
                write!(self.writer, " ")?;
                self.write_styled(StyleKind::PrimaryLabel(level), message)?;
            }

            writeln!(self.writer)
        }

        fn print_secondary_label_messages(
            &mut self,
            line_width: usize,
            positions: &[LabelPosition<'_>],
        ) -> Result<(), Error> {
            for position in positions {
                if position.label.is_primary {
                    continue;
                }
                let Some(message) = position.label.message else {
                    continue;
                };
                self.write_gutter_prefix(line_width)?;
                write!(self.writer, "{}", " ".repeat(position.range.start.saturating_sub(1)))?;
                self.write_styled(StyleKind::SecondaryLabel, "|")?;
                writeln!(self.writer)?;

                self.write_gutter_prefix(line_width)?;
                write!(self.writer, "{}", " ".repeat(position.range.start.saturating_sub(1)))?;
                self.write_styled(StyleKind::SecondaryLabel, message)?;
                writeln!(self.writer)?;
            }
            Ok(())
        }

        fn print_suggestion_replacement(
            &mut self,
            source_map: &SourceMap<'_, A>,
            span: Span,
            message: &str,
            replacement: &str,
        ) -> Result<(), Error> {
            self.write_styled(StyleKind::Level(DiagnosticLevel::Help), "help")?;
            writeln!(self.writer, ": {message}")?;

            let Some((line_num, start_col)) = source_map.line_col(span.file_id(), span.start())
            else {
                return Ok(());
            };
            let Some(line_start) = source_map.line_start(span.file_id(), span.start()) else {
                return Ok(());
            };
            let Some(source_line) = source_map.line(span.file_id(), line_num) else {
                return Ok(());
            };

            let offset_in_line = span.start() - line_start;
            let end_in_line = offset_in_line + span.len();
            if end_in_line > source_line.len() {
                return Ok(());
            }

            let before = &source_line[..offset_in_line];
            let after = &source_line[end_in_line..];
            let line_width = line_num.to_string().len();

            self.write_gutter(line_width, "")?;
            self.write_source_line_with_parts(line_width, line_num, before, replacement, after)?;

            self.write_gutter_prefix(line_width)?;
            write!(self.writer, "{}", " ".repeat(start_col.saturating_sub(1)))?;
            self.write_styled_repeat(StyleKind::Addition, '+', replacement.chars().count().max(1))?;
            writeln!(self.writer)
        }

        fn write_gutter(&mut self, line_width: usize, body: &str) -> Result<(), Error> {
            if body.is_empty() {
                write!(self.writer, "{:>line_width$} ", "")?;
                self.write_styled(StyleKind::Gutter, "|")?;
                writeln!(self.writer)
            } else {
                self.write_gutter_prefix(line_width)?;
                writeln!(self.writer, "{body}")
            }
        }

        fn write_gutter_prefix(&mut self, line_width: usize) -> Result<(), Error> {
            write!(self.writer, "{:>line_width$} ", "")?;
            self.write_styled(StyleKind::Gutter, "|")?;
            write!(self.writer, " ")
        }

        fn write_source_line(
            &mut self,
            line_width: usize,
            line: usize,
            source_line: &str,
        ) -> Result<(), Error> {
            self.write_styled(StyleKind::Gutter, &format!("{line:>line_width$}"))?;
            write!(self.writer, " ")?;
            self.write_styled(StyleKind::Gutter, "|")?;
            writeln!(self.writer, " {source_line}")
        }

        fn write_source_line_with_parts(
            &mut self,
            line_width: usize,
            line: usize,
            before: &str,
            replacement: &str,
            after: &str,
        ) -> Result<(), Error> {
            self.write_styled(StyleKind::Gutter, &format!("{line:>line_width$}"))?;
            write!(self.writer, " ")?;
            self.write_styled(StyleKind::Gutter, "|")?;
            write!(self.writer, " {before}{replacement}{after}")?;
            writeln!(self.writer)
        }

        fn print_header(&mut self, diag: &Diagnostic<'_, A>) -> Result<(), Error> {
            self.write_style_start(StyleKind::Level(diag.level))?;
            write!(self.writer, "{}", diag.level.as_str())?;
            let code_str = diag.code.as_deref().unwrap_or_default();
            if !code_str.is_empty() {
                write!(self.writer, "[{code_str}]")?;
            }
            self.write_reset()?;
            self.write_style_start(StyleKind::HeaderMsg)?;
            write!(self.writer, ": {}", diag.primary)?;
            self.write_reset()?;
            writeln!(self.writer)
        }

        fn write_styled(&mut self, style: StyleKind, text: &str) -> Result<(), Error> {
            self.write_style_start(style)?;
            write!(self.writer, "{text}")?;
            self.write_reset()
        }

        fn write_styled_repeat(
            &mut self,
            style: StyleKind,
            ch: char,
            count: usize,
        ) -> Result<(), Error> {
            self.write_style_start(style)?;
            write!(self.writer, "{}", ch.to_string().repeat(count))?;
            self.write_reset()
        }

        fn write_style_start(&mut self, style: StyleKind) -> Result<(), Error> {
            if !matches!(self.config.color, ColorChoice::Always) {
                return Ok(());
            }
            write!(self.writer, "{}", style.prefix())
        }

        fn write_reset(&mut self) -> Result<(), Error> {
            if matches!(self.config.color, ColorChoice::Always) {
                write!(self.writer, "{RESET}")?;
            }
            Ok(())
        }
    }

    impl<'alloc, W: Write, A: Allocator> HumanRenderer<'alloc, W, A> {
        fn new_in(writer: W, config: EmitterConfig, alloc: &'alloc A) -> Self {
            let config = config.build();
            Self { writer, config, alloc }
        }

        fn label_positions<'label>(
            &self,
            source_map: &SourceMap<'_, A>,
            primary_span: Span,
            primary_line: usize,
            labels: &'label [DisplayLabel<'_>],
        ) -> Vec<LabelPosition<'label>, &'alloc A> {
            let labels = labels.iter().filter_map(|label| {
                if label.span.file_id() != primary_span.file_id() {
                    return None;
                }
                let (line, start_col) =
                    source_map.line_col(label.span.file_id(), label.span.start())?;
                if line != primary_line {
                    return None;
                }
                let line_start = source_map.line_start(label.span.file_id(), label.span.start())?;
                let width = label.span.end().saturating_sub(label.span.start()).max(1);
                let end_col = start_col + width;
                let line_end =
                    line_start + source_map.line(label.span.file_id(), line).map_or(0, str::len);
                if label.span.end() > line_end {
                    return None;
                }

                Some(LabelPosition { label: *label, range: (start_col..end_col).into() })
            });

            let mut positions = Vec::new_in(self.alloc);
            positions.extend(labels);
            positions
        }

        fn print_trailing_sub_diag(&mut self, sub: &SubDiagnostic<'_, A>) -> Result<(), Error> {
            if sub.spans.is_empty() {
                writeln!(self.writer, "  = {}: {}", sub.level.as_str(), sub.message)?;
            }
            for child in &sub.children {
                self.print_trailing_sub_diag(child)?;
            }
            Ok(())
        }

        fn print_suggestion(
            &mut self,
            source_map: &SourceMap<'_, A>,
            suggestion: &Suggestion<'_, A>,
        ) -> Result<(), Error> {
            match suggestion {
                Suggestion::Replacement { span, message, replacement, .. } => {
                    self.print_suggestion_replacement(source_map, *span, message, replacement)
                }
                Suggestion::MultiPart { message, .. } => {
                    self.write_styled(StyleKind::Level(DiagnosticLevel::Help), "help")?;
                    writeln!(self.writer, ": {message}")
                }
            }
        }

        fn collect_labels<'diag>(
            &self,
            diag: &'diag Diagnostic<'_, A>,
        ) -> Vec<DisplayLabel<'diag>, &'alloc A> {
            let mut labels = Vec::new_in(self.alloc);
            for span in &diag.spans {
                labels.push(DisplayLabel { span: *span, message: None, is_primary: true });
            }
            for child in &diag.children {
                Self::collect_child_labels(child, &mut labels);
            }
            labels
        }

        fn collect_child_labels<'diag>(
            child: &'diag SubDiagnostic<'_, A>,
            labels: &mut Vec<DisplayLabel<'diag>, &'alloc A>,
        ) {
            for span in &child.spans {
                if let Some(primary) = labels.iter_mut().find(|label| {
                    label.is_primary && label.span == *span && label.message.is_none()
                }) {
                    primary.message = Some(child.message.as_ref());
                } else {
                    labels.push(DisplayLabel {
                        span: *span,
                        message: Some(child.message.as_ref()),
                        is_primary: false,
                    });
                }
            }
            for child in &child.children {
                Self::collect_child_labels(child, labels);
            }
        }
    }
}
