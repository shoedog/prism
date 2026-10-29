use super::super::super::binding_table::Predicate;
use crate::cpg::{FlowConfidence, FlowDoubt};
use crate::languages::Language;

mod task29;

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
        "void f() {\n  int x = source(), seed = 0;\n  {\n    int a = 1, x = 2;\n    sink(x);\n  }\n  sink(x);\n}\n",
        &[("2:x", "5:x", killed(4)), ("2:x", "7:x", EXACT)],
        ("dfg_label_nameonly_killed", 1),
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

#[test]
fn fix1_w1_declarator_identity() {
    const UNINITIALIZED: &[(&str, &str, FlowConfidence)] = &[
        ("2:buf", "3:buf", EXACT),
        ("2:buf", "4:buf", killed(3)),
        ("3:buf", "4:buf", EXACT),
    ];
    let cases = [
        case!(
            "c-fix1-pointer-definition",
            "declaration",
            "void f() {\n  int *p = source();\n  sink(p);\n}\n",
            &[("2:p", "3:p", EXACT)],
            ("dfg_label_exact", 1),
            None,
            None
        ),
        case!(
            "c-fix1-uninitialized-pointer",
            "declaration",
            "void f() {\n  char *buf;\n  buf = source();\n  sink(buf);\n}\n",
            UNINITIALIZED,
            ("dfg_label_exact", 2),
            None,
            None
        ),
        case!(
            "c-fix1-function-pointer-definition",
            "declaration",
            "void f() {\n  int (*fp)(int) = source();\n  sink(fp);\n}\n",
            &[("2:fp", "3:fp", EXACT)],
            ("dfg_label_exact", 1),
            None,
            None
        ),
        case!(
            "c-fix1-const-pointer-definition",
            "declaration",
            "void f() {\n  const char *s = source();\n  sink(s);\n}\n",
            &[("2:s", "3:s", EXACT)],
            ("dfg_label_exact", 1),
            None,
            None
        ),
        case!(
            "c-fix1-pointer-to-pointer-definition",
            "declaration",
            "void f() {\n  int **pp = source();\n  sink(pp);\n}\n",
            &[("2:pp", "3:pp", EXACT)],
            ("dfg_label_exact", 1),
            None,
            None
        ),
    ];
    for case in &cases {
        super::case::behavior::assert_behavior(
            case,
            &super::super::parsed(case.src, case.language),
        );
    }
}

#[test]
fn fix1_w2_alias_facts_do_not_declare_foreign_names() {
    let cases = [
        case!(
            "c-fix1-alias-uninitialized",
            "declaration",
            "void f(int x) {\n  int y;\n  y = x;\n  sink(x);\n}\n",
            &[("1:x", "3:x", EXACT), ("1:x", "4:x", EXACT)],
            ("dfg_label_exact", 2),
            None,
            None
        ),
        case!(
            "c-fix1-alias-initialized",
            "declaration",
            "void f(int x) {\n  int y = x;\n  sink(x);\n}\n",
            &[("1:x", "3:x", EXACT)],
            ("dfg_label_exact", 1),
            None,
            None
        ),
        case!(
            "c-fix1-alias-conditional",
            "declaration",
            "void f(int x) {\n  int y;\n  if (ready()) y = x;\n  sink(x);\n}\n",
            &[("1:x", "3:x", EXACT), ("1:x", "4:x", EXACT)],
            ("dfg_label_exact", 2),
            None,
            None
        ),
        case!(
            "c-fix1-frr-match-from",
            "declaration",
            "void f(int match) {\n  int from;\n  if (ready()) from = match;\n  sink(match);\n}\n",
            &[("1:match", "3:match", EXACT), ("1:match", "4:match", EXACT)],
            ("dfg_label_exact", 2),
            None,
            None
        ),
    ];
    for case in &cases {
        super::case::behavior::assert_behavior(
            case,
            &super::super::parsed(case.src, case.language),
        );
    }
}

