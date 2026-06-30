use alloc::alloc::{Allocator, Global};
use alloc::vec::Vec;
use core::slice;

use crate::context::DiagnosticContext;
use crate::diagnostic::{Diagnostic, IntoDiagnostic};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticId(usize);

impl DiagnosticId {
    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Debug)]
pub struct Diagnostics<'alloc, A: Allocator, Mode = Unbounded> {
    entries: Vec<Diagnostic<'alloc, A>, &'alloc A>,
    mode: Mode,
}

#[derive(Debug)]
pub struct Unbounded;

#[derive(Debug)]
pub struct Bounded(usize);

impl Default for Diagnostics<'_, Global> {
    fn default() -> Self {
        Self::new_in(&Global)
    }
}

impl<'diag, 'alloc, A: Allocator, Mode> IntoIterator for &'diag Diagnostics<'alloc, A, Mode> {
    type Item = &'diag Diagnostic<'alloc, A>;
    type IntoIter = slice::Iter<'diag, Diagnostic<'alloc, A>>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter()
    }
}

impl<'alloc, A: Allocator> Diagnostics<'alloc, A, Unbounded> {
    #[must_use]
    pub const fn new_in(alloc: &'alloc A) -> Self {
        Self { entries: Vec::new_in(alloc), mode: Unbounded }
    }

