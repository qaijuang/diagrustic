use alloc::alloc::Allocator;
use alloc::vec::Vec;
use core::panic::Location;

use crate::acow::Acow;
use crate::level::DiagnosticLevel;
use crate::span::Span;
use crate::sub_diag::SubDiagnostic;
use crate::suggestion::Suggestion;

pub trait IntoDiagnostic<'alloc, A: Allocator> {
    fn into_diagnostic(self, alloc: &'alloc A) -> Diagnostic<'alloc, A>;
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

    use super::{Diagnostic, IntoDiagnostic};
    use crate::builder::DiagnosticBuilder;
    use crate::level::DiagnosticLevel;
    use crate::source_map::SourceMap;
    use crate::span::Span;

    struct ParseError {
        span: Span,
    }

    impl<'alloc, A: Allocator> IntoDiagnostic<'alloc, A> for ParseError {
        fn into_diagnostic(self, alloc: &'alloc A) -> Diagnostic<'alloc, A> {
            DiagnosticBuilder::new_in(DiagnosticLevel::Error, alloc)
                .set_primary("parse failed")
                .add_span(self.span)
                .span_label(self.span, "unexpected token")
                .build()
        }
    }

    #[test]
    fn error_type_converts_into_diagnostic() {
        let mut source_map = SourceMap::default();
        let file = source_map.add_file("input.rs", "let = 1;");
        let span = source_map.span(file, 4..5);

        let diagnostic = ParseError { span }.into_diagnostic(&Global);

        assert_eq!(diagnostic.level, DiagnosticLevel::Error);
        assert_eq!(diagnostic.primary.as_str(), "parse failed");
        assert_eq!(diagnostic.spans.len(), 1);
        assert_eq!(diagnostic.spans[0], span);
        assert_eq!(diagnostic.children.len(), 1);
        assert_eq!(diagnostic.children[0].message.as_str(), "unexpected token");
        assert_eq!(diagnostic.children[0].spans.len(), 1);
        assert_eq!(diagnostic.children[0].spans[0], span);
    }
}
