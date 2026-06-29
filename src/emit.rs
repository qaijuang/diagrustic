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
    use std::vec::Vec;
    use std::{env, io};

    use super::{Allocator, Diagnostic, EmitDiagnostic, SourceMap};
    use crate::level::DiagnosticLevel;
    use crate::span::{FileId, Span};
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
            if s.eq_ignore_ascii_case("human") {
                Ok(DiagnosticFormat::Human)
            } else if s.eq_ignore_ascii_case("short") {
                Ok(DiagnosticFormat::Short)
            } else if s.eq_ignore_ascii_case("json") {
                Ok(DiagnosticFormat::Json)
            } else {
                Err(())
            }
        }
    }

    impl DiagnosticFormat {
        fn from_env() -> Option<Self> {
            env::var("DIAGRUSTIC_FORMAT").ok().and_then(|s| s.parse().ok())
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
            if s.eq_ignore_ascii_case("auto") {
                Ok(ColorChoice::Auto)
            } else if s.eq_ignore_ascii_case("always") {
                Ok(ColorChoice::Always)
            } else if s.eq_ignore_ascii_case("never") {
                Ok(ColorChoice::Never)
            } else {
                Err(())
            }
        }
    }

    impl ColorChoice {
        fn from_env() -> Option<Self> {
            env::var("DIAGRUSTIC_COLOR").ok().and_then(|s| s.parse().ok())
        }
    }

    #[derive(Debug, Default)]
    pub struct EmitterConfig {
        pub format: DiagnosticFormat,
        pub color: ColorChoice,
    }

    impl EmitterConfig {
        #[must_use]
        pub fn build(mut self) -> Self {
            self.format = DiagnosticFormat::from_env().unwrap_or(self.format);
            self.color = ColorChoice::from_env().unwrap_or(self.color);
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
        level: DiagnosticLevel,
        is_primary: bool,
    }

    #[derive(Copy, Clone)]
    struct WindowHeader<'a> {
        level: DiagnosticLevel,
        message: &'a str,
    }

    #[derive(Copy, Clone)]
    struct SourceWindow<'a> {
        span: Span,
        header: Option<WindowHeader<'a>>,
        start_line: usize,
        end_line: usize,
    }

    #[derive(Copy, Clone)]
    struct LineLabelPosition<'a> {
        label: DisplayLabel<'a>,
        range: Range<usize>,
        is_start: bool,
        is_end: bool,
        is_multiline: bool,
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
                self.print_source_windows(source_map, primary_label.span, &labels, diag.level)?;
            }

            for child in &diag.children {
                self.print_trailing_sub_diag(child)?;
            }

            for suggestion in &diag.suggestions {
                self.print_suggestion(source_map, suggestion)?;
            }

            writeln!(self.writer)
        }

        fn print_source_windows(
            &mut self,
            source_map: &SourceMap<'_, A>,
            primary_span: Span,
            labels: &[DisplayLabel<'_>],
            level: DiagnosticLevel,
        ) -> Result<(), Error> {
            let Some((primary_start_line, _, primary_end_line)) =
                Self::span_line_bounds(source_map, primary_span)
            else {
                return Ok(());
            };
            let primary_is_multiline = primary_start_line != primary_end_line;
            let mut windows = Vec::new_in(self.alloc);
            windows.push(SourceWindow {
                span: primary_span,
                header: None,
                start_line: primary_start_line,
                end_line: primary_end_line,
            });

            for label in labels {
                let Some((label_start_line, _, label_end_line)) =
                    Self::span_line_bounds(source_map, label.span)
                else {
                    continue;
                };
                if label.span.file_id() == primary_span.file_id()
                    && primary_is_multiline
                    && label_end_line + 1 == primary_start_line
                {
                    if let Some(primary_window) = windows.first_mut() {
                        primary_window.start_line = label_start_line.min(primary_window.start_line);
                    }
                    continue;
                }
                if windows.iter().any(|window| {
                    label.span.file_id() == window.span.file_id()
                        && label_start_line >= window.start_line
                        && label_end_line <= window.end_line
                }) {
                    continue;
                }
                let header = if label.is_primary
                    || Self::span_is_multiline(source_map, label.span).unwrap_or(false)
                {
                    None
                } else {
                    label.message.map(|message| WindowHeader { level: label.level, message })
                };
                windows.push(SourceWindow {
                    span: label.span,
                    header,
                    start_line: label_start_line,
                    end_line: label_end_line,
                });
            }

            for window in windows {
                self.print_source_window(source_map, window, labels, level)?;
            }

            Ok(())
        }

        fn print_source_window(
            &mut self,
            source_map: &SourceMap<'_, A>,
            window: SourceWindow<'_>,
            labels: &[DisplayLabel<'_>],
            level: DiagnosticLevel,
        ) -> Result<(), Error> {
            let window_span = window.span;
            let Some(filename) = source_map.filename(window_span.file_id()) else {
                return Ok(());
            };
            let Some((start_line, start_col)) =
                source_map.line_col(window_span.file_id(), window_span.start())
            else {
                return Ok(());
            };

            let file_id = window_span.file_id();
            let line_width = decimal_width(window.end_line).max(decimal_width(window.start_line));
            if let Some(header) = window.header {
                self.print_related_header(header)?;
            }
            write!(self.writer, " ")?;
            self.write_styled(StyleKind::Gutter, "--> ")?;
            writeln!(self.writer, "{filename}:{start_line}:{start_col}")?;
            self.write_gutter(line_width, "")?;

            for line in window.start_line..=window.end_line {
                let Some(source_line) = source_map.line(file_id, line) else {
                    continue;
                };
                let mut positions = self.line_label_positions(source_map, file_id, line, labels);
                positions.sort_by_key(|pos| (pos.range.start, !pos.label.is_primary));
                if let Some(header) = window.header {
                    Self::suppress_header_label_message(&mut positions, window.span, header);
                }
                let source_prefix = Self::line_source_prefix(&positions, level);
                self.write_source_line(line_width, line, source_line, source_prefix)?;
                self.print_line_multiline_openers(line_width, &positions, level)?;

                let inline_message = Self::inline_line_message_position(&positions);
                self.print_line_label_markers(line_width, &positions, level, inline_message)?;
                self.print_line_followup_messages(line_width, &positions, level, inline_message)?;
            }

            self.write_gutter(line_width, "")
        }

        fn print_related_header(&mut self, header: WindowHeader<'_>) -> Result<(), Error> {
            self.write_style_start(StyleKind::Level(header.level))?;
            write!(self.writer, "{}", header.level.as_str())?;
            self.write_reset()?;
            self.write_style_start(StyleKind::HeaderMsg)?;
            write!(self.writer, ": {}", header.message)?;
            self.write_reset()?;
            writeln!(self.writer)
        }

        fn suppress_header_label_message(
            positions: &mut [LineLabelPosition<'_>],
            span: Span,
            header: WindowHeader<'_>,
        ) {
            for position in positions {
                if position.label.span == span && position.label.message == Some(header.message) {
                    position.label.message = None;
                }
            }
        }

        fn print_line_multiline_openers(
            &mut self,
            line_width: usize,
            positions: &[LineLabelPosition<'_>],
            level: DiagnosticLevel,
        ) -> Result<(), Error> {
            for position in positions {
                if !position.is_multiline
                    || !position.is_start
                    || position.is_end
                    || position.range.start <= 1
                {
                    continue;
                }

                let style = if position.label.is_primary {
                    StyleKind::PrimaryLabel(level)
                } else {
                    StyleKind::SecondaryLabel
                };
                self.write_gutter_prefix(line_width)?;
                self.write_spaces(1)?;
                self.write_styled_repeat(style, '_', position.range.start.saturating_sub(2))?;
                self.write_styled_char(style, '-')?;
                writeln!(self.writer)?;
            }
            Ok(())
        }

        fn print_line_label_markers(
            &mut self,
            line_width: usize,
            positions: &[LineLabelPosition<'_>],
            level: DiagnosticLevel,
            inline_message: Option<usize>,
        ) -> Result<(), Error> {
            if !positions.iter().any(|pos| !pos.is_multiline || pos.is_end) {
                return Ok(());
            }

            self.write_gutter_prefix(line_width)?;
            let mut current_col = 1;
            for position in positions {
                let style = if position.label.is_primary {
                    StyleKind::PrimaryLabel(level)
                } else {
                    StyleKind::SecondaryLabel
                };
                let marker = if position.label.is_primary { '^' } else { '-' };
                if position.is_multiline {
                    if !position.is_end {
                        continue;
                    }
                    self.write_styled_char(style, '|')?;
                    self.write_styled_repeat(style, '_', position.range.end.saturating_sub(2))?;
                    self.write_styled_char(style, marker)?;
                    current_col = position.range.end;
                    continue;
                }
                if position.range.end <= current_col {
                    continue;
                }
                let start = position.range.start.max(current_col);
                self.write_spaces(start.saturating_sub(current_col))?;
                self.write_styled_repeat(style, marker, position.range.end - start)?;
                current_col = position.range.end;
            }

            if let Some(position) = inline_message.and_then(|idx| positions.get(idx))
                && let Some(message) = position.label.message
            {
                let style = if position.label.is_primary {
                    StyleKind::PrimaryLabel(level)
                } else {
                    StyleKind::SecondaryLabel
                };
                write!(self.writer, " ")?;
                self.write_styled(style, message)?;
            }

            writeln!(self.writer)
        }

        fn print_line_followup_messages(
            &mut self,
            line_width: usize,
            positions: &[LineLabelPosition<'_>],
            level: DiagnosticLevel,
            inline_message: Option<usize>,
        ) -> Result<(), Error> {
            for (idx, position) in positions.iter().enumerate() {
                if Some(idx) == inline_message || !position.is_end || position.is_multiline {
                    continue;
                }
                let Some(message) = position.label.message else {
                    continue;
                };
                let style = if position.label.is_primary {
                    StyleKind::PrimaryLabel(level)
                } else {
                    StyleKind::SecondaryLabel
                };
                self.write_gutter_prefix(line_width)?;
                self.write_spaces(position.range.start.saturating_sub(1))?;
                self.write_styled(style, "|")?;
                writeln!(self.writer)?;

                self.write_gutter_prefix(line_width)?;
                self.write_spaces(position.range.start.saturating_sub(1))?;
                self.write_styled(style, message)?;
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
            let part = (span, replacement);
            self.print_suggestion_parts(source_map, message, &[part])
        }

        fn print_suggestion_parts(
            &mut self,
            source_map: &SourceMap<'_, A>,
            message: &str,
            parts: &[(Span, &str)],
        ) -> Result<(), Error> {
            self.write_styled(StyleKind::Level(DiagnosticLevel::Help), "help")?;
            writeln!(self.writer, ": {message}")?;
            if parts.is_empty() {
                return Ok(());
            }

            let mut parts = {
                let mut sorted = Vec::new_in(self.alloc);
                sorted.extend(parts.iter().copied());
                sorted
            };
            parts.sort_by_key(|(span, _)| span.start());

            let Some((first_span, _)) = parts.first().copied() else {
                return Ok(());
            };
            let Some((last_span, _)) = parts.last().copied() else {
                return Ok(());
            };
            let file_id = first_span.file_id();
            let mut previous_end = first_span.start();
            for (idx, (span, _)) in parts.iter().enumerate() {
                if span.file_id() != file_id || (idx != 0 && span.start() < previous_end) {
                    return Ok(());
                }
                previous_end = span.end();
            }

            let Some(source) = source_map.source(file_id) else {
                return Ok(());
            };
            let Some((start_line, _)) = source_map.line_col(file_id, first_span.start()) else {
                return Ok(());
            };
            let Some((end_line, _)) = source_map.line_col(file_id, last_span.end()) else {
                return Ok(());
            };
            let Some(window_start) = source_map.line_start(file_id, first_span.start()) else {
                return Ok(());
            };
            let Some(window_end_line_start) = source_map.line_start(file_id, last_span.end())
            else {
                return Ok(());
            };
            let Some(window_end_line) = source_map.line(file_id, end_line) else {
                return Ok(());
            };
            let window_end = window_end_line_start + window_end_line.len();
            let line_width = decimal_width(end_line).max(decimal_width(start_line));

            self.write_gutter(line_width, "")?;
            for line in start_line..=end_line {
                if let Some(source_line) = source_map.line(file_id, line) {
                    self.write_diff_source_line(
                        line_width,
                        line,
                        '-',
                        source_line,
                        StyleKind::Removal,
                    )?;
                }
            }

            self.write_replacement_diff_lines(
                line_width,
                start_line,
                source,
                window_start,
                window_end,
                &parts,
            )?;
            self.write_gutter(line_width, "")
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
            prefix: Option<(StyleKind, char)>,
        ) -> Result<(), Error> {
            self.write_styled_line_number(line_width, line)?;
            write!(self.writer, " ")?;
            self.write_styled(StyleKind::Gutter, "|")?;
            write!(self.writer, " ")?;
            if let Some((style, ch)) = prefix {
                self.write_styled_char(style, ch)?;
            }
            self.write_display_str(source_line)?;
            writeln!(self.writer)
        }

        fn write_diff_source_line(
            &mut self,
            line_width: usize,
            line: usize,
            marker: char,
            source_line: &str,
            style: StyleKind,
        ) -> Result<(), Error> {
            self.write_styled_line_number(line_width, line)?;
            write!(self.writer, " ")?;
            self.write_styled_char(style, marker)?;
            write!(self.writer, " ")?;
            self.write_styled_display(style, source_line)?;
            writeln!(self.writer)
        }

        fn write_replacement_diff_lines(
            &mut self,
            line_width: usize,
            start_line: usize,
            source: &str,
            window_start: usize,
            window_end: usize,
            parts: &[(Span, &str)],
        ) -> Result<(), Error> {
            let mut line = start_line;
            self.write_diff_line_start(line_width, line, '+', StyleKind::Addition)?;

            let mut cursor = window_start;
            for (span, replacement) in parts {
                if span.start() < window_start || span.end() > window_end {
                    self.write_diff_line_end()?;
                    return Ok(());
                }
                self.write_diff_line_chunks(
                    line_width,
                    &source[cursor..span.start()],
                    &mut line,
                    StyleKind::Addition,
                )?;
                self.write_diff_line_chunks(
                    line_width,
                    replacement,
                    &mut line,
                    StyleKind::Addition,
                )?;
                cursor = span.end();
            }
            self.write_diff_line_chunks(
                line_width,
                &source[cursor..window_end],
                &mut line,
                StyleKind::Addition,
            )?;
            self.write_diff_line_end()
        }

        fn write_diff_line_chunks(
            &mut self,
            line_width: usize,
            mut text: &str,
            line: &mut usize,
            style: StyleKind,
        ) -> Result<(), Error> {
            while let Some(newline) = text.find('\n') {
                self.write_display_str(&text[..newline])?;
                self.write_diff_line_end()?;
                *line += 1;
                self.write_diff_line_start(line_width, *line, '+', style)?;
                text = &text[newline + 1..];
            }
            self.write_display_str(text)
        }

        fn write_diff_line_start(
            &mut self,
            line_width: usize,
            line: usize,
            marker: char,
            style: StyleKind,
        ) -> Result<(), Error> {
            self.write_styled_line_number(line_width, line)?;
            write!(self.writer, " ")?;
            self.write_styled_char(style, marker)?;
            write!(self.writer, " ")?;
            self.write_style_start(style)
        }

        fn write_diff_line_end(&mut self) -> Result<(), Error> {
            self.write_reset()?;
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

        fn write_styled_line_number(
            &mut self,
            line_width: usize,
            line: usize,
        ) -> Result<(), Error> {
            self.write_style_start(StyleKind::Gutter)?;
            write!(self.writer, "{line:>line_width$}")?;
            self.write_reset()
        }

        fn write_styled_char(&mut self, style: StyleKind, ch: char) -> Result<(), Error> {
            self.write_style_start(style)?;
            write!(self.writer, "{ch}")?;
            self.write_reset()
        }

        fn write_display_str(&mut self, text: &str) -> Result<(), Error> {
            for ch in text.chars() {
                if ch == '\u{200D}' {
                    continue;
                }
                if ch == '\t' {
                    self.write_spaces(TAB_WIDTH)?;
                } else {
                    write!(self.writer, "{ch}")?;
                }
            }
            Ok(())
        }

        fn write_styled_display(&mut self, style: StyleKind, text: &str) -> Result<(), Error> {
            self.write_style_start(style)?;
            self.write_display_str(text)?;
            self.write_reset()
        }

        fn write_styled_repeat(
            &mut self,
            style: StyleKind,
            ch: char,
            count: usize,
        ) -> Result<(), Error> {
            self.write_style_start(style)?;
            for _ in 0..count {
                write!(self.writer, "{ch}")?;
            }
            self.write_reset()
        }

        fn write_spaces(&mut self, mut count: usize) -> Result<(), Error> {
            const SPACES: &str = "                                                                ";
            while count >= SPACES.len() {
                write!(self.writer, "{SPACES}")?;
                count -= SPACES.len();
            }
            if count != 0 {
                write!(self.writer, "{}", &SPACES[..count])?;
            }
            Ok(())
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

        fn span_line_bounds(
            source_map: &SourceMap<'_, A>,
            span: Span,
        ) -> Option<(usize, usize, usize)> {
            let (start_line, start_col) = source_map.line_col(span.file_id(), span.start())?;
            let (mut end_line, _) = source_map.line_col(span.file_id(), span.end())?;
            if span.end() > span.start()
                && let Some(line_start) = source_map.line_start(span.file_id(), span.end())
                && line_start == span.end()
                && end_line > start_line
            {
                end_line -= 1;
            }
            Some((start_line, start_col, end_line))
        }

        fn span_is_multiline(source_map: &SourceMap<'_, A>, span: Span) -> Option<bool> {
            let (start_line, _, end_line) = Self::span_line_bounds(source_map, span)?;
            Some(start_line != end_line)
        }

        fn line_source_prefix(
            positions: &[LineLabelPosition<'_>],
            level: DiagnosticLevel,
        ) -> Option<(StyleKind, char)> {
            let position = positions
                .iter()
                .find(|pos| pos.is_multiline && pos.label.is_primary && !pos.is_start)
                .or_else(|| positions.iter().find(|pos| pos.is_multiline && !pos.is_start))
                .or_else(|| {
                    positions.iter().find(|pos| {
                        pos.is_multiline
                            && pos.is_start
                            && pos.range.start == 1
                            && pos.label.is_primary
                    })
                })
                .or_else(|| {
                    positions
                        .iter()
                        .find(|pos| pos.is_multiline && pos.is_start && pos.range.start == 1)
                })?;
            let style = if position.label.is_primary {
                StyleKind::PrimaryLabel(level)
            } else {
                StyleKind::SecondaryLabel
            };
            let marker = if position.is_start { '/' } else { '|' };
            Some((style, marker))
        }

        fn inline_line_message_position(positions: &[LineLabelPosition<'_>]) -> Option<usize> {
            positions
                .iter()
                .position(|pos| {
                    pos.label.is_primary
                        && pos.is_multiline
                        && pos.is_end
                        && pos.label.message.is_some()
                })
                .or_else(|| {
                    positions.iter().rposition(|pos| {
                        pos.is_multiline && pos.is_end && pos.label.message.is_some()
                    })
                })
                .or_else(|| {
                    positions.iter().position(|pos| {
                        pos.label.is_primary
                            && pos.is_end
                            && pos.label.message.is_some()
                            && Self::inline_message_fits(pos, positions)
                    })
                })
                .or_else(|| {
                    positions.iter().rposition(|pos| {
                        !pos.label.is_primary
                            && pos.is_end
                            && pos.label.message.is_some()
                            && Self::inline_message_fits(pos, positions)
                    })
                })
        }

        fn inline_message_fits(
            position: &LineLabelPosition<'_>,
            positions: &[LineLabelPosition<'_>],
        ) -> bool {
            let Some(message) = position.label.message else {
                return false;
            };
            let message_end = position.range.end + 1 + display_str_width(message);
            positions
                .iter()
                .find(|other| other.range.start > position.range.start)
                .is_none_or(|other| message_end <= other.range.start)
        }

        fn line_label_positions<'label>(
            &self,
            source_map: &SourceMap<'_, A>,
            file_id: FileId,
            line: usize,
            labels: &'label [DisplayLabel<'_>],
        ) -> Vec<LineLabelPosition<'label>, &'alloc A> {
            let mut positions = Vec::new_in(self.alloc);
            for label in labels {
                if label.span.file_id() != file_id {
                    continue;
                }
                let Some((label_start_line, _)) =
                    source_map.line_col(label.span.file_id(), label.span.start())
                else {
                    continue;
                };
                let Some((mut label_end_line, _)) =
                    source_map.line_col(label.span.file_id(), label.span.end())
                else {
                    continue;
                };
                if label.span.end() > label.span.start()
                    && let Some(line_start) =
                        source_map.line_start(label.span.file_id(), label.span.end())
                    && line_start == label.span.end()
                    && label_end_line > label_start_line
                {
                    label_end_line -= 1;
                }
                if line < label_start_line || line > label_end_line {
                    continue;
                }

                let Some(source_line) = source_map.line(label.span.file_id(), line) else {
                    continue;
                };
                let Some(line_start) = source_map.line_start_for_line(label.span.file_id(), line)
                else {
                    continue;
                };
                let line_end = line_start + source_line.len();
                let start_byte = if line == label_start_line {
                    label.span.start().saturating_sub(line_start)
                } else {
                    0
                };
                let end_byte = if line == label_end_line {
                    label.span.end().min(line_end).saturating_sub(line_start)
                } else {
                    source_line.len()
                };
                let Some(start_col) = display_col(source_line, start_byte) else {
                    continue;
                };
                let mut end_col = display_col(source_line, end_byte).unwrap_or(start_col);
                if end_col <= start_col {
                    end_col = start_col + 1;
                }

                positions.push(LineLabelPosition {
                    label: *label,
                    range: (start_col..end_col).into(),
                    is_start: line == label_start_line,
                    is_end: line == label_end_line,
                    is_multiline: label_start_line != label_end_line,
                });
            }
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
                Suggestion::MultiPart { parts, message, .. } => {
                    let mut suggestion_parts = Vec::new_in(self.alloc);
                    suggestion_parts.extend(
                        parts.iter().map(|(span, replacement)| (*span, replacement.as_ref())),
                    );
                    self.print_suggestion_parts(source_map, message, &suggestion_parts)
                }
            }
        }

        fn collect_labels<'diag>(
            &self,
            diag: &'diag Diagnostic<'_, A>,
        ) -> Vec<DisplayLabel<'diag>, &'alloc A> {
            let mut labels = Vec::new_in(self.alloc);
            for span in &diag.spans {
                labels.push(DisplayLabel {
                    span: *span,
                    message: None,
                    level: diag.level,
                    is_primary: true,
                });
            }
            for child in &diag.children {
                Self::collect_child_labels(child, &mut labels);
            }
            if !labels.iter().any(|label| label.is_primary)
                && let Some(label) = labels.first_mut()
            {
                label.is_primary = true;
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
                        level: child.level,
                        is_primary: false,
                    });
                }
            }
            for child in &child.children {
                Self::collect_child_labels(child, labels);
            }
        }
    }

    const TAB_WIDTH: usize = 4;

    fn display_col(line: &str, byte_offset: usize) -> Option<usize> {
        if byte_offset > line.len() || !line.is_char_boundary(byte_offset) {
            return None;
        }

        let mut col = 1;
        for ch in line[..byte_offset].chars() {
            col += display_width(ch);
        }
        Some(col)
    }

    fn display_str_width(text: &str) -> usize {
        text.chars().map(display_width).sum()
    }

    const fn decimal_width(mut value: usize) -> usize {
        let mut width = 1;
        while value >= 10 {
            value /= 10;
            width += 1;
        }
        width
    }

    const fn display_width(ch: char) -> usize {
        if ch == '\t' {
            return TAB_WIDTH;
        }
        if is_zero_width(ch) {
            return 0;
        }
        if is_wide(ch) { 2 } else { 1 }
    }

    const fn is_zero_width(ch: char) -> bool {
        matches!(
            ch,
            '\u{0300}'..='\u{036F}'
                | '\u{1AB0}'..='\u{1AFF}'
                | '\u{1DC0}'..='\u{1DFF}'
                | '\u{20D0}'..='\u{20FF}'
                | '\u{FE20}'..='\u{FE2F}'
                | '\u{200D}'
                | '\u{FE0E}'..='\u{FE0F}'
        )
    }

    const fn is_wide(ch: char) -> bool {
        matches!(
            ch,
            '\u{1100}'..='\u{115F}'
                | '\u{2329}'..='\u{232A}'
                | '\u{2E80}'..='\u{A4CF}'
                | '\u{AC00}'..='\u{D7A3}'
                | '\u{F900}'..='\u{FAFF}'
                | '\u{FE10}'..='\u{FE19}'
                | '\u{FE30}'..='\u{FE6F}'
                | '\u{FF00}'..='\u{FF60}'
                | '\u{FFE0}'..='\u{FFE6}'
                | '\u{1F300}'..='\u{1FAFF}'
        )
    }

    #[cfg(test)]
    mod tests {
        use super::{display_col, display_width};

        #[test]
        fn display_width_treats_combining_marks_as_zero_width() {
            assert_eq!(display_width('\u{0301}'), 0);
            assert_eq!(display_width('\u{200D}'), 0);
            assert_eq!(display_width('\u{FE0F}'), 0);
        }

        #[test]
        fn display_col_does_not_advance_for_combining_mark() {
            let line = "cafe\u{301}_value";

            assert_eq!(display_col(line, line.find('_').unwrap()), Some(5));
        }
    }
}