    /// Just like `new_in`, but returns a bounded typestate.
    /// The returned bounded typestate does not expose infallible `push` or
    /// `report`, therefore callers must choose `try_push` or `try_report`.
    pub fn new_bounded_in(alloc: &'alloc A) -> Diagnostics<'alloc, A, Bounded> {
        Diagnostics { entries: Vec::new_in(alloc), mode: Bounded(usize::MAX) }
    }

    /// Create a capacity-bounded collection.
    ///
    /// The returned bounded typestate does not expose infallible `push` or
    /// `report`, therefore callers must choose `try_push` or `try_report`.
    #[must_use]
    pub fn with_capacity_in(limit: usize, alloc: &'alloc A) -> Diagnostics<'alloc, A, Bounded> {
        Diagnostics { entries: Vec::with_capacity_in(limit, alloc), mode: Bounded(limit) }
    }

    pub fn push(&mut self, diagnostic: Diagnostic<'alloc, A>) -> DiagnosticId {
        let id = DiagnosticId(self.entries.len());
        self.entries.push(diagnostic);
        id
    }

    pub fn report<D>(&mut self, cx: DiagnosticContext<'_, 'alloc, A>, diagnostic: D) -> DiagnosticId
    where
        D: IntoDiagnostic<'alloc, A>,
    {
        let diagnostic = diagnostic.into_diagnostic(cx);
        self.push(diagnostic)
    }
}

impl<'alloc, A: Allocator> Diagnostics<'alloc, A, Bounded> {
    /// # Errors
    ///
    /// Returns the caller-provided error when this collection is already full.
    pub fn try_push<E>(
        &mut self,
        diagnostic: Diagnostic<'alloc, A>,
        on_capacity_exceeded: impl FnOnce(usize) -> E,
    ) -> Result<DiagnosticId, E> {
        let id = self.try_reserve_id(on_capacity_exceeded)?;
        self.entries.push(diagnostic);
        Ok(id)
    }

    /// # Errors
    ///
    /// Returns the caller-provided error when this collection is already full.
    /// The diagnostic is not built in that case.
    pub fn try_report<E, D>(
        &mut self,
        cx: DiagnosticContext<'_, 'alloc, A>,
        diagnostic: D,
        on_capacity_exceeded: impl FnOnce(usize) -> E,
    ) -> Result<DiagnosticId, E>
    where
        D: IntoDiagnostic<'alloc, A>,
    {
        let id = self.try_reserve_id(on_capacity_exceeded)?;
        let diagnostic = diagnostic.into_diagnostic(cx);
        self.entries.push(diagnostic);
        Ok(id)
    }

    fn try_reserve_id<E>(
        &self,
        on_capacity_exceeded: impl FnOnce(usize) -> E,
    ) -> Result<DiagnosticId, E> {
        if self.entries.len() >= self.mode.0 {
            return Err(on_capacity_exceeded(self.mode.0));
        }

        Ok(DiagnosticId(self.entries.len()))
    }
}

impl<'alloc, A: Allocator, Mode> Diagnostics<'alloc, A, Mode> {
    #[must_use]
    pub const fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[must_use]
    pub const fn as_slice(&self) -> &[Diagnostic<'alloc, A>] {
        self.entries.as_slice()
    }

    pub fn iter(&self) -> slice::Iter<'_, Diagnostic<'_, A>> {
        self.entries.iter()
    }

    #[must_use]
    pub fn into_vec(self) -> Vec<Diagnostic<'alloc, A>, &'alloc A> {
        self.entries
    }
}

#[cfg(test)]
mod tests {
    use alloc::alloc::{Allocator, Global};
    use core::cell::Cell;
    use core::ops::Range;

    use crate::context::DiagnosticContext;
    use crate::diagnostic::{Diagnostic, IntoDiagnostic};
    use crate::diagnostics::{Bounded, Diagnostics};
    use crate::level::DiagnosticLevel;
    use crate::source_map::SourceMap;

    struct Error<'count> {
        level: DiagnosticLevel,
        span: Range<usize>,
        built: Option<&'count Cell<usize>>,
    }

    impl<'alloc, A: Allocator> IntoDiagnostic<'alloc, A> for Error<'_> {
        fn into_diagnostic(self, cx: DiagnosticContext<'_, 'alloc, A>) -> Diagnostic<'alloc, A> {
            if let Some(built) = self.built {
                built.set(built.get() + 1);
            }

            let span = cx.span(self.span);
            cx.builder(self.level)
                .set_primary("unexpected token")
                .add_span(span)
                .span_label(span, "unexpected token")
                .build()
        }
    }

    #[test]
    fn report_stores_diagnostic_and_returns_id() {
        let mut source_map = SourceMap::default();
        let source = source_map.add_source("input.rs", "let = 1;");
        let cx = DiagnosticContext::new(source, &Global);
        let mut diagnostics = Diagnostics::default();

        let id = diagnostics
            .report(cx, Error { level: DiagnosticLevel::Warning, span: 4..5, built: None });

        assert_eq!(id.index(), 0);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics.as_slice()[0].level, DiagnosticLevel::Warning);
        assert_eq!(diagnostics.as_slice()[0].spans[0].range(), 4..5);
    }

    #[test]
    fn try_report_fails_before_building_when_capacity_is_full() {
        let mut source_map = SourceMap::default();
        let source = source_map.add_source("input.rs", "let = 1;");
        let cx = DiagnosticContext::new(source, &Global);
        let built = Cell::new(0);
        let mut diagnostics: Diagnostics<'_, Global, Bounded> =
            Diagnostics::with_capacity_in(0, &Global);

        let result = diagnostics.try_report(
            cx,
            Error { level: DiagnosticLevel::Error, span: 4..5, built: Some(&built) },
            |limit| limit,
        );

        assert_eq!(result, Err(0));
        assert_eq!(built.get(), 0);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn error_recovery_without_file_id_plumbing() {
        struct RecoveringError<'source, 'alloc, A: Allocator> {
            cx: DiagnosticContext<'source, 'alloc, A>,
            diagnostics: Diagnostics<'alloc, A>,
        }

        impl<'source, 'alloc, A: Allocator> RecoveringError<'source, 'alloc, A> {
            fn new(cx: DiagnosticContext<'source, 'alloc, A>, alloc: &'alloc A) -> Self {
                Self { cx, diagnostics: Diagnostics::new_in(alloc) }
            }

            fn recover(&mut self) {
                self.diagnostics.report(
                    self.cx,
                    Error { level: DiagnosticLevel::Error, span: 4..5, built: None },
                );
                self.diagnostics.report(
                    self.cx,
                    Error { level: DiagnosticLevel::Warning, span: 12..13, built: None },
                );
            }
        }

        let mut source_map = SourceMap::default();
        let source = source_map.add_source("input.rs", "let = 1;\nfn {");
        let cx = DiagnosticContext::new(source, &Global);
        let mut error = RecoveringError::new(cx, &Global);

        error.recover();

        let diagnostics = error.diagnostics.as_slice();
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(diagnostics[0].level, DiagnosticLevel::Error);
        assert_eq!(diagnostics[0].spans[0].range(), 4..5);
        assert_eq!(source_map.filename(diagnostics[0].spans[0].file_id()), Some("input.rs"));
        assert_eq!(diagnostics[1].level, DiagnosticLevel::Warning);
        assert_eq!(diagnostics[1].spans[0].range(), 12..13);
        assert_eq!(source_map.filename(diagnostics[1].spans[0].file_id()), Some("input.rs"));
    }
}
