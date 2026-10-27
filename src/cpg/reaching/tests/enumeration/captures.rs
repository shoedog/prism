use super::{Case, CaseClass};
use crate::cpg::reaching::binding_table::{
    capture_rows, select_capture_row, CaptureRow, Predicate, Timing,
};
use crate::cpg::reaching::RdFileStats;
use crate::cpg::{FlowConfidence, FlowDoubt};
use crate::languages::Language;

const CFG_INCOMPLETE: FlowConfidence = FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete);

const TSX_IIFE_FUNCTION_INVOKED: Case = Case {
    id: "tsx-iife-function-invoked",
    language: Language::Tsx,
    kind: "function_expression",
    variant: Some("capture"),
    row_variant: Some(Predicate::IsImmediatelyInvoked),
    class: CaseClass::Behavioral,
    src: "function outer(): void {\n  let x: number = source();\n  (function (): void { sink(x); })();\n  return <div>{x}</div>;\n}\n",
    expect: &[("2:x", "3:x", CFG_INCOMPLETE)],
    expect_counter: Some(("dfg_label_capture_immediate", 1)),
};

const TSX_IIFE_ARROW_INVOKED: Case = Case {
    id: "tsx-iife-arrow-invoked",
    language: Language::Tsx,
    kind: "arrow_function",
    variant: Some("capture"),
    row_variant: Some(Predicate::IsImmediatelyInvoked),
    class: CaseClass::Behavioral,
    src: "function outer(): void {\n  let x: number = source();\n  (() => sink(x))();\n  return <div>{x}</div>;\n}\n",
    expect: &[("2:x", "3:x", CFG_INCOMPLETE)],
    expect_counter: Some(("dfg_label_capture_immediate", 1)),
};

pub(super) const CASES: &[Case] = &[
    Case {
        id: "js-iife-parenthesized-invoked",
        language: Language::JavaScript,
        kind: "function_expression",
        variant: Some("capture"),
        row_variant: Some(Predicate::IsImmediatelyInvoked),
        class: CaseClass::Behavioral,
        src: "function outer() {\n  let x = source();\n  (function () { sink(x); })();\n}\n",
        expect: &[("2:x", "3:x", CFG_INCOMPLETE)],
        expect_counter: Some(("dfg_label_capture_immediate", 1)),
    },
    Case {
        id: "js-iife-arrow-invoked",
        language: Language::JavaScript,
        kind: "arrow_function",
        variant: Some("capture"),
        row_variant: Some(Predicate::IsImmediatelyInvoked),
        class: CaseClass::Behavioral,
        src: "function outer() {\n  let x = source();\n  (() => sink(x))();\n}\n",
        expect: &[("2:x", "3:x", CFG_INCOMPLETE)],
        expect_counter: Some(("dfg_label_capture_immediate", 1)),
    },
    Case {
        id: "ts-iife-function-invoked",
        language: Language::TypeScript,
        kind: "function_expression",
        variant: Some("capture"),
        row_variant: Some(Predicate::IsImmediatelyInvoked),
        class: CaseClass::Behavioral,
        src: "function outer(): void {\n  let x: number = source();\n  (function (): void { sink(x); })();\n}\n",
        expect: &[("2:x", "3:x", CFG_INCOMPLETE)],
        expect_counter: Some(("dfg_label_capture_immediate", 1)),
    },
    Case {
        id: "ts-iife-arrow-invoked",
        language: Language::TypeScript,
        kind: "arrow_function",
        variant: Some("capture"),
        row_variant: Some(Predicate::IsImmediatelyInvoked),
        class: CaseClass::Behavioral,
        src: "function outer(): void {\n  let x: number = source();\n  (() => sink(x))();\n}\n",
        expect: &[("2:x", "3:x", CFG_INCOMPLETE)],
        expect_counter: Some(("dfg_label_capture_immediate", 1)),
    },
    Case {
        id: "py-lambda-called-in-place",
        language: Language::Python,
        kind: "lambda",
        variant: Some("capture"),
        row_variant: Some(Predicate::IsImmediatelyInvoked),
        class: CaseClass::Behavioral,
        src: "def outer():\n    x = source()\n    (lambda: sink(x))()\n",
        expect: &[("2:x", "3:x", CFG_INCOMPLETE)],
        expect_counter: Some(("dfg_label_capture_immediate", 1)),
    },
    Case {
        id: "cpp-lambda-invoked-vs-stored",
        language: Language::Cpp,
        kind: "lambda_expression",
        variant: Some("capture"),
        row_variant: Some(Predicate::IsImmediatelyInvoked),
        class: CaseClass::Behavioral,
        src: "void outer() {\n  auto x = source();\n  ([&] { sink(x); })();\n}\n",
        expect: &[("2:x", "3:x", CFG_INCOMPLETE)],
        expect_counter: Some(("dfg_label_capture_immediate", 1)),
    },
    TSX_IIFE_FUNCTION_INVOKED,
    TSX_IIFE_ARROW_INVOKED,
];

