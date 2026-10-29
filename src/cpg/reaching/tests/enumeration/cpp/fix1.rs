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

const BASE_SINGLE: Option<super::super::super::binding_table::Predicate> = Some(
    super::super::super::binding_table::Predicate::FieldChildCountIs {
        field: "declarator",
        count: 1,
    },
);

macro_rules! exact_case {
    ($id:literal, $src:literal, $from:literal, $to:literal $(,)?) => {
        Case {
            id: $id,
            language: Language::Cpp,
            kind: "compound_statement",
            variant: None,
            row_variant: None,
            class: CaseClass::Behavioral,
            src: $src,
            expect: &[($from, $to, FlowConfidence::Exact)],
            expect_counter: Some(("dfg_label_exact", 1)),
        }
    };
}

macro_rules! capture_assignment_case {
    ($id:literal, $src:literal $(,)?) => {
        Case {
            id: $id,
            language: Language::Cpp,
            kind: "declaration",
            variant: None,
            row_variant: BASE_SINGLE,
            class: CaseClass::Behavioral,
            src: $src,
            expect: &[
                (
                    "2:x",
                    "4:x",
                    FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
                ),
                ("3:x", "4:x", FlowConfidence::Exact),
            ],
            expect_counter: Some(("dfg_label_nameonly_killed", 1)),
        }
    };
}

const W1_CASES: &[Case] = &[
    exact_case!(
        "cpp-pointer-own-definition-exact",
        "void f() {\n  int *p = source();\n  sink(p);\n}\n",
        "2:p",
        "3:p",
    ),
    exact_case!(
        "cpp-uninitialized-pointer-own-definition-exact",
        "void f() {\n  int *p;\n  sink(p);\n}\n",
        "2:p",
        "3:p",
    ),
    exact_case!(
        "cpp-pointer-to-pointer-own-definition-exact",
        "void f() {\n  int **p = source();\n  sink(p);\n}\n",
        "2:p",
        "3:p",
    ),
    exact_case!(
        "cpp-const-char-pointer-own-definition-exact",
        "void f() {\n  const char *s = source();\n  sink(s);\n}\n",
        "2:s",
        "3:s",
    ),
];

const W2_CASES: &[Case] = &[
    Case {
        id: "cpp-chain-own-definitions-uncertain",
        language: Language::Cpp,
        kind: "declaration",
        variant: None,
        row_variant: CHAIN,
        class: CaseClass::Behavioral,
        src: "void f(int x) {\n  { int a = 1, x = 2;\n    sink(a);\n    sink(x);\n  }\n  sink(x);\n}\n",
        expect: &[
            ("2:a", "3:a", FlowConfidence::Exact),
            ("1:x", "4:x", FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 2 })),
            ("2:x", "4:x", FlowConfidence::Exact),
            ("1:x", "6:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "cpp-multiline-chain-own-definitions-uncertain",
        language: Language::Cpp,
        kind: "declaration",
        variant: None,
        row_variant: CHAIN,
        class: CaseClass::Behavioral,
        src: "void f(int x) {\n  { int a = 1,\n        x = 2;\n    sink(a);\n    sink(x);\n  }\n  sink(x);\n}\n",
        expect: &[
            ("2:a", "4:a", FlowConfidence::Exact),
            ("1:x", "5:x", FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 })),
            ("3:x", "5:x", FlowConfidence::Exact),
            ("1:x", "7:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
];

const W3_CASES: &[Case] = &[
    exact_case!(
        "cpp-alias-assignment-does-not-bind-target",
        "void f() {\n  int x = source();\n  int y;\n  y = x;\n  sink(x);\n}\n",
        "2:x",
        "5:x",
    ),
    exact_case!(
        "cpp-initializer-alias-does-not-bind-target",
        "void f() {\n  int x = source();\n  int y = x;\n  sink(x);\n}\n",
        "2:x",
        "4:x",
    ),
    exact_case!(
        "cpp-conditional-alias-does-not-bind-target",
        "void f() {\n  int x = source();\n  int y;\n  if (ready()) { y = x; }\n  sink(x);\n}\n",
        "2:x",
        "5:x",
    ),
    exact_case!(
        "cpp-basename-alias-does-not-bind-fname",
        "void f() {\n  Name fname = source();\n  Name basename;\n  basename = fname;\n  sink(fname);\n}\n",
        "2:fname",
        "5:fname",
    ),
];

const W4_CASES: &[Case] = &[Case {
    id: "cpp-structured-own-definition-uncertain-top-level",
    language: Language::Cpp,
    kind: "structured_binding_declarator",
    variant: None,
    row_variant: None,
    class: CaseClass::Behavioral,
    src: "void f(Pair pair) {\n  auto [a, x] = pair;\n  sink(x);\n}\n",
    expect: &[(
        "2:x",
        "3:x",
        FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain { construct_line: 2 }),
    )],
    expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
}];

