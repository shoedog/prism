use super::super::case::{Case, CaseClass};
use crate::cpg::reaching::binding_table::Predicate;
use crate::cpg::{FlowConfidence, FlowDoubt};
use crate::languages::Language;

const EXACT: FlowConfidence = FlowConfidence::Exact;

const fn killed(line: u32) -> FlowConfidence {
    FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: line })
}

const CFG_INCOMPLETE: FlowConfidence = FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete);

macro_rules! case {
    ($id:literal, $kind:literal, $src:literal, $expect:expr, $counter:expr, $row_variant:expr) => {
        Case {
            id: $id,
            language: Language::TypeScript,
            kind: $kind,
            variant: Some("binding"),
            row_variant: $row_variant,
            class: CaseClass::Behavioral,
            src: $src,
            expect: $expect,
            expect_counter: Some($counter),
        }
    };
    ($id:literal, $kind:literal, $src:literal, $expect:expr, $counter:expr) => {
        Case {
            id: $id,
            language: Language::TypeScript,
            kind: $kind,
            variant: Some("binding"),
            row_variant: None,
            class: CaseClass::Behavioral,
            src: $src,
            expect: $expect,
            expect_counter: Some($counter),
        }
    };
}

pub(in crate::cpg::reaching::tests::enumeration) const CASES: &[Case] = &[
    case!(
        "e0a-js-statement_block",
        "statement_block",
        "function f() {\n  let x: number = source();\n  { let x: number = clean(); sink(x); }\n  sink(x);\n}\n",
        &[("2:x", "3:x", killed(3)), ("2:x", "4:x", EXACT)],
        ("dfg_label_nameonly_killed", 1)
    ),
    case!(
        "e0a-js-class_body",
        "class_body",
        "function f() {\n  let x: number = source();\n  class C { method(): void { sink(x); } }\n  sink(x);\n}\n",
        &[("2:x", "3:x", CFG_INCOMPLETE), ("2:x", "4:x", EXACT)],
        ("dfg_label_nameonly_cfg_incomplete", 1)
    ),
    case!(
        "e0a-js-function_declaration",
        "function_declaration",
        "function f() {\n  let x: number = source();\n  function nested(): void { sink(x); }\n  sink(x);\n}\n",
        &[("2:x", "3:x", CFG_INCOMPLETE), ("2:x", "4:x", EXACT)],
        ("dfg_label_nameonly_cfg_incomplete", 1)
    ),
    case!(
        "e0a-js-function_expression",
        "function_expression",
        "function f() {\n  let x: number = source();\n  const nested = function (): void { sink(x); };\n  sink(x);\n}\n",
        &[("2:x", "3:x", CFG_INCOMPLETE), ("2:x", "4:x", EXACT)],
        ("dfg_label_nameonly_cfg_incomplete", 1)
    ),
    case!(
        "e0a-js-arrow_function",
        "arrow_function",
        "function f() {\n  let x: number = source();\n  const nested = (): void => { sink(x); };\n  sink(x);\n}\n",
        &[("2:x", "3:x", CFG_INCOMPLETE), ("2:x", "4:x", EXACT)],
        ("dfg_label_nameonly_cfg_incomplete", 1)
    ),
    case!(
        "e0a-js-for_statement",
        "for_statement",
        "function f(): void {\n  let outside: number = source();\n  for (let inside: number = clean(); ready; step()) {\n    sink(inside);\n  }\n  sink(outside);\n}\n",
        &[("3:inside", "4:inside", EXACT), ("2:outside", "6:outside", EXACT)],
        ("dfg_label_exact", 2)
    ),
    case!(
        "e0a-x-js-formal_parameters",
        "formal_parameters",
        "function f(x: number): void {\n  { let x: number = clean(); sink(x); }\n  sink(x);\n}\n",
        &[("1:x", "2:x", killed(2)), ("1:x", "3:x", EXACT)],
        ("dfg_label_nameonly_killed", 1)
    ),
    case!(
        "e0a-js-variable_declaration",
        "variable_declaration",
        "function f(): void {\n  let x: number = source();\n  { var x: number = clean(); sink(x); }\n  sink(x);\n}\n",
        &[("2:x", "3:x", EXACT), ("2:x", "4:x", killed(3)), ("3:x", "4:x", EXACT)],
        ("dfg_label_nameonly_killed", 1)
    ),
    case!(
        "e0a-js-lexical_declaration",
        "lexical_declaration",
        "function f(): void {\n  let x: number = source();\n  { let x: number = clean(); sink(x); }\n  sink(x);\n}\n",
        &[("2:x", "3:x", killed(3)), ("2:x", "4:x", EXACT)],
        ("dfg_label_nameonly_killed", 1)
    ),
    case!(
        "ts-required-parameter-seeds",
        "required_parameter",
        "function f(x: number): void {\n  sink(x);\n  { let x: number = clean(); sink(x); }\n  sink(x);\n}\n",
        &[("1:x", "2:x", EXACT), ("1:x", "3:x", killed(3)), ("1:x", "4:x", EXACT)],
        ("dfg_label_nameonly_killed", 1),
        Some(Predicate::TypeScriptParameterPattern {
            kind: "identifier",
        })
    ),
    case!(
        "ts-optional-parameter",
        "optional_parameter",
        "function f(x?: number): void {\n  sink(x);\n  { let x: number = clean(); sink(x); }\n  sink(x);\n}\n",
        &[("1:x", "2:x", EXACT), ("1:x", "3:x", killed(3)), ("1:x", "4:x", EXACT)],
        ("dfg_label_nameonly_killed", 1),
        Some(Predicate::TypeScriptParameterPattern {
            kind: "identifier",
        })
    ),
    case!(
        "ts-rest-parameter",
        "required_parameter",
        "function f(...x: number[]): void {\n  sink(x);\n  { let x: number[] = clean(); sink(x); }\n  sink(x);\n}\n",
        &[("1:x", "2:x", EXACT), ("1:x", "3:x", killed(3)), ("1:x", "4:x", EXACT)],
        ("dfg_label_nameonly_killed", 1),
        Some(Predicate::TypeScriptParameterPattern {
            kind: "rest_pattern",
        })
    ),
    Case {
        id: "ts-required-parameter-pattern-uncertain",
        language: Language::TypeScript,
        kind: "required_parameter",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Structural,
        src: "function f({ x }: Box): void { sink(x); }\n",
        expect: &[],
        expect_counter: None,
    },
    Case {
        id: "ts-optional-parameter-pattern-uncertain",
        language: Language::TypeScript,
        kind: "optional_parameter",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Structural,
        src: "function f([x]?: Box): void { sink(x); }\n",
        expect: &[],
        expect_counter: None,
    },
    case!(
        "ts-rest-pattern-owned-by-parameter",
        "rest_pattern",
        "function f(...x: number[]): void {\n  observe();\n  sink(x);\n}\n",
        &[("1:x", "3:x", EXACT)],
        ("dfg_label_exact", 1),
        Some(Predicate::ParentKindIs {
            kind: "required_parameter",
        })
    ),
    Case {
        id: "e0b-ts-rest_pattern",
        language: Language::TypeScript,
        kind: "rest_pattern",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Structural,
        src: "function f(values: number[]): void { let [...x] = values; sink(x); }\n",
        expect: &[],
        expect_counter: None,
    },
    case!(
        "ts-enum-assignment-not-binding",
        "enum_assignment",
        "function f(): void {\n  let A: number = source();\n  enum E { A = 1 }\n  sink(A);\n}\n",
        &[("2:A", "4:A", EXACT)],
        ("dfg_label_exact", 1)
    ),
    case!(
        "ts-abstract-class-scope",
        "abstract_class_declaration",
        "function f(): void {\n  let x: number = source();\n  abstract class C { method(): void { sink(x); } }\n  sink(x);\n}\n",
        &[("2:x", "3:x", CFG_INCOMPLETE), ("2:x", "4:x", EXACT)],
        ("dfg_label_nameonly_cfg_incomplete", 1)
    ),
    Case {
        id: "ts-abstract-class-inner-def",
        language: Language::TypeScript,
        kind: "abstract_class_declaration",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "abstract class C {\n  method(): void {\n    let x: number = source();\n    sink(x);\n  }\n}\n",
        expect: &[("3:x", "4:x", EXACT)],
        expect_counter: Some(("dfg_label_exact", 1)),
    },
];