#[test]
fn fix1_w3_chain_uncertainty_is_scope_bounded() {
    const BLOCK: &[(&str, &str, FlowConfidence)] = &[
        ("2:x", "7:x", EXACT),
        ("4:x", "5:x", EXACT),
        ("4:x", "7:x", killed(6)),
    ];
    const FOR: &[(&str, &str, FlowConfidence)] = &[
        ("2:x", "4:x", killed(3)),
        ("2:x", "6:x", EXACT),
        ("3:x", "4:x", EXACT),
        ("3:x", "6:x", killed(5)),
    ];
    const SINGLE_LINE_FOR: &[(&str, &str, FlowConfidence)] =
        &[("2:i", "4:i", EXACT), ("3:i", "4:i", killed(4))];
    let cases = [
        case!("c-fix1-chain-block-scope", "declaration", "void f() {\n  int x = source();\n  {\n    int a = 1, x = 2;\n    sink(x);\n  }\n  sink(x);\n}\n", BLOCK, ("dfg_label_exact", 2), None, None),
        case!("c-fix1-chain-for-scope", "declaration", "void f() {\n  int x = source();\n  for (int a = 1, x = 2; ready(); step()) {\n    sink(x);\n  }\n  sink(x);\n}\n", FOR, ("dfg_label_nameonly_killed", 2), None, None),
        case!("c-fix1-single-line-for-scope-exit", "for_statement", "void f(int n) {\n  int i = 7;\n  for (int i = 0; i < n; i++) sink(i);\n  sink(i);\n}\n", SINGLE_LINE_FOR, ("dfg_label_exact", 1), None, None),
    ];
    for case in &cases {
        super::case::behavior::assert_behavior(
            case,
            &super::super::parsed(case.src, case.language),
        );
    }
}

#[test]
fn fix1_w4_multiline_chain_membership() {
    const OWNED: &[(&str, &str, FlowConfidence)] = &[("3:x", "4:x", EXACT)];
    let chain = Some(Predicate::FieldChildCountAtLeast {
        field: "declarator",
        count: 2,
    });
    let cases = [
        case!(
            "c-fix1-chain-continuation-line",
            "declaration",
            "void f() {\n  int a = 1,\n      x = 2;\n  sink(x);\n}\n",
            OWNED,
            ("dfg_label_exact", 1),
            None,
            chain
        ),
        case!(
            "c-fix1-chain-later-assignment",
            "declaration",
            "void f() {\n  int a, x;\n  x = source();\n  sink(x);\n}\n",
            OWNED,
            ("dfg_label_exact", 1),
            None,
            chain
        ),
    ];
    for case in &cases {
        super::case::behavior::assert_behavior(
            case,
            &super::super::parsed(case.src, case.language),
        );
    }
}

