use alloc::alloc::Allocator;
use alloc::vec::Vec;
use core::panic::Location;

use crate::acow::Acow;
use crate::context::DiagnosticContext;
use crate::level::DiagnosticLevel;
use crate::span::Span;
use crate::sub_diag::SubDiagnostic;
use crate::suggestion::Suggestion;

pub trait IntoDiagnostic<'alloc, A: Allocator> {
    fn into_diagnostic(self, cx: DiagnosticContext<'_, 'alloc, A>) -> Diagnostic<'alloc, A>;
}

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

#[cfg(test)]
mod tests {
    use alloc::alloc::{Allocator, Global};
    use core::ops::Range;

    use super::{Diagnostic, IntoDiagnostic};
    use crate::context::DiagnosticContext;
    use crate::level::DiagnosticLevel;
    use crate::source_map::SourceMap;

    struct ParseError {
        span: Range<usize>,
    }

    impl<'alloc, A: Allocator> IntoDiagnostic<'alloc, A> for ParseError {
        fn into_diagnostic(self, cx: DiagnosticContext<'_, 'alloc, A>) -> Diagnostic<'alloc, A> {
            let span = cx.span(self.span);

            cx.builder(DiagnosticLevel::Error)
                .set_primary("parse failed")
                .add_span(span)
                .span_label(span, "unexpected token")
                .build()
        }
    }

    #[test]
    fn error_type_converts_into_diagnostic() {
        let mut source_map = SourceMap::default();
        let source = source_map.add_source("input.rs", "let = 1;");
        let cx = DiagnosticContext::new(source, &Global);

        let diagnostic = ParseError { span: 4..5 }.into_diagnostic(cx);

        assert_eq!(diagnostic.level, DiagnosticLevel::Error);
        assert_eq!(diagnostic.primary.as_str(), "parse failed");
        assert_eq!(diagnostic.spans.len(), 1);
        assert_eq!(diagnostic.spans[0].start(), 4);
        assert_eq!(diagnostic.spans[0].end(), 5);
        assert_eq!(diagnostic.children.len(), 1);
        assert_eq!(diagnostic.children[0].message.as_str(), "unexpected token");
        assert_eq!(diagnostic.children[0].spans.len(), 1);
        assert_eq!(diagnostic.children[0].spans[0].start(), 4);
    }
}
