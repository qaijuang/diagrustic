#![feature(allocator_api)]

use std::alloc::Global;
use std::env;
use std::io::{self, Result};
use std::vec::Vec;

use diagrustic::applicability::Applicability;
use diagrustic::builder::DiagnosticBuilder;
use diagrustic::diagnostic::Diagnostic;
use diagrustic::level::DiagnosticLevel;
use diagrustic::source_map::SourceMap;
use diagrustic::sub_diag::SubDiagnostic;
use diagrustic::suggestion::Suggestion;
use diagrustic::{
    ColorChoice, DiagnosticFormat, EmitDiagnostic, EmitterConfig, IntoAcow, TerminalEmitter,
};

fn main() -> Result<()> {
    let scenario = env::args().nth(1).unwrap();
    match scenario.as_str() {
        "adjacent-multiline-spans" => adjacent_multiline_spans(),
        "combining-width" => combining_width(),
        "cross-line-span" => cross_line_span(),
        "cross-line-suggestion" => cross_line_suggestion(),
        "eof-empty-line" => eof_empty_line(),
        "emoji-width" => emoji_width(),
        "empty-replacement-suggestion" => empty_replacement_suggestion(),
        "far-apart-labels" => far_apart_labels(),
        "failure-note" => failure_note(),
        "filtered-labels" => filtered_labels(),
        "label-only" => label_only(),
        "levels" => levels(),
        "mismatched-types" => mismatched_types(),
        "multi-digit-lines" => multi_digit_lines(),
        "multiple-files" => multiple_files(),
        "multiple-secondary-same-line" => multiple_secondary_same_line(),
        "multiline-primary-span" => multiline_primary_span(),
        "multiline-secondary-span" => multiline_secondary_span(),
        "multipart-suggestion" => multipart_suggestion(),
        "no-code-no-span" => no_code_no_span(),
        "plain-color" => plain_color(),
        "primary-without-label" => primary_without_label(),
        "related-note-block" => related_note_block(),
        "same-line-labels" => same_line_labels(),
        "secondary-labels" => secondary_labels(),
        "span-shapes" => span_shapes(),
        "trailing-subdiagnostics" => trailing_subdiagnostics(),
        "unicode-tabs" => unicode_tabs(),
        "unsupported-json-format" => unsupported_format(DiagnosticFormat::Json),
        "unsupported-short-format" => unsupported_format(DiagnosticFormat::Short),
        _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "unknown diagnostic UI scenario")),
    }
}

fn mismatched_types() -> Result<()> {
    let mut source_map = SourceMap::default();
    let file_id = source_map.add_file("test.rs", "let x: i32 = \"hello\";");
    let type_span = source_map.span(file_id, 7..10);
    let expr_span = source_map.span(file_id, 13..20);

    let diag = DiagnosticBuilder::new(DiagnosticLevel::Error)
        .set_primary("mismatched types")
        .set_code("E0308")
        .add_span(expr_span)
        .span_label(expr_span, "expected `i32`, found `&str`")
        .span_label_with(type_span, || "due to this type annotation `i32`")
        .span_suggestion(
            type_span,
            "change the type to `&str`",
            "&str",
            Applicability::MachineApplicable,
        )
        .build();

    emit(&diag, &source_map)
}

fn levels() -> Result<()> {
    let mut source_map = SourceMap::default();
    let file_id = source_map.add_file("levels.rs", "let signal = 1;");
    let span = source_map.span(file_id, 4..10);

    for (level, message) in [
        (DiagnosticLevel::Error, "error level"),
        (DiagnosticLevel::Warning, "warning level"),
        (DiagnosticLevel::Help, "help level"),
        (DiagnosticLevel::Note, "note level"),
    ] {
        let diag = DiagnosticBuilder::new(level)
            .set_primary(message)
            .add_span(span)
            .span_label(span, "level marker")
            .build();
        emit(&diag, &source_map)?;
    }

    Ok(())
}

