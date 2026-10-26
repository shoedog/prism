use super::super::super::case::{Case, CaseClass};
use crate::cpg::reaching::binding_table::Predicate;
use crate::cpg::{FlowConfidence, FlowDoubt};
use crate::languages::Language;

const EXACT: FlowConfidence = FlowConfidence::Exact;

const fn killed(line: u32) -> FlowConfidence {
    FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: line })
}

const fn uncertain(line: u32) -> FlowConfidence {
    FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain {
        construct_line: line,
    })
}

macro_rules! check {
    ($id:literal, $kind:literal, $row_variant:expr, $src:literal, $expect:expr, $counter:expr $(,)?) => {{
        const EXPECT: &[(&str, &str, FlowConfidence)] = $expect;
        super::super::super::case::check_case(&Case {
            id: $id,
            language: Language::JavaScript,
            kind: $kind,
            variant: None,
            row_variant: $row_variant,
            class: CaseClass::Behavioral,
            src: $src,
            expect: EXPECT,
            expect_counter: Some($counter),
        });
    }};
}

fn assert_pattern_is_catch_owned(src: &str, kind: &str) {
    use crate::cpg::reaching::binding_table::select_binding_row;

    let parsed = super::super::super::super::parsed(src, Language::JavaScript);
    let pattern = super::super::super::case::nodes_of_kind(&parsed, kind)[0];
    let row = select_binding_row(&parsed, pattern).expect("catch pattern row");
    assert_eq!(
        row.variant, None,
        "catch parameter pattern must be residual"
    );
}

#[test]
fn catch_pattern_in_arrow_declarator_stops_at_catch() {
    check!(
        "r1-catch-pattern-arrow-declarator",
        "catch_clause",
        None,
        "const h = async () => {\n  let code = source();\n  try { risky(); } catch ({code}) { sink(code); }\n  sink(code);\n};\n",
        &[("2:code", "3:code", killed(3)), ("2:code", "4:code", EXACT)],
        ("dfg_label_nameonly_killed", 1),
    );
    check!(
        "r1-catch-pattern-function-declaration-control",
        "catch_clause",
        None,
        "function f() {\n  let message = source();\n  try { risky(); } catch ({message}) { sink(message); }\n  sink(message);\n}\n",
        &[("2:message", "3:message", killed(3)), ("2:message", "4:message", EXACT)],
        ("dfg_label_nameonly_killed", 1),
    );
    check!(
        "r1-plain-catch-control",
        "catch_clause",
        None,
        "const h = function () {\n  let e = source();\n  try { risky(); } catch (e) { sink(e); }\n  sink(e);\n};\n",
        &[("2:e", "3:e", killed(3)), ("2:e", "4:e", EXACT)],
        ("dfg_label_nameonly_killed", 1),
    );
    assert_pattern_is_catch_owned(
        "const h = async () => {\n  let code = source();\n  try { risky(); } catch ({code}) { sink(code); }\n  sink(code);\n};\n",
        "object_pattern",
    );
    assert_pattern_is_catch_owned(
        "function f() {\n  let message = source();\n  try { risky(); } catch ({message}) { sink(message); }\n  sink(message);\n}\n",
        "object_pattern",
    );
}

#[test]
fn catch_pattern_in_callback_declarator_stops_at_catch() {
    assert_pattern_is_catch_owned(
        "function f() {\n  const p = run(function () {\n    let e = source();\n    try { risky(); } catch ([e]) { sink(e); }\n    sink(e);\n  });\n}\n",
        "array_pattern",
    );
    assert_pattern_is_catch_owned(
        "function f() {\n  run(function () {\n    let e = source();\n    try { risky(); } catch ([e]) { sink(e); }\n    sink(e);\n  });\n}\n",
        "array_pattern",
    );
}

#[test]
fn switch_destructure_uses_switch_scope() {
    check!(
        "r1-switch-destructure-scope",
        "object_pattern",
        Some(Predicate::JavaScriptBindingPattern),
        "function f(v, o) {\n  let x = source();\n  switch (v) { case 1: const {x} = o; sink(x); break; }\n  sink(x);\n}\n",
        &[("2:x", "3:x", uncertain(3)), ("2:x", "4:x", EXACT)],
        ("dfg_label_nameonly_ownership_uncertain", 1),
    );
    check!(
        "r1-nested-block-destructure-control",
        "object_pattern",
        Some(Predicate::JavaScriptBindingPattern),
        "function f(o) {\n  let x = source();\n  {\n    { const {x} = o; sink(x); }\n    sink(x);\n  }\n  sink(x);\n}\n",
        &[
            ("2:x", "4:x", uncertain(4)),
            ("2:x", "5:x", EXACT),
            ("2:x", "7:x", EXACT),
        ],
        ("dfg_label_nameonly_ownership_uncertain", 1),
    );
}

#[test]
fn for_header_destructure_uses_for_scope() {
    check!(
        "r1-for-header-destructure-scope",
        "array_pattern",
        Some(Predicate::JavaScriptBindingPattern),
        "function f(o) {\n  let i = source();\n  for (let [i, j] = o; c; step()) { sink(i); }\n  sink(i);\n}\n",
        &[("2:i", "3:i", uncertain(3)), ("2:i", "4:i", EXACT)],
        ("dfg_label_nameonly_ownership_uncertain", 1),
    );
}