const W5_CASES: &[Case] = &[
    Case {
        id: "cpp-structured-range-header-stops-at-loop-exit",
        language: Language::Cpp,
        kind: "structured_binding_declarator",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "void f(Map map, int x) {\n  for (auto [x, value] : map) {\n    sink(x);\n  }\n  sink(x);\n}\n",
        expect: &[
            (
                "1:x",
                "3:x",
                FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain { construct_line: 2 }),
            ),
            ("1:x", "5:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
    },
    Case {
        id: "cpp-nested-structured-range-does-not-leak",
        language: Language::Cpp,
        kind: "structured_binding_declarator",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "void f(Map map, int x) {\n  for (auto [key, value] : map) {\n    for (auto [x, nested] : value) {\n      sink(x);\n    }\n    sink(x);\n  }\n  sink(x);\n}\n",
        expect: &[
            (
                "1:x",
                "4:x",
                FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain { construct_line: 3 }),
            ),
            ("1:x", "6:x", FlowConfidence::Exact),
            ("1:x", "8:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_exact", 2)),
    },
    Case {
        id: "cpp-for-chain-header-stops-at-loop-exit",
        language: Language::Cpp,
        kind: "declaration",
        variant: None,
        row_variant: CHAIN,
        class: CaseClass::Behavioral,
        src: "void f(int x) {\n  for (int a = 1, x = 2; ready(); step()) {\n    sink(x);\n  }\n  sink(x);\n}\n",
        expect: &[
            (
                "1:x",
                "3:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 2 }),
            ),
            (
                "2:x",
                "3:x",
                FlowConfidence::Exact,
            ),
            ("1:x", "5:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
];

const W6_CASES: &[Case] = &[
    capture_assignment_case!(
        "cpp-capture-init-assignment-keeps-ordinary-kill",
        "void f() {\n  int x = source();\n  auto call = [y = (x = clean())] { sink(y); };\n  sink(x);\n}\n",
    ),
    Case {
        id: "cpp-mixed-capture-init-assignment-keeps-ordinary-kill",
        language: Language::Cpp,
        kind: "declaration",
        variant: None,
        row_variant: BASE_SINGLE,
        class: CaseClass::Behavioral,
        src: "void f() {\n  int x = source();\n  int z = source();\n  auto call = [z, y = (x = clean())] { sink(y); };\n  sink(x);\n}\n",
        expect: &[
            (
                "2:x",
                "5:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 }),
            ),
            ("4:x", "5:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "cpp-reference-init-assignment-keeps-ordinary-kill",
        language: Language::Cpp,
        kind: "declaration",
        variant: None,
        row_variant: BASE_SINGLE,
        class: CaseClass::Behavioral,
        src: "void f() {\n  int x = source();\n  auto call = [&y = (x = clean())] { sink(y); };\n  sink(x);\n}\n",
        expect: &[
            (
                "2:x",
                "4:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
            ),
            ("3:x", "4:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
];

const S3_CASES: &[Case] = &[
    Case {
        id: "cpp-multiple-reference-captures-not-copy-bindings",
        language: Language::Cpp,
        kind: "compound_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "void f() {\n  int x = source();\n  int y = source();\n  auto call = [\n    &x, &y\n  ] {\n    sink(x);\n    sink(y);\n  };\n  sink(x);\n  sink(y);\n}\n",
        expect: &[
            (
                "2:x",
                "7:x",
                FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
            ),
            (
                "3:y",
                "8:y",
                FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
            ),
            ("2:x", "10:x", FlowConfidence::Exact),
            ("3:y", "11:y", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_cfg_incomplete", 2)),
    },
    Case {
        id: "cpp-reference-init-capture-not-copy-binding",
        language: Language::Cpp,
        kind: "compound_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "void f() {\n  int x = source();\n  auto call = [\n    &y = x\n  ] {\n    sink(y);\n  };\n  sink(x);\n}\n",
        expect: &[
            ("2:x", "4:x", FlowConfidence::Exact),
            ("2:x", "8:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_exact", 2)),
    },
];

fn check_all(cases: &[Case]) {
    for case in cases {
        check_case(case);
    }
}

fn park_replay_cases() -> [Case; 4] {
    const ALIAS_UNSTABLE: FlowConfidence = FlowConfidence::NameOnly(FlowDoubt::AliasUnstable);
    [
        Case {
            id: "cpp-park-plain-int-chain-valid",
            language: Language::Cpp,
            kind: "declaration",
            variant: None,
            row_variant: None,
            class: CaseClass::Structural,
            src: "int source(); void sink(int);\nvoid test() {\n int x=source();\n int a=x,b=0;\n sink(x);\n}\n",
            expect: &[(
                "3:x",
                "5:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 }),
            )],
            expect_counter: Some(("dfg_label_nameonly_killed", 1)),
        },
        Case {
            id: "cpp-park-direct-mixed-valid",
            language: Language::Cpp,
            kind: "declaration",
            variant: None,
            row_variant: None,
            class: CaseClass::Structural,
            src: "struct T { T(int); };\nvoid sink(T);\nvoid test(int y,int w) {\n T x(y), z{w};\n sink(x);\n sink(z);\n}\n",
            expect: &[("4:x", "5:x", FlowConfidence::Exact)],
            expect_counter: Some(("dfg_label_exact", 1)),
        },
        Case {
            id: "cpp-park-nested-lambda-2",
            language: Language::Cpp,
            kind: "compound_statement",
            variant: None,
            row_variant: None,
            class: CaseClass::Structural,
            src: "int source(); void sink(int);\nvoid test() {\n int x = source();\n auto g = [h = [x] {\n int a = x;\n sink(x);\n }] { return h; };\n sink(x);\n}\n",
            expect: &[("6:x", "8:x", ALIAS_UNSTABLE), ("6:sink", "8:sink", ALIAS_UNSTABLE)],
            expect_counter: Some(("dfg_label_nameonly_alias_unstable", 2)),
        },
        Case {
            id: "cpp-park-single-shadow-direct",
            language: Language::Cpp,
            kind: "declaration",
            variant: None,
            row_variant: None,
            class: CaseClass::Structural,
            src: "struct T { T(int); T(); };\nvoid sink(T); void sink(int (*)(int));\nvoid test(T x, int y, T *p) {\n {\n T x(y);\n sink(x);\n }\n sink(x);\n}\n",
            expect: &[
                (
                    "3:x",
                    "8:x",
                    FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 5 }),
                ),
                ("5:x", "8:x", FlowConfidence::Exact),
            ],
            expect_counter: Some(("dfg_label_nameonly_killed", 1)),
        },
    ]
}

fn assert_park_replay_case(index: usize) {
    let case = park_replay_cases()[index];
    super::super::case::behavior::assert_behavior(
        &case,
        &super::super::super::parsed(case.src, case.language),
    );
}

#[test]
fn park_replay_plain_int_chain_matches_base_label() {
    assert_park_replay_case(0);
}

#[test]
fn park_replay_direct_mixed_matches_base_label() {
    assert_park_replay_case(1);
}

#[test]
fn park_replay_nested_lambda_matches_base_labels() {
    assert_park_replay_case(2);
}

#[test]
fn park_replay_single_shadow_matches_base_labels() {
    assert_park_replay_case(3);
}

#[test]
fn astra_r1_w1_pointer_declarations_keep_own_identity() {
    check_all(W1_CASES);
    let parsed =
        super::super::super::parsed("void f() { int values[4]; sink(values); }", Language::Cpp);
    let declaration = super::super::case::nodes_of_kind(&parsed, "declaration")[0];
    let row = super::super::super::super::binding_table::select_binding_row(&parsed, declaration)
        .expect("array declaration row");
    let names: Vec<_> = super::super::super::super::transfer::row_introduced_identifiers(
        &parsed,
        declaration,
        row.fields,
    )
    .into_iter()
    .map(|identifier| parsed.node_text(&identifier))
    .collect();
    assert_eq!(names, ["values"]);
}

#[test]
#[ignore = "chain recovery parked: DECISIONS §P.1 / #530"]
fn astra_r1_w2_declarator_chain_own_reads_fail_closed() {
    for case in W2_CASES {
        check_case(case);
    }
}

#[test]
fn astra_r1_w3_alias_twins_do_not_create_lexical_binders() {
    check_all(W3_CASES);
}

#[test]
fn astra_r1_w4_structured_binding_own_reads_fail_closed() {
    check_all(W4_CASES);
}

#[test]
fn astra_r1_w5_structured_range_uncertainty_stops_at_its_owner() {
    check_all(&W5_CASES[..2]);
}

#[test]
#[ignore = "chain recovery parked: DECISIONS §P.1 / #530"]
fn astra_r1_w5_chain_header_uncertainty_stops_at_its_owner() {
    check_case(&W5_CASES[2]);
}

#[test]
fn astra_r1_w6_capture_assignments_preserve_base_labels() {
    check_all(W6_CASES);
}

#[test]
fn astra_r1_s3_reference_captures_are_not_copies() {
    check_all(S3_CASES);
    for case in S3_CASES {
        let parsed = super::super::super::parsed(case.src, Language::Cpp);
        let binder = super::super::case::nodes_of_kind(&parsed, "lambda_capture_specifier")[0];
        let row = super::super::super::super::binding_table::select_binding_row(&parsed, binder)
            .expect("reference capture row");
        assert!(row
            .roles
            .has(super::super::super::super::binding_table::Role::NotBinding));
        assert!(
            super::super::super::super::transfer::row_introduced_identifiers(
                &parsed, binder, row.fields,
            )
            .is_empty()
        );
    }
}