fn failure_note() -> Result<()> {
    let source_map = SourceMap::default();
    let diag = DiagnosticBuilder::new(DiagnosticLevel::FailureNote)
        .set_primary("aborting due to 2 previous errors")
        .build();
    emit(&diag, &source_map)
}

fn no_code_no_span() -> Result<()> {
    let source_map = SourceMap::default();
    let diag = DiagnosticBuilder::new(DiagnosticLevel::Warning)
        .set_primary("configuration value is unused")
        .build();
    emit(&diag, &source_map)
}

fn primary_without_label() -> Result<()> {
    let mut source_map = SourceMap::default();
    let file_id = source_map.add_file("primary.rs", "let value = input;");
    let span = source_map.span(file_id, 4..9);
    let diag = DiagnosticBuilder::new(DiagnosticLevel::Error)
        .set_primary("primary span without a label")
        .add_span(span)
        .build();
    emit(&diag, &source_map)
}

fn secondary_labels() -> Result<()> {
    let mut source_map = SourceMap::default();
    let source = "fn call(a: i32) {}\nfn main() { call(a, b); }".to_owned();
    let def_name_start = source.find("call").unwrap();
    let call_start = source.rfind("call").unwrap();
    let extra_start = source.rfind(", b").unwrap();
    let file_id = source_map.add_file("call.rs", source);
    let def_name = source_map.span(file_id, def_name_start..def_name_start + "call".len());
    let call = source_map.span(file_id, call_start..call_start + "call".len());
    let extra_arg = source_map.span(file_id, extra_start + 2..extra_start + 3);
    let removal = source_map.span(file_id, extra_start..extra_start + ", b".len());

    let mut definition =
        SubDiagnostic::new_in(DiagnosticLevel::Note, "function defined here", &Global);
    definition.spans.push(def_name);

    let diag = DiagnosticBuilder::new(DiagnosticLevel::Error)
        .set_primary("this function takes 1 argument but 2 arguments were supplied")
        .add_span(call)
        .span_label(extra_arg, "unexpected argument")
        .add_sub_diagnostic(definition)
        .span_suggestion(removal, "remove the extra argument", "", Applicability::MachineApplicable)
        .build();
    emit(&diag, &source_map)
}

fn related_note_block() -> Result<()> {
    let mut source_map = SourceMap::default();
    let source = "fn call(a: i32) {}\nfn main() { call(a, b); }".to_owned();
    let def_name_start = source.find("call").unwrap();
    let call_start = source.rfind("call").unwrap();
    let extra_start = source.rfind(", b").unwrap();
    let file_id = source_map.add_file("related-note.rs", source);
    let def_name = source_map.span(file_id, def_name_start..def_name_start + "call".len());
    let call = source_map.span(file_id, call_start..call_start + "call".len());
    let extra_arg = source_map.span(file_id, extra_start + 2..extra_start + 3);

    let mut note = SubDiagnostic::new_in(DiagnosticLevel::Note, "function defined here", &Global);
    note.spans.push(def_name);

    let diag = DiagnosticBuilder::new(DiagnosticLevel::Error)
        .set_primary("this function takes 1 argument but 2 arguments were supplied")
        .add_span(call)
        .span_label(extra_arg, "unexpected argument")
        .add_sub_diagnostic(note)
        .build();
    emit(&diag, &source_map)
}

fn span_shapes() -> Result<()> {
    let mut source_map = SourceMap::default();
    let file_id = source_map.add_file("shapes.rs", "abcdefghi");
    let first = source_map.span(file_id, 1..4);
    let overlap = source_map.span(file_id, 3..6);
    let zero_width = source_map.span(file_id, 7..7);

    let diag = DiagnosticBuilder::new(DiagnosticLevel::Error)
        .set_primary("overlapping and zero-width primary spans")
        .add_span(first)
        .add_span(overlap)
        .add_span(zero_width)
        .build();
    emit(&diag, &source_map)
}

