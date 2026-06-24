use alloc::alloc::Allocator;
use alloc::borrow::Cow;
use alloc::vec::Vec;

use crate::level::DiagnosticLevel;
use crate::span::Span;
use crate::suggestion::Suggestion;

#[derive(Debug)]
pub struct SubDiagnostic<'alloc, A: Allocator> {
    pub level: DiagnosticLevel,
    pub message: Cow<'static, str>,
    pub spans: Vec<Span, &'alloc A>,
    pub suggestions: Vec<Suggestion<'alloc, A>, &'alloc A>,
    pub children: Vec<SubDiagnostic<'alloc, A>, &'alloc A>,
}

impl<'alloc, A: Allocator> SubDiagnostic<'alloc, A> {
    pub fn new_in(
        level: DiagnosticLevel,
        message: impl Into<Cow<'static, str>>,
        alloc: &'alloc A,
    ) -> Self {
        Self {
            level,
            message: message.into(),
            spans: Vec::new_in(alloc),
            suggestions: Vec::new_in(alloc),
            children: Vec::new_in(alloc),
        }
    }
}
