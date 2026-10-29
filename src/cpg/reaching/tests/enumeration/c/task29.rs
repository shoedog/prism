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

const KILLED_3: FlowConfidence = killed(3);
const UNCERTAIN_2: FlowConfidence = uncertain(2);
const UNCERTAIN_3: FlowConfidence = uncertain(3);
const CFG_INCOMPLETE: FlowConfidence = FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete);

fn check(case: &Case) {
    assert_behavior(case, &super::super::super::parsed(case.src, case.language));
}

#[test]
#[ignore = "chain recovery parked: DECISIONS §P.1 / #530"]
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
#[ignore = "chain recovery parked: DECISIONS §P.1 / #530"]
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
#[ignore = "chain recovery parked: DECISIONS §P.1 / #530"]
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

#[test]
#[ignore = "chain recovery parked: DECISIONS §P.1 / #530"]
fn task29_fix1_c_initializer_controls_preserve_reads_and_writes() {
    let cases = [
        Case {
            id: "c-chain-initializer-read-preserves-outer",
            language: Language::C,
            kind: "declaration",
            variant: None,
            row_variant: None,
            class: CaseClass::Behavioral,
            src: "void f() {\n  int x = source(), y = 0;\n  int a = x, b = 0;\n  sink(x);\n}\n",
            expect: &[("2:x", "4:x", EXACT)],
            expect_counter: Some(("dfg_label_exact", 1)),
        },
        Case {
            id: "c-chain-initializer-assignment-kills-outer",
            language: Language::C,
            kind: "declaration",
            variant: None,
            row_variant: None,
            class: CaseClass::Behavioral,
            src:
                "void f() {\n  int x = source(), y = 0;\n  int a = (x = 0), b = 0;\n  sink(x);\n}\n",
            expect: &[("2:x", "4:x", KILLED_3)],
            expect_counter: Some(("dfg_label_nameonly_killed", 1)),
        },
    ];
    for case in &cases {
        check(case);
    }
}

#[test]
#[ignore = "chain recovery parked: DECISIONS §P.1 / #530"]
fn task29_fix1_c_exclusive_branch_routes_fail_closed() {
    let cases = [
        Case {
            id: "c-chain-exclusive-arm-false-exact",
            language: Language::C,
            kind: "declaration",
            variant: None,
            row_variant: None,
            class: CaseClass::Behavioral,
            src: "void f(int kind) {\n  int x = 0, y = 0;\n  if (kind == 1) {\n    if ((x = source()) == -1) return;\n  } else if (kind == 2) {\n    sink(x);\n  }\n}\n",
            expect: &[("4:x", "6:x", UNCERTAIN_2)],
            expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
        },
        Case {
            id: "c-chain-exclusive-arm-wrong-kill-witness",
            language: Language::C,
            kind: "declaration",
            variant: None,
            row_variant: None,
            class: CaseClass::Behavioral,
            src: "void f(int kind) {\n  int x = 0, y = 0;\n  if (kind == 1) {\n    if ((x = source()) == -1) return;\n  } else if (kind == 2) {\n    if ((x = clean()) == -1) return;\n    sink(x);\n  }\n}\n",
            expect: &[("2:x", "7:x", UNCERTAIN_2), ("4:x", "7:x", UNCERTAIN_2)],
            expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 2)),
        },
        Case {
            id: "c-chain-redis-rdb-save-exclusive-object-arms",
            language: Language::C,
            kind: "declaration",
            variant: None,
            row_variant: None,
            class: CaseClass::Behavioral,
            src: "void f(int kind) {\n  int n = 0, written = 0;\n  if (kind == 1) {\n    if ((n = save()) == -1) return;\n  } else if (kind == 2) {\n    sink(n);\n  }\n}\n",
            expect: &[("4:n", "6:n", UNCERTAIN_2)],
            expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
        },
        Case {
            id: "c-chain-redis-rdb-load-kill-from-selected-arm",
            language: Language::C,
            kind: "declaration",
            variant: None,
            row_variant: None,
            class: CaseClass::Behavioral,
            src: "void f(int kind) {\n  int o = 0, ele = 0;\n  if (kind == 1) {\n    o = make_set();\n  } else if (kind == 2) {\n    o = make_hash();\n    sink(o);\n  }\n}\n",
            expect: &[("2:o", "7:o", UNCERTAIN_2), ("4:o", "7:o", CFG_INCOMPLETE)],
            expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
        },
        Case {
            id: "c-chain-redis-sort-loop-exclusive-option-arms",
            language: Language::C,
            kind: "declaration",
            variant: None,
            row_variant: None,
            class: CaseClass::Behavioral,
            src: "void f(int kind) {\n  int j, left = 0, vectorlen;\n  j = 2;\n  while (ready()) {\n    if (kind == 0) {\n      sink(left);\n    } else if (kind == 1) {\n      j += 2;\n    } else if (kind == 2) {\n      sink(left);\n    } else if (kind == 3) {\n      sink(\n          j);\n    }\n    j++;\n  }\n}\n",
            expect: &[("8:j", "13:j", UNCERTAIN_2)],
            expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
        },
    ];
    for case in &cases {
        check(case);
    }
}

#[test]
fn task29_park_r2_c_probes_match_base_ownership_uncertainty() {
    let cases = [
        Case {
            id: "task29-park-if-else-nobraces-chain-c",
            language: Language::C,
            kind: "declaration",
            variant: None,
            row_variant: None,
            class: CaseClass::Behavioral,
            src: include_str!(
                "../../../../../../tests/fixtures/task29_park/r2_if_else_nobraces_chain_c/main.c"
            ),
            expect: &[("3:x", "7:x", UNCERTAIN_3)],
            expect_counter: None,
        },
        Case {
            id: "task29-park-switch-case-same-chain-c",
            language: Language::C,
            kind: "declaration",
            variant: None,
            row_variant: None,
            class: CaseClass::Behavioral,
            src: include_str!(
                "../../../../../../tests/fixtures/task29_park/r2_switch_case_same_chain_c/main.c"
            ),
            expect: &[("5:x", "8:x", UNCERTAIN_3)],
            expect_counter: None,
        },
        Case {
            id: "task29-park-if-same-line-chain-c",
            language: Language::C,
            kind: "declaration",
            variant: None,
            row_variant: None,
            class: CaseClass::Behavioral,
            src: include_str!(
                "../../../../../../tests/fixtures/task29_park/r2_if_same_line_chain_c/main.c"
            ),
            expect: &[("4:x", "6:x", UNCERTAIN_3)],
            expect_counter: None,
        },
        Case {
            id: "task29-park-conditional-chain-c",
            language: Language::C,
            kind: "declaration",
            variant: None,
            row_variant: None,
            class: CaseClass::Behavioral,
            src: include_str!(
                "../../../../../../tests/fixtures/task29_park/r2_conditional_chain_c/main.c"
            ),
            expect: &[("3:x", "6:x", UNCERTAIN_3)],
            expect_counter: None,
        },
    ];
    for case in &cases {
        check(case);
    }
}
