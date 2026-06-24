use alloc::alloc::{Allocator, Global};
use alloc::borrow::Cow;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::panic::Location;

use crate::applicability::Applicability;
use crate::diagnostic::Diagnostic;
use crate::level::DiagnosticLevel;
use crate::span::Span;
use crate::sub_diag::SubDiagnostic;
use crate::suggestion::Suggestion;

// Private structure to defer a label until `DiagnosticBuilder::build()`.
struct LazyLabel<'alloc, A: Allocator> {
    span: Span,
    level: DiagnosticLevel,
    f: Box<dyn FnOnce() -> Cow<'static, str>, &'alloc A>,
}

pub struct DiagnosticBuilder<'alloc, A: Allocator = Global> {
    level: DiagnosticLevel,
    primary: Option<Cow<'static, str>>,
    code: Option<Cow<'static, str>>,
    spans: Vec<Span, &'alloc A>,
    suggestions: Vec<Suggestion<'alloc, A>, &'alloc A>,
    children: Vec<SubDiagnostic<'alloc, A>, &'alloc A>,
    lazy_labels: Vec<LazyLabel<'alloc, A>, &'alloc A>,
    alloc: &'alloc A,
    created_at: &'static Location<'static>,
}

impl DiagnosticBuilder<'_> {
    #[must_use]
    #[track_caller]
    pub fn new(level: DiagnosticLevel) -> Self {
        Self {
            level,
            primary: None,
            code: None,
            spans: Vec::new_in(&Global),
            suggestions: Vec::new_in(&Global),
            children: Vec::new_in(&Global),
            lazy_labels: Vec::new_in(&Global),
            alloc: &Global,
            created_at: Location::caller(),
        }
    }
}

impl<'alloc, A: Allocator> DiagnosticBuilder<'alloc, A> {
    #[track_caller]
    pub fn new_in(level: DiagnosticLevel, alloc: &'alloc A) -> Self {
        Self {
            level,
            primary: None,
            code: None,
            spans: Vec::new_in(alloc),
            suggestions: Vec::new_in(alloc),
            children: Vec::new_in(alloc),
            lazy_labels: Vec::new_in(alloc),
            alloc,
            created_at: Location::caller(),
        }
    }

    #[must_use]
    pub fn set_primary(mut self, msg: impl Into<Cow<'static, str>>) -> Self {
        self.primary = Some(msg.into());
        self
    }

    #[must_use]
    pub fn set_code(mut self, code: impl Into<Cow<'static, str>>) -> Self {
        self.code = Some(code.into());
        self
    }

    #[must_use]
    pub fn add_span(mut self, span: Span) -> Self {
        self.spans.push(span);
        self
    }

    #[must_use]
    pub fn add_sub_diagnostic(mut self, sub: SubDiagnostic<'alloc, A>) -> Self {
        self.children.push(sub);
        self
    }

    #[must_use]
    pub fn add_suggestion(mut self, sugg: Suggestion<'alloc, A>) -> Self {
        self.suggestions.push(sugg);
        self
    }

    /// Attach a label to a span to create a Help `SubDiagnostic` eagerly.
    #[must_use]
    pub fn span_label(mut self, span: Span, label: impl Into<Cow<'static, str>>) -> Self {
        let mut sub = SubDiagnostic::new_in(DiagnosticLevel::Help, label.into(), self.alloc);
        let mut spans = Vec::new_in(self.alloc);
        spans.push(span);
        sub.spans = spans;
        self.children.push(sub);
        self
    }

    /// Just like [`DiagnosticBuilder::span_label`], but the closure is lazily evaluated in [`DiagnosticBuilder::build()`].
    #[must_use]
    #[track_caller]
    pub fn span_label_with(
        mut self,
        span: Span,
        f: impl FnOnce() -> Cow<'static, str> + 'static,
    ) -> Self {
        self.lazy_labels.push(LazyLabel {
            span,
            level: DiagnosticLevel::Help,
            f: Box::new_in(f, self.alloc),
        });
        self
    }

    #[must_use]
    pub fn span_suggestion(
        mut self,
        span: Span,
        message: impl Into<Cow<'static, str>>,
        replacement: impl Into<Cow<'static, str>>,
        applicability: Applicability,
    ) -> Self {
        self.suggestions.push(Suggestion::Replacement {
            span,
            message: message.into(),
            replacement: replacement.into(),
            applicability,
        });
        self
    }

    /// Finalize the diagnostic. All lazy labels are evaluated here.
    #[must_use]
    pub fn build(mut self) -> Diagnostic<'alloc, A> {
        // Evaluate lazy labels
        for ll in self.lazy_labels {
            let message = (ll.f)();
            let mut sub = SubDiagnostic::new_in(ll.level, message, self.alloc);
            let mut spans = Vec::new_in(self.alloc);
            spans.push(ll.span);
            sub.spans = spans;
            self.children.push(sub);
        }
        Diagnostic {
            level: self.level,
            primary: self.primary.unwrap_or_default(),
            code: self.code,
            spans: self.spans,
            suggestions: self.suggestions,
            children: self.children,
            created_at: self.created_at,
        }
    }
}