fn filtered_labels() -> Result<()> {
    let mut source_map = SourceMap::default();
    let main_file = source_map.add_file("main.rs", "first();\nsecond();");
    let other_file = source_map.add_file("other.rs", "external();");
    let primary = source_map.span(main_file, 0..5);
    let other_line = source_map.span(main_file, 9..15);
    let other_file_span = source_map.span(other_file, 0..8);

    let mut second_line =
        SubDiagnostic::new_in(DiagnosticLevel::Note, "second line label", &Global);
    second_line.spans.push(other_line);
    let mut other_file_label =
        SubDiagnostic::new_in(DiagnosticLevel::Note, "other file label", &Global);
    other_file_label.spans.push(other_file_span);

    let diag = DiagnosticBuilder::new(DiagnosticLevel::Error)
        .set_primary("labels on related lines are rendered")
        .add_span(primary)
        .span_label(primary, "primary line")
        .add_sub_diagnostic(second_line)
        .add_sub_diagnostic(other_file_label)
        .build();
    emit(&diag, &source_map)
}

fn cross_line_span() -> Result<()> {
    let mut source_map = SourceMap::default();
    let file_id = source_map.add_file("cross.rs", "alpha\nbeta");
    let span = source_map.span(file_id, 2..8);
    let diag = DiagnosticBuilder::new(DiagnosticLevel::Error)
        .set_primary("span crosses a line boundary")
        .add_span(span)
        .span_label(span, "cross-line label")
        .build();
    emit(&diag, &source_map)
}

fn cross_line_suggestion() -> Result<()> {
    let mut source_map = SourceMap::default();
    let file_id = source_map.add_file("suggest-cross.rs", "alpha\nbeta");
    let primary = source_map.span(file_id, 0..5);
    let replacement = source_map.span(file_id, 2..8);
    let diag = DiagnosticBuilder::new(DiagnosticLevel::Error)
        .set_primary("cross-line replacement suggestion")
        .add_span(primary)
        .span_label(primary, "replacement starts here")
        .span_suggestion(
            replacement,
            "replace text across the line boundary",
            "gamma",
            Applicability::MaybeIncorrect,
        )
        .build();
    emit(&diag, &source_map)
}

fn empty_replacement_suggestion() -> Result<()> {
    let source = "let value = unused;".to_owned();
    let start = source.find("unused").unwrap();
    let mut source_map = SourceMap::default();
    let file_id = source_map.add_file("remove.rs", source);
    let span = source_map.span(file_id, start..start + "unused".len());
    let diag = DiagnosticBuilder::new(DiagnosticLevel::Error)
        .set_primary("remove unused binding")
        .add_span(span)
        .span_label(span, "remove this")
        .span_suggestion(span, "remove the unused binding", "", Applicability::MachineApplicable)
        .build();
    emit(&diag, &source_map)
}

fn label_only() -> Result<()> {
    let source = "item = value;".to_owned();
    let start = source.find("value").unwrap();
    let mut source_map = SourceMap::default();
    let file_id = source_map.add_file("label-only.rs", source);
    let span = source_map.span(file_id, start..start + "value".len());
    let diag = DiagnosticBuilder::new(DiagnosticLevel::Error)
        .set_primary("label without a primary span")
        .span_label(span, "label supplies the source window")
        .build();
    emit(&diag, &source_map)
}

fn trailing_subdiagnostics() -> Result<()> {
    let source_map = SourceMap::default();
    let mut note = SubDiagnostic::new_in(DiagnosticLevel::Note, "while checking item", &Global);
    note.children.push(SubDiagnostic::new_in(
        DiagnosticLevel::Help,
        "try simplifying the expression",
        &Global,
    ));

    let diag = DiagnosticBuilder::new(DiagnosticLevel::Error)
        .set_primary("trailing subdiagnostics")
        .add_sub_diagnostic(note)
        .build();
    emit(&diag, &source_map)
}

