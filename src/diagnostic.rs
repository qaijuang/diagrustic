use alloc::alloc::Allocator;
use alloc::vec::Vec;
use core::panic::Location;

use crate::acow::Acow;
use crate::level::DiagnosticLevel;
use crate::span::Span;
use crate::sub_diag::SubDiagnostic;
use crate::suggestion::Suggestion;

#[derive(Debug)]
pub struct Diagnostic<'alloc, A: Allocator> {
    pub level: DiagnosticLevel,
    pub primary: Acow<'alloc, A>,
    pub code: Option<Acow<'alloc, A>>,
    pub spans: Vec<Span, &'alloc A>,
    pub suggestions: Vec<Suggestion<'alloc, A>, &'alloc A>,
    pub children: Vec<SubDiagnostic<'alloc, A>, &'alloc A>,
    pub created_at: &'static Location<'static>,
}
