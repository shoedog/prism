use crate::cpg::{FlowConfidence, FlowDoubt};
use crate::languages::Language;

mod fix1;
mod fix2;

pub(super) const CASES: &[super::case::Case] = &[
    super::case::Case {
        id: "cpp-block-shadow",
        language: Language::Cpp,
        kind: "declaration",
        variant: Some("binding"),
        row_variant: Some(super::super::super::binding_table::Predicate::FieldChildCountIs {
            field: "declarator",
            count: 1,
        }),
        class: super::case::CaseClass::Behavioral,
        src: "void f() {\n  int x = source();\n  {\n    int x = clean();\n    sink(x);\n  }\n  sink(x);\n}\n",
        expect: &[
            (
                "2:x",
                "5:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 }),
            ),
            ("2:x", "7:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "cpp-range-for-masks",
        language: Language::Cpp,
        kind: "for_range_loop",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "void f(Values values) {\n  int x = source();\n  for (auto x : values) { sink(x); }\n  sink(x);\n}\n",
        expect: &[
            (
                "2:x",
                "3:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
            ),
            ("2:x", "4:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "cpp-range-for-reference",
        language: Language::Cpp,
        kind: "for_range_loop",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Structural,
        src: "void f(Values values) {\n  int x = source();\n  for (auto &x : values) { sink(x); }\n  sink(x);\n}\n",
        expect: &[
            (
                "2:x",
                "3:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
            ),
            ("2:x", "4:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "cpp-catch-param-masks",
        language: Language::Cpp,
        kind: "catch_clause",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "void f() {\n  int x = source();\n  try { work(); } catch (const Error &x) { sink(x); }\n  sink(x);\n}\n",
        expect: &[
            (
                "2:x",
                "3:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
            ),
            ("2:x", "4:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "cpp-for-init-scope",
        language: Language::Cpp,
        kind: "for_statement",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "void f() {\n  int x = source();\n  for (int x = clean(); ready(x); step(x)) { sink(x); }\n  sink(x);\n}\n",
        expect: &[
            (
                "2:x",
                "3:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
            ),
            ("2:x", "4:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "cpp-declarator-chain-uncertain",
        language: Language::Cpp,
        kind: "declaration",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "void f() {\n  int x = clean(),\n      y = x;\n  sink(x);\n}\n",
        expect: &[
            (
                "2:x",
                "3:x",
                FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain { construct_line: 2 }),
            ),
            (
                "2:x",
                "4:x",
                FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain { construct_line: 2 }),
            ),
        ],
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 2)),
    },
    super::case::Case {
        id: "cpp-declarator-chain-uncertain-behavior",
        language: Language::Cpp,
        kind: "compound_statement",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Structural,
        src: "void f() {\n  int x = source();\n  {\n    int x = clean(),\n        y = x;\n    sink(x);\n  }\n  sink(x);\n}\n",
        expect: &[
            (
                "2:x",
                "5:x",
                FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain { construct_line: 4 }),
            ),
            (
                "2:x",
                "6:x",
                FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain { construct_line: 4 }),
            ),
            ("2:x", "8:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 2)),
    },
    super::case::Case {
        id: "cpp-if-init-declaration",
        language: Language::Cpp,
        kind: "condition_clause",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "void f() {\n  int x = source();\n  if (int x = clean(); ready(x)) { sink(x); } else { sink(x); }\n  sink(x);\n}\n",
        expect: &[
            (
                "2:x",
                "3:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
            ),
            ("2:x", "4:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "cpp-lambda-body-block-scope",
        language: Language::Cpp,
        kind: "compound_statement",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "void f() {\n  int x = source();\n  auto call = [&] {\n    int x = clean();\n    sink(x);\n  };\n  sink(x);\n}\n",
        expect: &[
            (
                "2:x",
                "5:x",
                FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
            ),
            ("2:x", "7:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_cfg_incomplete", 1)),
    },
    super::case::Case {
        id: "cpp-structured-binding-uncertain-inside",
        language: Language::Cpp,
        kind: "structured_binding_declarator",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "void f(Pair pair) {\n  int x = source();\n  {\n    auto [x, y] = pair;\n    sink(x);\n  }\n  sink(x);\n}\n",
        expect: &[
            (
                "2:x",
                "5:x",
                FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain { construct_line: 4 }),
            ),
            (
                "4:x",
                "5:x",
                FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain { construct_line: 4 }),
            ),
            ("2:x", "7:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 2)),
    },
    super::case::Case {
        id: "cpp-structured-binding-outside-preserved",
        language: Language::Cpp,
        kind: "compound_statement",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "void f(Pair pair) {\n  int x = source();\n  {\n    auto [x, y] = pair;\n    sink(x);\n  }\n  sink(x);\n}\n",
        expect: &[("2:x", "7:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 1)),
    },
    super::case::Case {
        id: "cpp-structured-binding-no-shadow-exact",
        language: Language::Cpp,
        kind: "compound_statement",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "void f(Pair pair) {\n  int z = source();\n  {\n    auto [x, y] = pair;\n    sink(z);\n  }\n}\n",
        expect: &[("2:z", "5:z", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 1)),
    },
    super::case::Case {
        id: "cpp-capture-specifier-multiline",
        language: Language::Cpp,
        kind: "lambda_capture_specifier",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "void f() {\n  int x = source();\n  auto call = [\n    x\n  ] {\n    sink(x);\n  };\n}\n",
        expect: &[
            ("2:x", "4:x", FlowConfidence::Exact),
            (
                "2:x",
                "6:x",
                FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
            ),
        ],
        expect_counter: Some(("dfg_label_nameonly_cfg_incomplete", 1)),
    },
    super::case::Case {
        id: "cpp-capture-specifier-same-line",
        language: Language::Cpp,
        kind: "compound_statement",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "void f() {\n  int x = source();\n  auto call = [x] { sink(x); };\n}\n",
        expect: &[(
            "2:x",
            "3:x",
            FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
        )],
        expect_counter: Some(("dfg_label_nameonly_cfg_incomplete", 1)),
    },
    super::case::Case {
        id: "cpp-capture-not-a-mask",
        language: Language::Cpp,
        kind: "compound_statement",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "void f() {\n  int x = source();\n  auto call = [x] { sink(x); };\n  sink(x);\n}\n",
        expect: &[("2:x", "4:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 1)),
    },
    super::case::Case {
        id: "cpp-capture-init-copy",
        language: Language::Cpp,
        kind: "compound_statement",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "void f() {\n  int x = source();\n  auto call = [\n    y = x\n  ] {\n    sink(y);\n  };\n}\n",
        expect: &[
            ("2:x", "4:x", FlowConfidence::Exact),
            (
                "4:y",
                "6:y",
                FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
            ),
        ],
        expect_counter: Some(("dfg_label_nameonly_cfg_incomplete", 1)),
    },
    super::case::Case {
        id: "cpp-default-capture-not-binding",
        language: Language::Cpp,
        kind: "lambda_default_capture",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "void f() {\n  int z = source();\n  auto copy = [=] { work(); };\n  auto reference = [&] { work(); };\n  sink(z);\n}\n",
        expect: &[("2:z", "5:z", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 1)),
    },
    super::case::Case {
        id: "cpp-capture-by-reference",
        language: Language::Cpp,
        kind: "lambda_capture_specifier",
        variant: Some("binding"),
        row_variant: Some(
            super::super::super::binding_table::Predicate::CppReferenceCaptureOnly,
        ),
        class: super::case::CaseClass::Behavioral,
        src: "void f() {\n  int x = source();\n  auto call = [&x] { sink(x); };\n  sink(x);\n}\n",
        expect: &[("2:x", "4:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 1)),
    },
];
const TASK23_CASE_COUNT: usize = 9;
const TASK24_CASE_END: usize = 12;
pub(super) const CURATED: &[(&str, &str)] = &[
    ("parameter_list", "e0a-x-cpp-parameter_list"),
    ("parameter_declaration", "e0a-x-cpp-parameter_declaration"),
];

#[test]
fn cpp_task23_cases() {
    for case in &CASES[..TASK23_CASE_COUNT] {
        super::case::check_case(case);
    }
}

#[test]
fn cpp_task24_cases() {
    for case in &CASES[TASK23_CASE_COUNT..TASK24_CASE_END] {
        super::case::check_case(case);
    }
}

#[test]
fn cpp_task25_cases() {
    for case in &CASES[TASK24_CASE_END..] {
        super::case::check_case(case);
    }
}

#[test]
fn cpp_task23_declaration_rows_extract_only_their_declarators() {
    let parsed = super::super::parsed(CASES[0].src, Language::Cpp);
    let declarations = super::case::nodes_of_kind(&parsed, "declaration");
    let inner = declarations
        .iter()
        .find(|node| node.start_position().row + 1 == 4)
        .expect("inner declaration");
    let row = super::super::super::binding_table::select_binding_row(&parsed, *inner)
        .expect("selected declaration row");
    let names: Vec<_> =
        super::super::super::transfer::row_introduced_identifiers(&parsed, *inner, row.fields)
            .into_iter()
            .map(|node| parsed.node_text(&node).to_string())
            .collect();
    assert_eq!(names, ["x"]);

    let chain_case = CASES
        .iter()
        .find(|case| case.id == "cpp-declarator-chain-uncertain-behavior")
        .unwrap();
    let chain = super::super::parsed(chain_case.src, Language::Cpp);
    let uncertain = super::super::super::grammar_lint::uncertain_binding_nodes(&chain, "x");
    let lines: Vec<_> = uncertain
        .iter()
        .map(|node| node.start_position().row + 1)
        .collect();
    assert_eq!(lines, [4]);
}

#[test]
fn cpp_task24_structured_row_extracts_each_identifier() {
    let parsed = super::super::parsed(CASES[TASK23_CASE_COUNT].src, Language::Cpp);
    let binder = super::case::nodes_of_kind(&parsed, "structured_binding_declarator")[0];
    let row = super::super::super::binding_table::select_binding_row(&parsed, binder).unwrap();
    let names: Vec<_> =
        super::super::super::transfer::row_introduced_identifiers(&parsed, binder, row.fields)
            .into_iter()
            .map(|node| parsed.node_text(&node).to_string())
            .collect();
    assert_eq!(names, ["x", "y"]);
}

#[test]
fn cpp_task25_capture_rows_extract_only_copies() {
    let names = |case: &super::case::Case| {
        let parsed = super::super::parsed(case.src, Language::Cpp);
        let binder = super::case::nodes_of_kind(&parsed, "lambda_capture_specifier")[0];
        let row = super::super::super::binding_table::select_binding_row(&parsed, binder).unwrap();
        let names: Vec<_> =
            super::super::super::transfer::row_introduced_identifiers(&parsed, binder, row.fields)
                .into_iter()
                .map(|node| parsed.node_text(&node).to_string())
                .collect();
        (row, names)
    };
    let (copy, copy_names) = names(&CASES[TASK24_CASE_END]);
    assert_eq!(
        copy.declaration,
        Some(super::super::super::binding_table::DeclarationKind::CaptureCopy)
    );
    assert_eq!(copy_names, ["x"]);
    assert_eq!(names(&CASES[TASK24_CASE_END + 3]).1, ["y"]);
    let (reference, reference_names) = names(&CASES[TASK24_CASE_END + 5]);
    assert!(reference
        .roles
        .has(super::super::super::binding_table::Role::NotBinding));
    assert!(reference_names.is_empty());
}

pub(super) fn source_for_kind(kind: &str) -> &'static str {
    match kind {
        "abstract_array_declarator" => "void f(int []);",
        "abstract_parenthesized_declarator"
        | "abstract_pointer_declarator"
        | "abstract_reference_declarator" => "void f(int (*)(int), int (&)[2]);",
        "attribute_declaration" => "[[deprecated]];",
        "attributed_declarator" => "int value [[deprecated]];",
        "declaration_list" => "extern \"C\" { int value; }",
        "delete_expression" => "void f(int *value) { delete value; }",
        "optional_type_parameter_declaration" => "template <typename T = int> struct C {};",
        "parenthesized_declarator" => "int (function)(int);",
        "pointer_type_declarator" => "struct C {}; using Pointer = int C::*;",
        "template_function" => "void f() { auto pointer = function<int>; }",
        _ => super::case::source(crate::languages::Language::Cpp),
    }
}
