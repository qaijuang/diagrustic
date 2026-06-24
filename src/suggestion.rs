use alloc::alloc::Allocator;
use alloc::borrow::Cow;
use alloc::vec::Vec;

use crate::applicability::Applicability;
use crate::span::Span;

#[derive(Debug, PartialEq, Eq)]
pub enum Suggestion<'alloc, A: Allocator> {
    Replacement {
        span: Span,
        message: Cow<'static, str>,
        replacement: Cow<'static, str>,
        applicability: Applicability,
    },
    MultiPart {
        parts: Vec<(Span, Cow<'static, str>), &'alloc A>,
        message: Cow<'static, str>,
        applicability: Applicability,
    },
}
