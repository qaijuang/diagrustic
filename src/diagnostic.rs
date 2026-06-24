use alloc::alloc::Allocator;
use alloc::borrow::Cow;
use alloc::vec::Vec;
use core::panic::Location;

use crate::level::DiagnosticLevel;
use crate::span::Span;
use crate::sub_diag::SubDiagnostic;
use crate::suggestion::Suggestion;

#[derive(Debug)]
pub struct Diagnostic<'alloc, A: Allocator> {
    pub level: DiagnosticLevel,
    pub primary: Cow<'static, str>,
    pub code: Option<Cow<'static, str>>,
    pub spans: Vec<Span, &'alloc A>,
    pub suggestions: Vec<Suggestion<'alloc, A>, &'alloc A>,
    pub children: Vec<SubDiagnostic<'alloc, A>, &'alloc A>,
    pub created_at: &'static Location<'static>,
}