fn multipart_suggestion() -> Result<()> {
    let mut source_map = SourceMap::default();
    let file_id = source_map.add_file("rename.rs", "let mut value = old_name;");
    let span = source_map.span(file_id, 16..24);

    let mut parts = Vec::new_in(&Global);
    parts.push((span, "new_name".into_acow(&Global)));

    let diag = DiagnosticBuilder::new(DiagnosticLevel::Error)
        .set_primary("rename required")
        .add_span(span)
        .span_label(span, "old name")
        .add_suggestion(Suggestion::MultiPart {
            parts,
            message: "rename all related parts".into_acow(&Global),
            applicability: Applicability::MaybeIncorrect,
        })
        .build();
    emit(&diag, &source_map)
}

fn plain_color() -> Result<()> {
    let mut source_map = SourceMap::default();
    let file_id = source_map.add_file("plain.rs", "let number = \"text\";");
    let span = source_map.span(file_id, 13..19);

    let diag = DiagnosticBuilder::new(DiagnosticLevel::Error)
        .set_primary("plain output")
        .add_span(span)
        .span_label(span, "no ANSI styling")
        .build();
    emit_with_config(&diag, &source_map, DiagnosticFormat::Human, ColorChoice::Never)
}

fn multi_digit_lines() -> Result<()> {
    let source = "line 01\nline 02\nline 03\nline 04\nline 05\nline 06\nline 07\nline 08\nline 09\nline 10\nline 11\nline 12 target\n".to_owned();
    let start = source.find("target").unwrap();
    let mut source_map = SourceMap::default();
    let file_id = source_map.add_file("lines.rs", source);
    let span = source_map.span(file_id, start..start + "target".len());

    let diag = DiagnosticBuilder::new(DiagnosticLevel::Warning)
        .set_primary("multi-digit line number")
        .add_span(span)
        .span_label(span, "aligned marker")
        .build();
    emit(&diag, &source_map)
}

fn unicode_tabs() -> Result<()> {
    let source = "let\tπ = \"中\";".to_owned();
    let pi_start = source.find('π').unwrap();
    let string_start = source.find('中').unwrap();
    let mut source_map = SourceMap::default();
    let file_id = source_map.add_file("unicode.rs", source);
    let pi_span = source_map.span(file_id, pi_start..pi_start + 'π'.len_utf8());
    let string_span = source_map.span(file_id, string_start..string_start + '中'.len_utf8());

    let diag = DiagnosticBuilder::new(DiagnosticLevel::Error)
        .set_primary("unicode and tabbed source")
        .add_span(string_span)
        .span_label(string_span, "unicode string")
        .span_label(pi_span, "tabbed unicode identifier")
        .build();
    emit(&diag, &source_map)
}

fn eof_empty_line() -> Result<()> {
    let source = "first\n\n".to_owned();
    let eof = source.len();
    let mut source_map = SourceMap::default();
    let file_id = source_map.add_file("empty.rs", source);
    let span = source_map.span(file_id, eof..eof);

    let diag = DiagnosticBuilder::new(DiagnosticLevel::Error)
        .set_primary("zero-width span at EOF on an empty line")
        .add_span(span)
        .span_label(span, "end of file")
        .build();
    emit(&diag, &source_map)
}

fn same_line_labels() -> Result<()> {
    let mut source_map = SourceMap::default();
    let source = "let total = left + right;".to_owned();
    let left_start = source.find("left").unwrap();
    let right_start = source.find("right").unwrap();
    let op_start = source.find('+').unwrap();
    let file_id = source_map.add_file("same-line.rs", source);
    let primary = source_map.span(file_id, op_start..op_start + 1);
    let left = source_map.span(file_id, left_start..left_start + "left".len());
    let right = source_map.span(file_id, right_start..right_start + "right".len());

    let diag = DiagnosticBuilder::new(DiagnosticLevel::Error)
        .set_primary("cannot add these operands")
        .add_span(primary)
        .span_label(primary, "operator requires matching types")
        .span_label(left, "left operand")
        .span_label(right, "right operand")
        .build();
    emit(&diag, &source_map)
}

