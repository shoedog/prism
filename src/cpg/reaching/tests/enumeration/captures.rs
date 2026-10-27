use super::{Case, CaseClass};
use crate::cpg::reaching::binding_table::{select_capture_row, Predicate, Timing};
use crate::cpg::reaching::RdFileStats;
use crate::cpg::{FlowConfidence, FlowDoubt};
use crate::languages::Language;

const CFG_INCOMPLETE: FlowConfidence = FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete);

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
