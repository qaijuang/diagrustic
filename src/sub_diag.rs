use alloc::alloc::Allocator;
use alloc::vec::Vec;

use crate::acow::{Acow, IntoAcow};
use crate::level::DiagnosticLevel;
use crate::span::Span;
use crate::suggestion::Suggestion;

#[derive(Debug)]
pub struct SubDiagnostic<'alloc, A: Allocator> {
    pub level: DiagnosticLevel,
    pub message: Acow<'alloc, A>,
    pub spans: Vec<Span, &'alloc A>,
    pub suggestions: Vec<Suggestion<'alloc, A>, &'alloc A>,
    pub children: Vec<SubDiagnostic<'alloc, A>, &'alloc A>,
}

impl<'alloc, A: Allocator> SubDiagnostic<'alloc, A> {
    pub fn new_in(
        level: DiagnosticLevel,
        message: impl IntoAcow<'alloc, A>,
        alloc: &'alloc A,
    ) -> Self {
        Self {
            level,
            message: message.into_acow(alloc),
            spans: Vec::new_in(alloc),
            suggestions: Vec::new_in(alloc),
            children: Vec::new_in(alloc),
        }
    }
}