#[test]
fn fix2_f1_side_effect_definitions_survive_normalization() {
    const UPDATE: &[(&str, &str, FlowConfidence)] =
        &[("1:x", "3:x", killed(2)), ("2:x", "3:x", EXACT)];
    const POINTER_UPDATE: &[(&str, &str, FlowConfidence)] =
        &[("1:p", "3:p", killed(2)), ("2:p", "3:p", EXACT)];
    let cases = [
        case!(
            "c-fix2-side-effect-post-increment",
            "update_expression",
            "void f(int x) {\n  int y = x++;\n  sink(x);\n}\n",
            UPDATE,
            ("dfg_label_exact", 1),
            None,
            None
        ),
        case!(
            "c-fix2-side-effect-prefix-increment",
            "update_expression",
            "void f(int x) {\n  int y = ++x;\n  sink(x);\n}\n",
            UPDATE,
            ("dfg_label_exact", 1),
            None,
            None
        ),
        case!(
            "c-fix2-side-effect-post-decrement",
            "update_expression",
            "void f(int x) {\n  int y = x--;\n  sink(x);\n}\n",
            UPDATE,
            ("dfg_label_exact", 1),
            None,
            None
        ),
        case!(
            "c-fix2-side-effect-prefix-decrement",
            "update_expression",
            "void f(int x) {\n  int y = --x;\n  sink(x);\n}\n",
            UPDATE,
            ("dfg_label_exact", 1),
            None,
            None
        ),
        case!(
            "c-fix2-side-effect-simple-assignment",
            "assignment_expression",
            "void f(int x) {\n  int y = (x = clean());\n  sink(x);\n}\n",
            UPDATE,
            ("dfg_label_exact", 1),
            None,
            None
        ),
        case!(
            "c-fix2-side-effect-compound-assignment",
            "assignment_expression",
            "void f(int x) {\n  int y = (x += clean());\n  sink(x);\n}\n",
            UPDATE,
            ("dfg_label_exact", 1),
            None,
            None
        ),
        case!(
            "c-fix2-side-effect-pointer-update",
            "update_expression",
            "void f(char *p) {\n  int y = *p++;\n  sink(p);\n}\n",
            POINTER_UPDATE,
            ("dfg_label_exact", 1),
            None,
            None
        ),
        case!(
            "c-fix2-side-effect-initializer-read",
            "declaration",
            "void f(int x) {\n  int y = x;\n  sink(x);\n}\n",
            &[("1:x", "3:x", EXACT)],
            ("dfg_label_exact", 1),
            None,
            None
        ),
    ];
    for case in &cases {
        super::case::behavior::assert_behavior(
            case,
            &super::super::parsed(case.src, case.language),
        );
    }
}

#[test]
fn fix2_f2_outer_chain_shadow_is_parked() {
    const OWNED: &[(&str, &str, FlowConfidence)] =
        &[("4:x", "5:x", EXACT), ("4:x", "7:x", killed(6))];
    let case = case!(
        "c-fix2-parked-chain-shadow",
        "declaration",
        "void f() {\n  int x = source(), y = 0;\n  {\n    int x = clean();\n    sink(x);\n  }\n  sink(x);\n}\n",
        OWNED,
        ("dfg_label_exact", 1),
        None,
        None
    );
    super::case::behavior::assert_behavior(&case, &super::super::parsed(case.src, case.language));
}

#[test]
fn fix2_f3_recovered_parameter_seeds_fail_closed() {
    const RECOVERED: &[(&str, &str, FlowConfidence)] = &[("2:conn", "4:conn", uncertain(5))];
    const ORDINARY: &[(&str, &str, FlowConfidence)] =
        &[("1:conn", "3:conn", EXACT), ("1:conn", "4:conn", EXACT)];
    const REAL_MASK: &[(&str, &str, FlowConfidence)] =
        &[("1:conn", "2:conn", killed(2)), ("1:conn", "3:conn", EXACT)];
    let cases = [
        case!("c-fix2-recovered-macro-parameter", "parameter_declaration", "#define container_of(a,b,c) (a)\nvoid f(int *conn) {\n  if (ready()) {\n    sink(conn);\n    use(container_of(conn, struct msg_client, conn), 0);\n  }\n}\n", RECOVERED, ("dfg_label_nameonly_ownership_uncertain", 1), None, None),
        case!("c-fix2-ordinary-call-control", "parameter_declaration", "void f(int *conn) {\n  if (ready()) {\n    sink(conn);\n    use(conn, conn);\n  }\n}\n", ORDINARY, ("dfg_label_exact", 2), None, None),
        case!("c-fix2-real-parameter-mask", "declaration", "void f(int *conn) {\n  { int *conn = source(); sink(conn); }\n  sink(conn);\n}\n", REAL_MASK, ("dfg_label_exact", 1), None, None),
    ];
    for case in &cases {
        super::case::behavior::assert_behavior(
            case,
            &super::super::parsed(case.src, case.language),
        );
    }
}