fn multiple_secondary_same_line() -> Result<()> {
    let mut source_map = SourceMap::default();
    let source = "call(first, second, third);".to_owned();
    let call_start = source.find("call").unwrap();
    let first_start = source.find("first").unwrap();
    let second_start = source.find("second").unwrap();
    let third_start = source.find("third").unwrap();
    let file_id = source_map.add_file("many-secondary.rs", source);
    let call = source_map.span(file_id, call_start..call_start + "call".len());
    let first = source_map.span(file_id, first_start..first_start + "first".len());
    let second = source_map.span(file_id, second_start..second_start + "second".len());
    let third = source_map.span(file_id, third_start..third_start + "third".len());

    let diag = DiagnosticBuilder::new(DiagnosticLevel::Error)
        .set_primary("too many arguments")
        .add_span(call)
        .span_label(call, "called here")
        .span_label(first, "first argument")
        .span_label(second, "second argument")
        .span_label(third, "third argument")
        .build();
    emit(&diag, &source_map)
}

fn multiline_primary_span() -> Result<()> {
    let mut source_map = SourceMap::default();
    let source = "let value = match input {\n    Some(x) => x,\n    None => return,\n};".to_owned();
    let start = source.find("match").unwrap();
    let end = source.rfind('}').unwrap() + 1;
    let file_id = source_map.add_file("multiline-primary.rs", source);
    let span = source_map.span(file_id, start..end);

    let diag = DiagnosticBuilder::new(DiagnosticLevel::Error)
        .set_primary("match arms have incompatible types")
        .add_span(span)
        .span_label(span, "this match expression has mixed arm types")
        .build();
    emit(&diag, &source_map)
}

fn multiline_secondary_span() -> Result<()> {
    let mut source_map = SourceMap::default();
    let source = "fn build() -> i32 {\n    compute(\n        input,\n    )\n}".to_owned();
    let primary_start = source.find("i32").unwrap();
    let call_start = source.find("compute").unwrap();
    let call_end = source.rfind(')').unwrap() + 1;
    let file_id = source_map.add_file("multiline-secondary.rs", source);
    let return_type = source_map.span(file_id, primary_start..primary_start + "i32".len());
    let call = source_map.span(file_id, call_start..call_end);

    let mut return_type_note = SubDiagnostic::new_in(
        DiagnosticLevel::Note,
        "expected because of this return type",
        &Global,
    );
    return_type_note.spans.push(return_type);

    let diag = DiagnosticBuilder::new(DiagnosticLevel::Error)
        .set_primary("expected `i32`, found call result")
        .add_span(call)
        .span_label(call, "expected `i32`, found call result")
        .add_sub_diagnostic(return_type_note)
        .build();
    emit(&diag, &source_map)
}

fn adjacent_multiline_spans() -> Result<()> {
    let mut source_map = SourceMap::default();
    let source = "let first = call(\n    a,\n);\nlet second = call(\n    b,\n);".to_owned();
    let first_start = source.find("call").unwrap();
    let first_end = source.find(");").unwrap() + 1;
    let second_start = source.rfind("call").unwrap();
    let second_end = source.rfind(");").unwrap() + 1;
    let file_id = source_map.add_file("adjacent-multiline.rs", source);
    let first = source_map.span(file_id, first_start..first_end);
    let second = source_map.span(file_id, second_start..second_end);

    let diag = DiagnosticBuilder::new(DiagnosticLevel::Error)
        .set_primary("adjacent multi-line spans")
        .add_span(first)
        .span_label(first, "first call")
        .span_label(second, "second call")
        .build();
    emit(&diag, &source_map)
}

fn far_apart_labels() -> Result<()> {
    let mut source_map = SourceMap::default();
    let source = "first();\nline2();\nline3();\nline4();\nlast();".to_owned();
    let first_start = source.find("first").unwrap();
    let last_start = source.find("last").unwrap();
    let file_id = source_map.add_file("far.rs", source);
    let first = source_map.span(file_id, first_start..first_start + "first".len());
    let last = source_map.span(file_id, last_start..last_start + "last".len());

    let mut last_note = SubDiagnostic::new_in(DiagnosticLevel::Note, "last label", &Global);
    last_note.spans.push(last);

    let diag = DiagnosticBuilder::new(DiagnosticLevel::Error)
        .set_primary("labels are far apart")
        .add_span(first)
        .span_label(first, "first label")
        .add_sub_diagnostic(last_note)
        .build();
    emit(&diag, &source_map)
}

