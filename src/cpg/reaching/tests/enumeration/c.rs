use super::super::super::binding_table::Predicate;
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

macro_rules! case {
    ($id:literal, $kind:literal, $src:literal, $expect:expr, $counter:expr, $variant:expr, $row_variant:expr) => {
        super::case::Case {
            id: $id,
            language: Language::C,
            kind: $kind,
            variant: $variant,
            row_variant: $row_variant,
            class: super::case::CaseClass::Behavioral,
            src: $src,
            expect: $expect,
            expect_counter: Some($counter),
        }
    };
}

pub(super) const CASES: &[super::case::Case] = &[
    case!(
        "c-block-shadow",
        "compound_statement",
        "void f() {\n  int x = source();\n  { int x = clean(); sink(x); }\n  sink(x);\n}\n",
        &[("2:x", "3:x", killed(3)), ("2:x", "4:x", EXACT)],
        ("dfg_label_nameonly_killed", 1),
        Some("binding"),
        None
    ),
    case!(
        "c-for-init-scope",
        "for_statement",
        "void f() {\n  int x = source();\n  for (int x = clean(); ready(); step()) {\n    sink(x);\n  }\n  sink(x);\n}\n",
        &[("2:x", "4:x", killed(3)), ("2:x", "6:x", EXACT)],
        ("dfg_label_nameonly_killed", 1),
        Some("binding"),
        None
    ),
    case!(
        "c-local-without-initializer",
        "declaration",
        "void f() {\n  int x = source();\n  { int x; sink(x); }\n  sink(x);\n}\n",
        &[("2:x", "3:x", killed(3)), ("2:x", "4:x", EXACT)],
        ("dfg_label_nameonly_killed", 1),
        Some("binding"),
        None
    ),
    case!(
        "c-init-declarator-masks",
        "init_declarator",
        "void f() {\n  int x = source();\n  { int x = clean(); sink(x); }\n  sink(x);\n}\n",
        &[("2:x", "3:x", killed(3)), ("2:x", "4:x", EXACT)],
        ("dfg_label_nameonly_killed", 1),
        Some("binding"),
        None
    ),
    case!(
        "c-pointer-declarator-masks",
        "declaration",
        "void f() {\n  int x = source();\n  { int *x = pointer(); sink(x); }\n  sink(x);\n}\n",
        &[("2:x", "3:x", killed(3)), ("2:x", "4:x", EXACT)],
        ("dfg_label_nameonly_killed", 1),
        None,
        None
    ),
    case!(
        "c-array-declarator-masks",
        "declaration",
        "void f() {\n  int x = source();\n  { int x[4]; sink(x); }\n  sink(x);\n}\n",
        &[("2:x", "3:x", killed(3)), ("2:x", "4:x", EXACT)],
        ("dfg_label_nameonly_killed", 1),
        None,
        None
    ),
    case!(
        "c-declarator-chain-uncertain",
        "declaration",
        "void f() {\n  int a = 1, x = 2;\n  sink(x);\n}\n",
        &[("2:x", "3:x", uncertain(2))],
        ("dfg_label_nameonly_ownership_uncertain", 1),
        Some("binding"),
        Some(Predicate::FieldChildCountAtLeast {
            field: "declarator",
            count: 2,
        })
    ),
    case!(
        "c-assignment-reuse-bypass",
        "assignment_expression",
        "void f() {\n  int x = source();\n  if (ready()) {\n    x = clean();\n    sink(x);\n  }\n  sink(x);\n}\n",
        &[("2:x", "5:x", killed(4)), ("2:x", "7:x", EXACT)],
        ("dfg_label_nameonly_killed", 1),
        Some("binding"),
        None
    ),
    case!(
        "c-switch-case-no-scope",
        "case_statement",
        "void f(int tag) {\n  switch (tag) {\n  case 0:\n    int x = source();\n  case 1:\n    sink(x);\n  }\n}\n",
        &[("4:x", "6:x", EXACT)],
        ("dfg_label_exact", 1),
        Some("binding"),
        None
    ),
];
pub(super) const CURATED: &[(&str, &str)] = &[
    ("compound_statement", "c-block-shadow"),
    ("for_statement", "c-for-init-scope"),
    ("declaration", "c-local-without-initializer"),
    ("init_declarator", "c-init-declarator-masks"),
    ("assignment_expression", "c-assignment-reuse-bypass"),
    ("case_statement", "c-switch-case-no-scope"),
    ("parameter_list", "e0a-x-c-parameter_list"),
    ("parameter_declaration", "e0a-x-c-parameter_declaration"),
];

pub(super) fn source_for_kind(kind: &str) -> &'static str {
    match kind {
        "abstract_function_declarator" => "void f(int (int));",
        "abstract_parenthesized_declarator" | "abstract_pointer_declarator" => {
            "void f(int (*)(int));"
        }
        "attributed_declarator" => "int value [[deprecated]];",
        "declaration_list" => "extern \"C\" { int value; }",
        _ => super::case::source(crate::languages::Language::C),
    }
}

#[test]
fn task22_c_cases() {
    for case in CASES {
        super::case::check_case(case);
    }
}

#[test]
fn c_declaration_rows_own_only_their_introduced_identifiers() {
    use super::super::super::binding_table::{select_binding_row, DeclarationKind, Role};
    use super::super::super::transfer::row_introduced_identifiers;

    let parsed = super::super::parsed(
        "void f() {\n  int owned = ({ nested = 1; nested; });\n  { int scoped = 0; }\n  int *pointer = &owned;\n  int array[4];\n}\n",
        Language::C,
    );
    for (marker, expected) in [
        ("owned =", "owned"),
        ("scoped =", "scoped"),
        ("pointer =", "pointer"),
        ("array[", "array"),
    ] {
        let node = super::case::nodes_of_kind(&parsed, "declaration")
            .into_iter()
            .find(|node| parsed.node_text(node).contains(marker))
            .unwrap_or_else(|| panic!("missing declaration containing {marker}"));
        let row = select_binding_row(&parsed, node).expect("declaration row");
        let actual: Vec<_> = row_introduced_identifiers(&parsed, node, row.fields)
            .into_iter()
            .map(|identifier| parsed.node_text(&identifier))
            .collect();
        assert_eq!(actual, [expected]);
    }

    let assignment = super::case::nodes_of_kind(&parsed, "assignment_expression")[0];
    let assignment_row = select_binding_row(&parsed, assignment).expect("assignment row");
    assert_eq!(assignment_row.declaration, Some(DeclarationKind::Reuse));
    let assigned: Vec<_> = row_introduced_identifiers(&parsed, assignment, assignment_row.fields)
        .into_iter()
        .map(|identifier| parsed.node_text(&identifier))
        .collect();
    assert_eq!(assigned, ["nested"]);

    for compound in super::case::nodes_of_kind(&parsed, "compound_statement") {
        let row = select_binding_row(&parsed, compound).expect("compound scope row");
        assert!(row.roles.has(Role::Scope));
        assert_eq!(row.declaration, None);
        assert!(row_introduced_identifiers(&parsed, compound, row.fields).is_empty());
    }
}
