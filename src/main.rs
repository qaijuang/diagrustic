use std::io::Result;

use diagrustic::applicability::Applicability;
use diagrustic::builder::DiagnosticBuilder;
use diagrustic::level::DiagnosticLevel;
use diagrustic::source_map::SourceMap;
use diagrustic::{EmitDiagnostic, TerminalEmitter};

fn main() -> Result<()> {
    let mut source_map = SourceMap::new();
    let file_id = source_map.add_file("test.rs", "let x: i32 = \"hello\";");
    let type_span = source_map.span(file_id, 7..10); // "i32"
    let expr_span = source_map.span(file_id, 13..20); // "\"hello\""

    let diag = DiagnosticBuilder::new(DiagnosticLevel::Error)
        .set_primary("mismatched types")
        .set_code("E0308")
        .add_span(expr_span)
        .span_label(expr_span, "expected `i32`, found `&str`")
        .span_label_with(type_span, || "due to this type annotation `i32`".to_string())
        .span_suggestion(
            type_span,
            "change the type to `&str`",
            "&str",
            Applicability::MachineApplicable,
        )
        .build();

    let mut emitter = TerminalEmitter::default();
    emitter.emit(&diag, &source_map)
}
