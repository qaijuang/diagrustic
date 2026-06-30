use alloc::alloc::Allocator;

use crate::builder::DiagnosticBuilder;
use crate::level::DiagnosticLevel;
use crate::source_map::Source;
use crate::span::{ByteSpan, Span};

pub struct DiagnosticContext<'source, 'alloc, A: Allocator> {
    source: Source<'source, 'alloc, A>,
    alloc: &'alloc A,
}

impl<A: Allocator> Copy for DiagnosticContext<'_, '_, A> {}

impl<A: Allocator> Clone for DiagnosticContext<'_, '_, A> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'source, 'alloc, A: Allocator> DiagnosticContext<'source, 'alloc, A> {
    #[must_use]
    pub const fn new(source: Source<'source, 'alloc, A>, alloc: &'alloc A) -> Self {
        Self { source, alloc }
    }

    #[must_use]
    pub const fn alloc(&self) -> &'alloc A {
        self.alloc
    }

    #[must_use]
    pub fn span(&self, span: impl Into<ByteSpan>) -> Span {
        self.source.span(span)
    }

    #[must_use]
    pub const fn builder(&self, level: DiagnosticLevel) -> DiagnosticBuilder<'alloc, A> {
        DiagnosticBuilder::new_in(level, self.alloc)
    }

    #[must_use]
    pub const fn source(&self) -> Source<'source, 'alloc, A> {
        self.source
    }
}