fn assert_behavior(case: &Case) {
    let parsed = super::super::super::parsed(case.src, case.language);
    super::behavior::assert_behavior(case, &parsed);
}

#[test]
fn task19_capture_cases() {
    for case in CASES {
        super::check_case(case);
    }
}

fn supplemental_case(
    id: &'static str,
    language: Language,
    src: &'static str,
    expect: &'static [(&'static str, &'static str, FlowConfidence)],
    counter: i64,
) -> Case {
    Case {
        id,
        language,
        kind: "supplemental",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src,
        expect,
        expect_counter: Some(("dfg_label_capture_immediate", counter)),
    }
}

fn assert_selected_timing(src: &str, language: Language, kind: &str, timing: Timing) {
    let parsed = super::super::super::parsed(src, language);
    let nodes = super::nodes_of_kind(&parsed, kind);
    assert_eq!(nodes.len(), 1, "{kind} occurrence count");
    assert_eq!(
        select_capture_row(&parsed, nodes[0]).map(|row| row.timing),
        Some(timing),
        "{kind} timing"
    );
}

fn assert_nested_capture_counter(
    id: &'static str,
    language: Language,
    src: &'static str,
    counter: i64,
) {
    assert_behavior(&supplemental_case(
        id,
        language,
        src,
        &[("2:x", "3:x", CFG_INCOMPLETE)],
        counter,
    ));
}

#[test]
fn js_concise_nested_arrow_uses_inner_deferred_timing() {
    assert_nested_capture_counter(
        "js-concise-nested-arrow",
        Language::JavaScript,
        "function outer() {\n  let x = source();\n  (() => () => sink(x))();\n}\n",
        0,
    );
}

#[test]
fn js_concise_nested_function_expression_uses_inner_deferred_timing() {
    assert_nested_capture_counter(
        "js-concise-nested-function-expression",
        Language::JavaScript,
        "function outer() {\n  let x = source();\n  (() => function () { sink(x); })();\n}\n",
        0,
    );
}

#[test]
fn python_concise_nested_lambda_uses_inner_deferred_timing() {
    assert_nested_capture_counter(
        "py-concise-nested-lambda",
        Language::Python,
        "def outer():\n    x = source()\n    (lambda: lambda: sink(x))()\n",
        0,
    );
}

#[test]
fn js_parenthesized_concise_nested_arrow_stays_deferred() {
    assert_nested_capture_counter(
        "js-parenthesized-concise-nested-arrow",
        Language::JavaScript,
        "function outer() {\n  let x = source();\n  (() => (() => sink(x)))();\n}\n",
        0,
    );
}

#[test]
fn python_parenthesized_concise_nested_lambda_stays_deferred() {
    assert_nested_capture_counter(
        "py-parenthesized-concise-nested-lambda",
        Language::Python,
        "def outer():\n    x = source()\n    (lambda: (lambda: sink(x)))()\n",
        0,
    );
}

#[test]
fn js_block_returned_arrow_stays_deferred() {
    assert_nested_capture_counter(
        "js-block-returned-arrow",
        Language::JavaScript,
        "function outer() {\n  let x = source();\n  (() => { return () => sink(x); })();\n}\n",
        0,
    );
}

#[test]
fn js_concise_nested_arrow_invoked_in_outer_body_counts_once() {
    assert_nested_capture_counter(
        "js-concise-nested-arrow-invoked",
        Language::JavaScript,
        "function outer() {\n  let x = source();\n  (() => (() => sink(x))())();\n}\n",
        1,
    );
}

#[test]
fn tsx_iife_function_expression_is_immediate() {
    super::check_case(&TSX_IIFE_FUNCTION_INVOKED);
}

#[test]
fn tsx_iife_arrow_is_immediate() {
    super::check_case(&TSX_IIFE_ARROW_INVOKED);
}

#[test]
fn tsx_capture_rows_match_typescript_twins_except_language_and_regression() {
    let typescript = capture_rows(Language::TypeScript);
    let tsx = capture_rows(Language::Tsx);
    assert_eq!(tsx.len(), typescript.len(), "TSX capture row count");
    for (ts_row, tsx_row) in typescript.iter().zip(tsx) {
        assert_eq!(
            tsx_row.language,
            Language::Tsx,
            "TSX-owned {}",
            tsx_row.kind
        );
        assert_eq!(
            CaptureRow {
                language: Language::TypeScript,
                regression: ts_row.regression,
                ..*tsx_row
            },
            *ts_row,
            "TSX capture twin {} {:?}",
            tsx_row.kind,
            tsx_row.variant
        );
    }
}