#[test]
fn task17_cases() {
    for case in CASES {
        super::super::case::check_case(case);
    }
}

#[test]
fn task17_parameter_patterns_fail_closed_to_the_function_body() {
    use crate::cpg::reaching::binding_table::{select_binding_row, Ruling};
    use crate::cpg::reaching::grammar_lint::{visibility_span, SpanKind};

    for (src, kind) in [
        (
            "function f({ x }: Box): void { sink(x); }",
            "required_parameter",
        ),
        (
            "function f([x]?: Box): void { sink(x); }",
            "optional_parameter",
        ),
    ] {
        let parsed = super::super::super::parsed(src, Language::TypeScript);
        let parameter = super::super::case::nodes_of_kind(&parsed, kind)[0];
        let row = select_binding_row(&parsed, parameter).expect("parameter row");
        assert_eq!(
            row.ruling,
            Ruling::Uncertain {
                reason: "pattern identifiers not field-addressable",
                revisit: "future:ts-destructuring",
            }
        );
        let function = super::super::super::function(&parsed);
        let body = function.child_by_field_name("body").expect("function body");
        assert_eq!(
            visibility_span(&parsed, parameter),
            SpanKind::Known(vec![crate::cpg::reaching::grammar_lint::ByteSpan {
                start: body.start_byte(),
                end: body.end_byte(),
            }])
        );
    }

    for src in [
        "function f(this: Context): void {}",
        "class C { constructor(public x: number) {} }",
    ] {
        let parsed = super::super::super::parsed(src, Language::TypeScript);
        let parameter = super::super::case::nodes_of_kind(&parsed, "required_parameter")[0];
        assert!(
            matches!(
                select_binding_row(&parsed, parameter).map(|row| row.ruling),
                Some(Ruling::Uncertain {
                    revisit: "future:ts-destructuring",
                    ..
                })
            ),
            "{src} must select the residual fail-closed row"
        );
    }
}

