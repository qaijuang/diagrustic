use alloc::alloc::Allocator;
use alloc::vec::Vec;

use crate::acow::Acow;
use crate::applicability::Applicability;
use crate::span::Span;

#[derive(Debug, PartialEq, Eq)]
pub enum Suggestion<'alloc, A: Allocator> {
    Replacement {
        span: Span,
        message: Acow<'alloc, A>,
        replacement: Acow<'alloc, A>,
        applicability: Applicability,
    },
    MultiPart {
        parts: Vec<(Span, Acow<'alloc, A>), &'alloc A>,
        message: Acow<'alloc, A>,
        applicability: Applicability,
    },
}
