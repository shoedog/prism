use super::super::case::{check_case, Case, CaseClass};
use crate::cpg::{FlowConfidence, FlowDoubt};
use crate::languages::Language;

const CHAIN: Option<super::super::super::binding_table::Predicate> = Some(
    super::super::super::binding_table::Predicate::FieldChildCountAtLeastExcludingKinds {
        field: "declarator",
        count: 2,
        excluded: &["function_declarator"],
    },
);

const F1_CASES: &[Case] = &[
    Case {
        id: "cpp-chain-subsequent-definition-uncertain",
        language: Language::Cpp,
        kind: "declaration",
        variant: None,
        row_variant: CHAIN,
        class: CaseClass::Behavioral,
        src: "void f() {\n  int x = source(), y = 0;\n  x = clean();\n  sink(x);\n}\n",
        expect: &[
            ("2:x", "4:x", FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 })),
            ("3:x", "4:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "cpp-multiline-chain-subsequent-definition-uncertain",
        language: Language::Cpp,
        kind: "declaration",
        variant: None,
        row_variant: CHAIN,
        class: CaseClass::Behavioral,
        src: "void f() {\n  int x = source(),\n      y = 0;\n  x = clean();\n  sink(x);\n}\n",
        expect: &[
            ("2:x", "5:x", FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 })),
            ("4:x", "5:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "cpp-structured-subsequent-definition-uncertain",
        language: Language::Cpp,
        kind: "structured_binding_declarator",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "void f(Pair pair) {\n  auto [x, y] = pair;\n  x = clean();\n  sink(x);\n}\n",
        expect: &[(
            "3:x",
            "4:x",
            FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain { construct_line: 2 }),
        )],
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
    },
    Case {
        id: "cpp-uncertain-owner-preserves-nested-shadow-and-exit",
        language: Language::Cpp,
        kind: "declaration",
        variant: None,
        row_variant: CHAIN,
        class: CaseClass::Behavioral,
        src: "void f() {\n  int x = source(), y = 0;\n  x = clean();\n  {\n    int x = fresh();\n    sink(x);\n  }\n  sink(x);\n}\n",
        expect: &[
            (
                "3:x",
                "6:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 5 }),
            ),
            ("5:x", "6:x", FlowConfidence::Exact),
            (
                "3:x",
                "8:x",
                FlowConfidence::Exact,
            ),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "cpp-uncertain-owner-stops-at-block-exit",
        language: Language::Cpp,
        kind: "declaration",
        variant: None,
        row_variant: CHAIN,
        class: CaseClass::Behavioral,
        src: "void f(int x) {\n  {\n    int x = source(), y = 0;\n    x = clean();\n    sink(x);\n  }\n  sink(x);\n}\n",
        expect: &[
            (
                "3:x",
                "5:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 }),
            ),
            (
                "4:x",
                "5:x",
                FlowConfidence::Exact,
            ),
            ("1:x", "7:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
];

const F2_CASES: &[Case] = &[
    Case {
        id: "cpp-reference-capture-shadow-declaration-is-not-alias-derived",
        language: Language::Cpp,
        kind: "declaration",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "void f() {\n  int x = source();\n  auto g = [&x] {\n    { int x = clean();\n      sink(x);\n    }\n  };\n  sink(x);\n}\n",
        expect: &[
            ("2:x", "8:x", FlowConfidence::Exact),
            (
                "4:x",
                "8:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 6 }),
            ),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "cpp-copy-capture-shadow-declaration-control",
        language: Language::Cpp,
        kind: "declaration",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "void f() {\n  int x = source();\n  auto g = [x] {\n    { int x = clean();\n      sink(x);\n    }\n  };\n  sink(x);\n}\n",
        expect: &[
            ("2:x", "8:x", FlowConfidence::Exact),
            (
                "4:x",
                "8:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 6 }),
            ),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
];

fn check_all(cases: &[Case]) {
    for case in cases {
        check_case(case);
    }
}

#[test]
#[ignore = "chain recovery parked: DECISIONS §P.1 / #530"]
fn astra_r2_f1_uncertainty_covers_subsequent_definitions_in_owner() {
    for case in F1_CASES {
        check_case(case);
    }
}

#[test]
#[ignore = "chain recovery parked: DECISIONS §P.1 / #530"]
fn astra_r2_f2_reference_capture_normalization_excludes_declarations() {
    check_all(F2_CASES);
}