#[test]
fn task17_declaration_rows_own_only_named_fields() {
    use crate::cpg::reaching::binding_table::{select_binding_row, DeclarationKind};
    use crate::cpg::reaching::transfer::row_introduced_identifiers;

    let parsed = super::super::super::parsed(
        "function declared(x: number): void {\n  nested = 1;\n  let owned = (() => { body = 1; return body; })();\n  var varOwned = 0;\n  for (let loopOwned = 0; loopOwned < 1; loopOwned++) { loopBody = 1; }\n  abstract class Owned { method(): void { member = 1; } }\n}\n",
        Language::TypeScript,
    );
    let required = super::super::case::nodes_of_kind(&parsed, "required_parameter")[0];
    let row = select_binding_row(&parsed, required).expect("required parameter row");
    assert_eq!(row.declaration, Some(DeclarationKind::Parameter));
    let introduced: Vec<_> = row_introduced_identifiers(&parsed, required, row.fields)
        .into_iter()
        .map(|node| parsed.node_text(&node))
        .collect();
    assert_eq!(introduced, ["x"]);

    for (kind, declaration) in [
        ("lexical_declaration", DeclarationKind::Other),
        ("variable_declaration", DeclarationKind::JavaScriptVar),
    ] {
        let node = super::super::case::nodes_of_kind(&parsed, kind)[0];
        let row = select_binding_row(&parsed, node).expect("declaration row");
        assert_eq!(row.declaration, Some(declaration));
        assert!(
            row.fields.is_empty(),
            "{kind} must not name nested assignments"
        );
    }
    for assignment in super::super::case::nodes_of_kind(&parsed, "assignment_expression") {
        let row = select_binding_row(&parsed, assignment).expect("assignment row");
        assert_eq!(row.declaration, None, "assignment must remain reuse-only");
    }

    for kind in [
        "function_declaration",
        "arrow_function",
        "for_statement",
        "abstract_class_declaration",
    ] {
        let node = super::super::case::nodes_of_kind(&parsed, kind)[0];
        let row = select_binding_row(&parsed, node).expect("scope row");
        assert_eq!(
            row.declaration, None,
            "{kind} must not claim body assignments"
        );
    }
}
