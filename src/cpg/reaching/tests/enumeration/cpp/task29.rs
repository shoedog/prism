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
fn task29_cpp_chain_matches_single_declarations_after_the_declaration() {
    let cases = [
        Case {
            id: "cpp-chain-per-declarator",
            language: Language::Cpp,
            kind: "declaration",
            variant: None,
            row_variant: None,
            class: CaseClass::Behavioral,
            src: "void f() {\n  int a = source(), x = clean();\n  sink(a);\n  sink(x);\n}\n",
            expect: &[("2:a", "3:a", EXACT), ("2:x", "4:x", EXACT)],
            expect_counter: Some(("dfg_label_exact", 2)),
        },
        Case {
            id: "cpp-chain-single-declaration-control",
            language: Language::Cpp,
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
fn task29_cpp_chain_nested_shadow_and_same_line_read_partition() {
    const SHADOW: &[(&str, &str, FlowConfidence)] =
        &[("2:x", "3:x", killed(3)), ("2:x", "4:x", EXACT)];
    const SAME_LINE: &[(&str, &str, FlowConfidence)] =
        &[("2:a", "2:a", uncertain(2)), ("2:a", "3:a", EXACT)];
    const MULTI_LINE: &[(&str, &str, FlowConfidence)] =
        &[("2:a", "3:a", uncertain(2)), ("2:a", "4:a", EXACT)];
    let cases = [
        Case {
            id: "cpp-chain-nested-shadow",
            language: Language::Cpp,
            kind: "declaration",
            variant: None,
            row_variant: None,
            class: CaseClass::Behavioral,
            src: "void f() {\n  int x = source();\n  { int a = 1, x = clean(); sink(x); }\n  sink(x);\n}\n",
            expect: SHADOW,
            expect_counter: Some(("dfg_label_nameonly_killed", 1)),
        },
        Case {
            id: "cpp-chain-same-line-read-uncertain",
            language: Language::Cpp,
            kind: "declaration",
            variant: None,
            row_variant: None,
            class: CaseClass::Behavioral,
            src: "void f() {\n  int a = source(), b = a;\n  sink(a);\n}\n",
            expect: SAME_LINE,
            expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
        },
        Case {
            id: "cpp-chain-multi-line-read-uncertain",
            language: Language::Cpp,
            kind: "declaration",
            variant: None,
            row_variant: None,
            class: CaseClass::Behavioral,
            src: "void f() {\n  int a = source(),\n      b = a;\n  sink(a);\n}\n",
            expect: MULTI_LINE,
            expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
        },
    ];
    for case in &cases {
        check(case);
    }
}

#[test]
fn task29_cpp_direct_init_and_most_vexing_parse_stay_uncertain() {
    const DIRECT: &[(&str, &str, FlowConfidence)] = &[("3:x", "4:x", uncertain(3))];
    let direct = Case {
        id: "cpp-direct-init-uncertain",
        language: Language::Cpp,
        kind: "declaration",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "struct T { T(int); };\nvoid f(int y) {\n  T x(y);\n  sink(x);\n}\n",
        expect: DIRECT,
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
    };
    let direct_parsed = super::super::super::parsed(direct.src, Language::Cpp);
    let direct_declaration = super::super::case::nodes_of_kind(&direct_parsed, "declaration")
        .into_iter()
        .find(|node| direct_parsed.node_text(node).contains("x(y)"))
        .expect("direct-initialization declaration");
    let direct_row = super::super::super::super::binding_table::select_binding_row(
        &direct_parsed,
        direct_declaration,
    )
    .expect("direct-initialization row");
    let names: Vec<_> = super::super::super::super::transfer::row_introduced_identifiers(
        &direct_parsed,
        direct_declaration,
        direct_row.fields,
    )
    .into_iter()
    .map(|identifier| direct_parsed.node_text(&identifier))
    .collect();
    assert_eq!(names, ["x"]);
    check(&direct);

    let parsed = super::super::super::parsed(
        "struct T { T(); };\nvoid f() {\n  T x(T());\n}\n",
        Language::Cpp,
    );
    let declaration = super::super::case::nodes_of_kind(&parsed, "declaration")
        .into_iter()
        .find(|node| parsed.node_text(node).contains("x(T())"))
        .expect("most-vexing declaration");
    let row = super::super::super::super::binding_table::select_binding_row(&parsed, declaration)
        .expect("most-vexing declaration row");
    assert!(matches!(
        row.ruling,
        super::super::super::super::binding_table::Ruling::Uncertain {
            revisit: "future:cpp-declarators",
            ..
        }
    ));
}