fn combining_width() -> Result<()> {
    let mut source_map = SourceMap::default();
    let source = "let cafe\u{301}_value = 1;".to_owned();
    let start = source.find("value").unwrap();
    let file_id = source_map.add_file("combining.rs", source);
    let span = source_map.span(file_id, start..start + "value".len());

    let diag = DiagnosticBuilder::new(DiagnosticLevel::Error)
        .set_primary("combining mark before span")
        .add_span(span)
        .span_label(span, "value starts after combining mark")
        .build();
    emit(&diag, &source_map)
}

fn emoji_width() -> Result<()> {
    let mut source_map = SourceMap::default();
    let source =
        "fn main() {\n    let family_👨\u{200d}👩\u{200d}👧\u{200d}👦_value = 1;\n}".to_owned();
    let start = source.find("family").unwrap();
    let end = source.find(" = 1").unwrap();
    let file_id = source_map.add_file("emoji.rs", source);
    let span = source_map.span(file_id, start..end);

    let diag = DiagnosticBuilder::new(DiagnosticLevel::Error)
        .set_primary("identifiers cannot contain emoji: `family_👨👩👧👦_value`")
        .add_span(span)
        .build();
    emit(&diag, &source_map)
}

fn multiple_files() -> Result<()> {
    let mut source_map = SourceMap::default();
    let main = source_map.add_file("main.rs", "mod helper;\nfn main() { helper::run(); }");
    let helper = source_map.add_file("helper.rs", "pub fn run() -> bool { true }");
    let main_start = source_map.source(main).and_then(|source| source.find("helper::run")).unwrap();
    let helper_start = source_map.source(helper).and_then(|source| source.find("run")).unwrap();
    let primary = source_map.span(main, main_start..main_start + "helper::run".len());
    let secondary = source_map.span(helper, helper_start..helper_start + "run".len());

    let mut definition =
        SubDiagnostic::new_in(DiagnosticLevel::Note, "defined in another file", &Global);
    definition.spans.push(secondary);

    let diag = DiagnosticBuilder::new(DiagnosticLevel::Error)
        .set_primary("called function has incompatible return type")
        .add_span(primary)
        .span_label(primary, "called here")
        .add_sub_diagnostic(definition)
        .build();
    emit(&diag, &source_map)
}

fn unsupported_format(format: DiagnosticFormat) -> Result<()> {
    let format_name = match format {
        DiagnosticFormat::Human => "human",
        DiagnosticFormat::Short => "short",
        DiagnosticFormat::Json => "json",
    };
    let source_map = SourceMap::default();
    let diag =
        DiagnosticBuilder::new(DiagnosticLevel::Error).set_primary("unsupported format").build();
    match emit_with_config(&diag, &source_map, format, ColorChoice::Always) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::Unsupported => {
            eprintln!("error: diagnostic format `{format_name}` is not implemented");
            Ok(())
        }
        Err(err) => Err(err),
    }
}

fn emit(diag: &Diagnostic<'_, Global>, source_map: &SourceMap<'_, Global>) -> Result<()> {
    let mut emitter = TerminalEmitter::new_in(io::stderr(), &Global);
    emitter.emit(diag, source_map)
}

fn emit_with_config(
    diag: &Diagnostic<'_, Global>,
    source_map: &SourceMap<'_, Global>,
    format: DiagnosticFormat,
    color: ColorChoice,
) -> Result<()> {
    let config = EmitterConfig { format, color };
    let mut emitter = TerminalEmitter::with_config(io::stderr(), config, &Global);
    emitter.emit(diag, source_map)
}