#[test]
fn js_iife_stored_parenthesized_is_deferred() {
    const SRC: &str =
        "function outer() {\n  let x = source();\n  const f = (function () { sink(x); });\n}\n";
    assert_selected_timing(
        SRC,
        Language::JavaScript,
        "function_expression",
        Timing::Deferred,
    );
    assert_behavior(&supplemental_case(
        "js-iife-stored-parenthesized",
        Language::JavaScript,
        SRC,
        &[("2:x", "3:x", CFG_INCOMPLETE)],
        0,
    ));
}

#[test]
fn js_iife_mutation_w5_stays_cfg_incomplete_and_counted() {
    const SRC: &str = "function outer() {\n  let x = source();\n  (() => {\n    x = clean();\n    sink(x);\n  })();\n}\n";
    assert_behavior(&supplemental_case(
        "js-iife-mutation-w5",
        Language::JavaScript,
        SRC,
        &[("2:x", "5:x", CFG_INCOMPLETE)],
        1,
    ));
}

#[test]
fn js_invoked_inside_stored_dedup_counts_innermost_once() {
    const SRC: &str = "function outer() {\n  let x = source();\n  const f = function () {\n    (() => sink(x))();\n  };\n}\n";
    assert_behavior(&supplemental_case(
        "js-invoked-inside-stored-dedup",
        Language::JavaScript,
        SRC,
        &[("2:x", "4:x", CFG_INCOMPLETE)],
        1,
    ));
}

#[test]
fn js_stored_inside_invoked_uses_innermost_deferred_timing() {
    const SRC: &str = "function outer() {\n  let x = source();\n  (function () {\n    const g = () => sink(x);\n  })();\n}\n";
    assert_behavior(&supplemental_case(
        "js-stored-inside-invoked",
        Language::JavaScript,
        SRC,
        &[("2:x", "4:x", CFG_INCOMPLETE)],
        0,
    ));
}

#[test]
fn cpp_stored_lambda_is_deferred() {
    const SRC: &str = "void outer() {\n  auto x = source();\n  auto f = [&] { sink(x); };\n}\n";
    assert_selected_timing(SRC, Language::Cpp, "lambda_expression", Timing::Deferred);
    assert_behavior(&supplemental_case(
        "cpp-lambda-stored",
        Language::Cpp,
        SRC,
        &[("2:x", "3:x", CFG_INCOMPLETE)],
        0,
    ));
}

#[test]
fn js_nonlocal_invocation_shapes_are_not_immediate() {
    const STORED_CALLED: &str = "function outer() {\n  let x = source();\n  const f = function () { sink(x); };\n  f();\n}\n";
    const CALLBACK: &str =
        "function outer() {\n  let x = source();\n  run(function () { sink(x); });\n}\n";
    const CONSTRUCTED: &str =
        "function outer() {\n  let x = source();\n  new (function () { sink(x); })();\n}\n";
    for (id, src) in [
        ("js-stored-then-called-not-immediate", STORED_CALLED),
        ("js-callback-argument-not-immediate", CALLBACK),
        ("js-new-parenthesized-not-immediate", CONSTRUCTED),
    ] {
        assert_selected_timing(
            src,
            Language::JavaScript,
            "function_expression",
            Timing::Deferred,
        );
        assert_behavior(&supplemental_case(
            id,
            Language::JavaScript,
            src,
            &[("2:x", "3:x", CFG_INCOMPLETE)],
            0,
        ));
    }
}

#[test]
fn js_iife_does_not_reclassify_an_ordinary_edge() {
    const SRC: &str = "function outer() {\n  let x = source();\n  let y = source();\n  (function () { sink(x); })();\n  sink(y);\n}\n";
    assert_behavior(&supplemental_case(
        "js-iife-ordinary-edge-preserved",
        Language::JavaScript,
        SRC,
        &[
            ("2:x", "4:x", CFG_INCOMPLETE),
            ("3:y", "5:y", FlowConfidence::Exact),
        ],
        1,
    ));
}

#[test]
fn capture_immediate_stats_merge_is_idempotent() {
    let mut retained = RdFileStats::default();
    retained.record_capture_immediate("outer".to_string(), 1, "2:x→3:x".to_string());
    let mut merged = RdFileStats::default();
    merged.merge(retained.clone());
    merged.merge(retained);
    assert_eq!(merged.capture_immediate_edges, 1);
}
