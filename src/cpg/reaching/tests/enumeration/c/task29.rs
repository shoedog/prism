use super::super::case::{behavior::assert_behavior, Case, CaseClass};
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

fn check(case: &Case) {
    assert_behavior(case, &super::super::super::parsed(case.src, case.language));
}

#[test]
fn task29_c_chain_matches_single_declarations_after_the_declaration() {
    let cases = [
        Case {
            id: "c-chain-per-declarator",
            language: Language::C,
            kind: "declaration",
            variant: None,
            row_variant: None,
            class: CaseClass::Behavioral,
            src: "void f() {\n  int a = source(), x = clean();\n  sink(a);\n  sink(x);\n}\n",
            expect: &[("2:a", "3:a", EXACT), ("2:x", "4:x", EXACT)],
            expect_counter: Some(("dfg_label_exact", 2)),
        },
        Case {
            id: "c-chain-single-declaration-control",
            language: Language::C,
            kind: "declaration",
            variant: None,
            row_variant: None,
            class: CaseClass::Behavioral,
            src: "void f() {\n  int a = source();\n  int x = clean();\n  sink(a);\n  sink(x);\n}\n",
            expect: &[("2:a", "4:a", EXACT), ("3:x", "5:x", EXACT)],
            expect_counter: Some(("dfg_label_exact", 2)),
        },
    ];
    for case in &cases {
        check(case);
    }
}

#[test]
fn task29_c_chain_pointer_array_and_nested_shadow_are_owned_per_declarator() {
    const POINTER_ARRAY: &[(&str, &str, FlowConfidence)] = &[
        ("3:p", "4:p", EXACT),
        ("2:arr", "5:arr", killed(3)),
        ("2:arr", "7:arr", EXACT),
    ];
    const SHADOW: &[(&str, &str, FlowConfidence)] =
        &[("2:x", "3:x", killed(3)), ("2:x", "4:x", EXACT)];
    let cases = [
        Case {
            id: "c-chain-pointer-array",
            language: Language::C,
            kind: "declaration",
            variant: None,
            row_variant: None,
            class: CaseClass::Behavioral,
            src: "void f(int a) {\n  int arr = source();\n  { int *p = &a, arr[2];\n    sink(p);\n    sink(arr);\n  }\n  sink(arr);\n}\n",
            expect: POINTER_ARRAY,
            expect_counter: Some(("dfg_label_exact", 2)),
        },
        Case {
            id: "c-chain-nested-shadow",
            language: Language::C,
            kind: "declaration",
            variant: None,
            row_variant: None,
            class: CaseClass::Behavioral,
            src: "void f() {\n  int x = source();\n  { int a = 1, x = clean(); sink(x); }\n  sink(x);\n}\n",
            expect: SHADOW,
            expect_counter: Some(("dfg_label_nameonly_killed", 1)),
        },
    ];
    for case in &cases {
        check(case);
    }
}

#[test]
fn task29_c_chain_same_line_mate_read_stays_uncertain() {
    const SAME_LINE: &[(&str, &str, FlowConfidence)] =
        &[("2:a", "2:a", uncertain(2)), ("2:a", "3:a", EXACT)];
    let case = Case {
        id: "c-chain-same-line-read-uncertain",
        language: Language::C,
        kind: "declaration",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "void f() {\n  int a = source(), b = a;\n  sink(a);\n}\n",
        expect: SAME_LINE,
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
    };
    check(&case);
}
