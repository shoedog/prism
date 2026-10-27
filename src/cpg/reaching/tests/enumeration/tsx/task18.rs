use super::super::case::{Case, CaseClass};
use crate::cpg::{FlowConfidence, FlowDoubt};
use crate::languages::Language;

const EXACT: FlowConfidence = FlowConfidence::Exact;

const fn killed(line: u32) -> FlowConfidence {
    FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: line })
}

const CFG_INCOMPLETE: FlowConfidence = FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete);

macro_rules! tsx_case {
    ($id:literal, $kind:literal, $src:literal, $expect:expr, $counter:expr) => {
        Case {
            id: $id,
            language: Language::Tsx,
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

macro_rules! transparent_case {
    ($id:literal, $language:expr, $kind:literal, $src:literal, $from:literal, $to:literal) => {
        Case {
            id: $id,
            language: $language,
            kind: $kind,
            variant: Some("binding"),
            row_variant: None,
            class: CaseClass::Behavioral,
            src: $src,
            expect: &[($from, $to, EXACT)],
            expect_counter: Some(("dfg_label_exact", 1)),
        }
    };
}

pub(in crate::cpg::reaching::tests::enumeration) const CASES: &[Case] = &[
    tsx_case!(
        "e0a-js-statement_block",
        "statement_block",
        "function f() {\n  let x: number = source();\n  { let x: number = clean(); sink(x); }\n  sink(x);\n}\n",
        &[("2:x", "3:x", killed(3)), ("2:x", "4:x", EXACT)],
        ("dfg_label_nameonly_killed", 1)
    ),
    tsx_case!(
        "e0a-js-class_body",
        "class_body",
        "function f() {\n  let x: number = source();\n  class C { method(): void { sink(x); } }\n  sink(x);\n}\n",
        &[("2:x", "3:x", CFG_INCOMPLETE), ("2:x", "4:x", EXACT)],
        ("dfg_label_nameonly_cfg_incomplete", 1)
    ),
    tsx_case!(
        "e0a-js-function_declaration",
        "function_declaration",
        "function f() {\n  let x: number = source();\n  function nested(): void { sink(x); }\n  sink(x);\n}\n",
        &[("2:x", "3:x", CFG_INCOMPLETE), ("2:x", "4:x", EXACT)],
        ("dfg_label_nameonly_cfg_incomplete", 1)
    ),
    tsx_case!(
        "e0a-js-function_expression",
        "function_expression",
        "function f() {\n  let x: number = source();\n  const nested = function (): void { sink(x); };\n  sink(x);\n}\n",
        &[("2:x", "3:x", CFG_INCOMPLETE), ("2:x", "4:x", EXACT)],
        ("dfg_label_nameonly_cfg_incomplete", 1)
    ),
    tsx_case!(
        "e0a-js-arrow_function",
        "arrow_function",
        "function f() {\n  let x: number = source();\n  const nested = (): void => { sink(x); };\n  sink(x);\n}\n",
        &[("2:x", "3:x", CFG_INCOMPLETE), ("2:x", "4:x", EXACT)],
        ("dfg_label_nameonly_cfg_incomplete", 1)
    ),
    tsx_case!(
        "e0a-js-for_statement",
        "for_statement",
        "function f(): void {\n  let outside: number = source();\n  for (let inside: number = clean(); ready; step()) {\n    sink(inside);\n  }\n  sink(outside);\n}\n",
        &[("3:inside", "4:inside", EXACT), ("2:outside", "6:outside", EXACT)],
        ("dfg_label_exact", 2)
    ),
    tsx_case!(
        "e0a-x-js-formal_parameters",
        "formal_parameters",
        "function f(x: number): void {\n  { let x: number = clean(); sink(x); }\n  sink(x);\n}\n",
        &[("1:x", "2:x", killed(2)), ("1:x", "3:x", EXACT)],
        ("dfg_label_nameonly_killed", 1)
    ),
    tsx_case!(
        "e0a-js-variable_declaration",
        "variable_declaration",
        "function f(): void {\n  let x: number = source();\n  { var x: number = clean(); sink(x); }\n  sink(x);\n}\n",
        &[("2:x", "3:x", EXACT), ("2:x", "4:x", killed(3)), ("3:x", "4:x", EXACT)],
        ("dfg_label_nameonly_killed", 1)
    ),
    tsx_case!(
        "e0a-js-lexical_declaration",
        "lexical_declaration",
        "function f(): void {\n  let x: number = source();\n  { let x: number = clean(); sink(x); }\n  sink(x);\n}\n",
        &[("2:x", "3:x", killed(3)), ("2:x", "4:x", EXACT)],
        ("dfg_label_nameonly_killed", 1)
    ),
    transparent_case!(
        "ts-type-alias-not-binding",
        Language::TypeScript,
        "type_alias_declaration",
        "function f(): void {\n  let x = source();\n  type T = { x: number };\n  sink(x);\n}\n",
        "2:x",
        "4:x"
    ),
    transparent_case!(
        "ts-interface-not-binding",
        Language::TypeScript,
        "interface_declaration",
        "function f(): void {\n  let x = source();\n  interface I { x: string }\n  sink(x);\n}\n",
        "2:x",
        "4:x"
    ),
    transparent_case!(
        "ts-ambient-not-binding",
        Language::TypeScript,
        "ambient_declaration",
        "function f(): void {\n  let x = source();\n  declare namespace N { const x: number; }\n  sink(x);\n}\n",
        "2:x",
        "4:x"
    ),
    transparent_case!(
        "ts-enum-declaration-not-binding",
        Language::TypeScript,
        "enum_declaration",
        "function f(): void {\n  let x = source();\n  enum E { x = 1 }\n  sink(x);\n}\n",
        "2:x",
        "4:x"
    ),
    transparent_case!(
        "ts-type-parameter-not-binding",
        Language::TypeScript,
        "type_parameter",
        "function f(): void {\n  let x = source();\n  function g<x>(): void {}\n  sink(x);\n}\n",
        "2:x",
        "4:x"
    ),
    transparent_case!(
        "ts-type-parameters-not-binding",
        Language::TypeScript,
        "type_parameters",
        "function f(): void {\n  let x = source();\n  function g<x>(): void {}\n  sink(x);\n}\n",
        "2:x",
        "4:x"
    ),
    transparent_case!(
        "tsx-type-alias-not-binding",
        Language::Tsx,
        "type_alias_declaration",
        "function f(): void {\n  let x = source();\n  type T = { x: number };\n  sink(x);\n}\n",
        "2:x",
        "4:x"
    ),
    transparent_case!(
        "tsx-interface-not-binding",
        Language::Tsx,
        "interface_declaration",
        "function f(): void {\n  let x = source();\n  interface I { x: string }\n  sink(x);\n}\n",
        "2:x",
        "4:x"
    ),
    transparent_case!(
        "tsx-ambient-not-binding",
        Language::Tsx,
        "ambient_declaration",
        "function f(): void {\n  let x = source();\n  declare namespace N { const x: number; }\n  sink(x);\n}\n",
        "2:x",
        "4:x"
    ),
    transparent_case!(
        "tsx-enum-declaration-not-binding",
        Language::Tsx,
        "enum_declaration",
        "function f(): void {\n  let x = source();\n  enum E { x = 1 }\n  sink(x);\n}\n",
        "2:x",
        "4:x"
    ),
    transparent_case!(
        "tsx-type-parameter-not-binding",
        Language::Tsx,
        "type_parameter",
        "function f(): void {\n  let x = source();\n  function g<x>(): void {}\n  sink(x);\n}\n",
        "2:x",
        "4:x"
    ),
    transparent_case!(
        "tsx-type-parameters-not-binding",
        Language::Tsx,
        "type_parameters",
        "function f(): void {\n  let x = source();\n  function g<x>(): void {}\n  sink(x);\n}\n",
        "2:x",
        "4:x"
    ),
    transparent_case!(
        "ts-function-type-not-binding",
        Language::TypeScript,
        "function_type",
        "function f(): void {\n  let x = source();\n  let g: ({ x }: Box) => void = h;\n  sink(x);\n}\n",
        "2:x",
        "4:x"
    ),
    transparent_case!(
        "ts-method-signature-not-binding",
        Language::TypeScript,
        "method_signature",
        "function f(): void {\n  let x = source();\n  let o: { m({ x }: Box): void } = h;\n  sink(x);\n}\n",
        "2:x",
        "4:x"
    ),
    transparent_case!(
        "ts-constructor-type-not-binding",
        Language::TypeScript,
        "constructor_type",
        "function f(): void {\n  let x = source();\n  let C: new ({ x }: Box) => Thing = h;\n  sink(x);\n}\n",
        "2:x",
        "4:x"
    ),
    transparent_case!(
        "ts-call-signature-not-binding",
        Language::TypeScript,
        "call_signature",
        "function f(): void {\n  let x = source();\n  let o: { ({ x }: Box): void } = h;\n  sink(x);\n}\n",
        "2:x",
        "4:x"
    ),
    transparent_case!(
        "ts-construct-signature-not-binding",
        Language::TypeScript,
        "construct_signature",
        "function f(): void {\n  let x = source();\n  let o: { new ({ x }: Box): Thing } = h;\n  sink(x);\n}\n",
        "2:x",
        "4:x"
    ),
    transparent_case!(
        "ts-function-signature-not-binding",
        Language::TypeScript,
        "function_signature",
        "function f(): void {\n  let x = source();\n  function g({ x }: Box): void;\n  function g(a: any): void {}\n  sink(x);\n}\n",
        "2:x",
        "5:x"
    ),
    transparent_case!(
        "ts-abstract-method-signature-not-binding",
        Language::TypeScript,
        "abstract_method_signature",
        "function f(): void {\n  let x = source();\n  abstract class A { abstract m({ x }: Box): void; }\n  sink(x);\n}\n",
        "2:x",
        "4:x"
    ),
    transparent_case!(
        "tsx-function-type-not-binding",
        Language::Tsx,
        "function_type",
        "function f(): void {\n  let x = source();\n  let g: ({ x }: Box) => void = h;\n  sink(x);\n}\n",
        "2:x",
        "4:x"
    ),
    transparent_case!(
        "tsx-method-signature-not-binding",
        Language::Tsx,
        "method_signature",
        "function f(): void {\n  let x = source();\n  let o: { m({ x }: Box): void } = h;\n  sink(x);\n}\n",
        "2:x",
        "4:x"
    ),
    transparent_case!(
        "tsx-constructor-type-not-binding",
        Language::Tsx,
        "constructor_type",
        "function f(): void {\n  let x = source();\n  let C: new ({ x }: Box) => Thing = h;\n  sink(x);\n}\n",
        "2:x",
        "4:x"
    ),
    transparent_case!(
        "tsx-call-signature-not-binding",
        Language::Tsx,
        "call_signature",
        "function f(): void {\n  let x = source();\n  let o: { ({ x }: Box): void } = h;\n  sink(x);\n}\n",
        "2:x",
        "4:x"
    ),
    transparent_case!(
        "tsx-construct-signature-not-binding",
        Language::Tsx,
        "construct_signature",
        "function f(): void {\n  let x = source();\n  let o: { new ({ x }: Box): Thing } = h;\n  sink(x);\n}\n",
        "2:x",
        "4:x"
    ),
    transparent_case!(
        "tsx-function-signature-not-binding",
        Language::Tsx,
        "function_signature",
        "function f(): void {\n  let x = source();\n  function g({ x }: Box): void;\n  function g(a: any): void {}\n  sink(x);\n}\n",
        "2:x",
        "5:x"
    ),
    transparent_case!(
        "tsx-abstract-method-signature-not-binding",
        Language::Tsx,
        "abstract_method_signature",
        "function f(): void {\n  let x = source();\n  abstract class A { abstract m({ x }: Box): void; }\n  sink(x);\n}\n",
        "2:x",
        "4:x"
    ),
    transparent_case!(
        "tsx-jsx-attribute-not-binding",
        Language::Tsx,
        "jsx_attribute",
        "function f(): JSX.Element {\n  let x = source();\n  const view = <C x={x} />;\n  sink(x);\n  return view;\n}\n",
        "2:x",
        "4:x"
    ),
    transparent_case!(
        "tsx-jsx-expression-not-binding",
        Language::Tsx,
        "jsx_expression",
        "function f(): JSX.Element {\n  let x = source();\n  const view = <C>{x}</C>;\n  sink(x);\n  return view;\n}\n",
        "2:x",
        "4:x"
    ),
    transparent_case!(
        "tsx-jsx-namespace-name-not-binding",
        Language::Tsx,
        "jsx_namespace_name",
        "function f(): JSX.Element {\n  let x = source();\n  const view = <x:item />;\n  sink(x);\n  return view;\n}\n",
        "2:x",
        "4:x"
    ),
];

#[test]
fn task18_cases() {
    for case in CASES {
        super::super::case::check_case(case);
    }
}

#[test]
fn task18_tsx_rows_match_typescript_twins_except_language() {
    use crate::cpg::reaching::binding_table::{rows, BindingRow};

    let typescript = rows(Language::TypeScript);
    let tsx = rows(Language::Tsx);
    for kind in [
        "statement_block",
        "class_body",
        "function_declaration",
        "function_expression",
        "arrow_function",
        "for_statement",
        "formal_parameters",
        "variable_declaration",
        "lexical_declaration",
    ] {
        let ts_row = typescript
            .iter()
            .find(|row| row.kind == kind && row.language == Language::TypeScript)
            .unwrap_or_else(|| panic!("missing TypeScript row {kind}"));
        let tsx_row = tsx
            .iter()
            .find(|row| row.kind == kind)
            .unwrap_or_else(|| panic!("missing TSX row {kind}"));
        assert_eq!(tsx_row.language, Language::Tsx, "TSX-owned row {kind}");
        assert_eq!(
            BindingRow {
                language: Language::TypeScript,
                ..*tsx_row
            },
            *ts_row,
            "shared curated row {kind}"
        );
    }
}

#[test]
fn task18_tsx_declaration_rows_do_not_claim_nested_assignments() {
    use crate::cpg::reaching::binding_table::{select_binding_row, DeclarationKind};

    let parsed = super::super::super::parsed(
        "function declared(x: number): JSX.Element {\n  nested = 1;\n  let owned = (() => { body = 1; return <C x={body} />; })();\n  var varOwned = 0;\n  for (let loopOwned = 0; loopOwned < 1; loopOwned++) { loopBody = 1; }\n  return <C x={x} />;\n}\n",
        Language::Tsx,
    );
    for (kind, declaration) in [
        ("formal_parameters", DeclarationKind::Parameter),
        ("lexical_declaration", DeclarationKind::Other),
        ("variable_declaration", DeclarationKind::JavaScriptVar),
    ] {
        let node = super::super::case::nodes_of_kind(&parsed, kind)[0];
        let row = select_binding_row(&parsed, node).expect("declaration row");
        assert_eq!(row.language, Language::Tsx, "TSX-owned {kind}");
        assert_eq!(row.declaration, Some(declaration), "{kind} declaration");
        assert!(row.fields.is_empty(), "{kind} must not claim nested names");
    }
    for kind in [
        "function_declaration",
        "arrow_function",
        "for_statement",
        "statement_block",
    ] {
        for node in super::super::case::nodes_of_kind(&parsed, kind) {
            let row = select_binding_row(&parsed, node).expect("scope row");
            assert_eq!(
                row.declaration, None,
                "{kind} must not claim body assignments"
            );
        }
    }
}

fn assert_type_position_parameter_transparent(language: Language, src: &str, owner_kind: &str) {
    use crate::cpg::reaching::grammar_lint::{uncertain_binding_nodes, visibility_span, SpanKind};

    let parsed = super::super::super::parsed(src, language);
    let parameter = super::super::case::nodes_of_kind(&parsed, "required_parameter")
        .into_iter()
        .find(|parameter| {
            parameter
                .parent()
                .and_then(|parameters| parameters.parent())
                .is_some_and(|owner| owner.kind() == owner_kind)
        })
        .unwrap_or_else(|| panic!("missing parameter owned by {owner_kind}"));
    assert_eq!(visibility_span(&parsed, parameter), SpanKind::Empty);
    assert!(
        uncertain_binding_nodes(&parsed, "x")
            .into_iter()
            .all(|binder| binder.id() != parameter.id()),
        "{language:?} {owner_kind} type-position parameter must be transparent"
    );
}

#[test]
fn task18_bodiless_signature_parameters_are_transparent_in_typescript_and_tsx() {
    for language in [Language::TypeScript, Language::Tsx] {
        for (owner, src) in [
            ("function_type", "function f(): void { let x = source(); let g: ({ x }: Box) => void = h; sink(x); }"),
            ("method_signature", "function f(): void { let x = source(); let o: { m({ x }: Box): void } = h; sink(x); }"),
            ("constructor_type", "function f(): void { let x = source(); let C: new ({ x }: Box) => Thing = h; sink(x); }"),
            ("call_signature", "function f(): void { let x = source(); let o: { ({ x }: Box): void } = h; sink(x); }"),
            ("construct_signature", "function f(): void { let x = source(); let o: { new ({ x }: Box): Thing } = h; sink(x); }"),
            ("function_signature", "function f(): void { let x = source(); function g({ x }: Box): void; function g(a: any): void {} sink(x); }"),
            ("abstract_method_signature", "function f(): void { let x = source(); abstract class A { abstract m({ x }: Box): void; } sink(x); }"),
        ] {
            assert_type_position_parameter_transparent(language, src, owner);
        }
    }
}
