use super::super::case::{Case, CaseClass};
use crate::cpg::reaching::binding_table::Predicate;
use crate::cpg::{FlowConfidence, FlowDoubt};
use crate::languages::Language;

mod r1;

const EXACT: FlowConfidence = FlowConfidence::Exact;
const IMPORT_SOURCE: &str = "import defaultName, * as ns from 'pkg' with { type: 'json' };\nimport {item as alias} from 'pkg';\nfunction f() {\n  let keep = source();\n  sink(keep);\n}\n";

const fn killed(line: u32) -> FlowConfidence {
    FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: line })
}

const fn uncertain(line: u32) -> FlowConfidence {
    FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain {
        construct_line: line,
    })
}

macro_rules! case {
    ($id:literal, $kind:literal, $src:expr, $expect:expr, $counter:expr, $row_variant:expr) => {
        Case {
            id: $id,
            language: Language::JavaScript,
            kind: $kind,
            variant: Some("binding"),
            row_variant: $row_variant,
            class: CaseClass::Behavioral,
            src: $src,
            expect: $expect,
            expect_counter: Some($counter),
        }
    };
    ($id:literal, $kind:literal, $src:expr, $expect:expr, $counter:expr) => {
        Case {
            id: $id,
            language: Language::JavaScript,
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
        "function f() {\n  let x = source();\n  { let x = clean(); sink(x); }\n  sink(x);\n}\n",
        &[("2:x", "3:x", killed(3)), ("2:x", "4:x", EXACT)],
        ("dfg_label_nameonly_killed", 1)
    ),
    case!(
        "e0a-js-class_body",
        "class_body",
        "function f() {\n  let x = source();\n  class C { method() { sink(x); } }\n  sink(x);\n}\n",
        &[
            (
                "2:x",
                "3:x",
                FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
            ),
            ("2:x", "4:x", EXACT),
        ],
        ("dfg_label_nameonly_cfg_incomplete", 1)
    ),
    case!(
        "e0a-js-function_declaration",
        "function_declaration",
        "function f() {\n  let x = source();\n  function nested() { sink(x); }\n  sink(x);\n}\n",
        &[
            (
                "2:x",
                "3:x",
                FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
            ),
            ("2:x", "4:x", EXACT),
        ],
        ("dfg_label_nameonly_cfg_incomplete", 1)
    ),
    case!(
        "e0a-js-function_expression",
        "function_expression",
        "function f() {\n  let x = source();\n  const nested = function () { sink(x); };\n  sink(x);\n}\n",
        &[
            (
                "2:x",
                "3:x",
                FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
            ),
            ("2:x", "4:x", EXACT),
        ],
        ("dfg_label_nameonly_cfg_incomplete", 1)
    ),
    case!(
        "e0a-js-arrow_function",
        "arrow_function",
        "function f() {\n  let x = source();\n  const nested = () => sink(x);\n  sink(x);\n}\n",
        &[
            (
                "2:x",
                "3:x",
                FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
            ),
            ("2:x", "4:x", EXACT),
        ],
        ("dfg_label_nameonly_cfg_incomplete", 1)
    ),
    case!(
        "e0a-js-for_statement",
        "for_statement",
        "function f() {\n  let outside = source();\n  for (let inside = clean(); ready; step()) {\n    sink(inside);\n  }\n  sink(outside);\n}\n",
        &[("3:inside", "4:inside", EXACT), ("2:outside", "6:outside", EXACT)],
        ("dfg_label_exact", 2)
    ),
    case!(
        "e0a-x-js-formal_parameters",
        "formal_parameters",
        "function f(x) {\n  { let x = clean(); sink(x); }\n  sink(x);\n}\n",
        &[("1:x", "2:x", killed(2)), ("1:x", "3:x", EXACT)],
        ("dfg_label_nameonly_killed", 1)
    ),
    case!(
        "e0a-js-variable_declaration",
        "variable_declaration",
        "function f() {\n  let x = source();\n  { var x = clean(); sink(x); }\n  sink(x);\n}\n",
        &[("2:x", "3:x", EXACT), ("2:x", "4:x", killed(3)), ("3:x", "4:x", EXACT)],
        ("dfg_label_nameonly_killed", 1)
    ),
    case!(
        "js-destructure-assignment-reuse",
        "assignment_expression",
        "function f(o) {\n  x = source();\n  [x] = o;\n  sink(x);\n}\n",
        &[("2:x", "4:x", killed(3)), ("3:x", "4:x", EXACT)],
        ("dfg_label_nameonly_killed", 1)
    ),
    case!(
        "js-lexical-destructuring-declarator",
        "variable_declarator",
        "function f(o, x) {\n  { const {x} = o; sink(x); }\n  sink(x);\n}\n",
        &[("1:x", "2:x", uncertain(2)), ("1:x", "3:x", EXACT)],
        ("dfg_label_nameonly_ownership_uncertain", 1),
        Some(Predicate::JavaScriptDestructuringDeclarator {
            lexical: true,
        })
    ),
    case!(
        "js-var-destructuring-declarator",
        "variable_declarator",
        "function f(o, x) {\n  { var {x} = o; sink(x); }\n  sink(x);\n}\n",
        &[
            ("1:x", "2:x", uncertain(2)),
            ("1:x", "3:x", uncertain(2)),
            ("2:x", "3:x", EXACT),
        ],
        ("dfg_label_nameonly_ownership_uncertain", 2),
        Some(Predicate::JavaScriptDestructuringDeclarator {
            lexical: false,
        })
    ),
    case!(
        "js-variable-declarator-not-pattern",
        "variable_declarator",
        "function f() {\n  let keep = source();\n  let ordinary = clean();\n  sink(keep);\n}\n",
        &[("2:keep", "4:keep", EXACT)],
        ("dfg_label_exact", 1)
    ),
    case!(
        "js-destructure-masks",
        "object_pattern",
        "function f(o) {\n  let x = source();\n  { const {x} = o; sink(x); }\n  sink(x);\n}\n",
        &[("2:x", "3:x", uncertain(3)), ("2:x", "4:x", EXACT)],
        ("dfg_label_nameonly_ownership_uncertain", 1),
        Some(Predicate::JavaScriptBindingPattern)
    ),
    case!(
        "js-array-pattern",
        "array_pattern",
        "function f(o) {\n  let x = source();\n  { const [x] = o; sink(x); }\n  sink(x);\n}\n",
        &[("2:x", "3:x", uncertain(3)), ("2:x", "4:x", EXACT)],
        ("dfg_label_nameonly_ownership_uncertain", 1),
        Some(Predicate::JavaScriptBindingPattern)
    ),
    case!(
        "js-destructure-default-reads-own-name",
        "assignment_pattern",
        "function f(o) {\n  let x = source();\n  { const [x =\n      x] = o;\n    sink(x); }\n  sink(x);\n}\n",
        &[("2:x", "4:x", uncertain(3)), ("2:x", "5:x", uncertain(3)), ("2:x", "6:x", EXACT)],
        ("dfg_label_nameonly_ownership_uncertain", 2),
        Some(Predicate::JavaScriptBindingPattern)
    ),
    case!(
        "js-rest-pattern",
        "rest_pattern",
        "function f(o) {\n  let rest = source();\n  { const {...rest} = o; sink(rest); }\n  sink(rest);\n}\n",
        &[("2:rest", "3:rest", uncertain(3)), ("2:rest", "4:rest", EXACT)],
        ("dfg_label_nameonly_ownership_uncertain", 1),
        Some(Predicate::JavaScriptBindingPattern)
    ),
    case!(
        "js-shorthand-pattern",
        "shorthand_property_identifier_pattern",
        "function f(o) {\n  let x = source();\n  { const {x} = o; sink(x); }\n  sink(x);\n}\n",
        &[("2:x", "3:x", uncertain(3)), ("2:x", "4:x", EXACT)],
        ("dfg_label_nameonly_ownership_uncertain", 1),
        Some(Predicate::JavaScriptBindingPattern)
    ),
    case!(
        "js-pair-pattern-not-binding",
        "pair_pattern",
        "function f(o) {\n  let key = source();\n  { const {key: value} = o; sink(key); }\n  sink(key);\n}\n",
        &[("2:key", "3:key", EXACT), ("2:key", "4:key", EXACT)],
        ("dfg_label_exact", 2)
    ),
    case!(
        "js-regex-not-binding",
        "regex_pattern",
        "function f(s) {\n  let pattern = source();\n  /pattern/.test(s);\n  sink(pattern);\n}\n",
        &[("2:pattern", "4:pattern", EXACT)],
        ("dfg_label_exact", 1)
    ),
    case!(
        "js-class-heritage-not-binding",
        "class_heritage",
        "function f() {\n  let Base = source();\n  class Child extends Base {}\n  sink(Base);\n}\n",
        &[("2:Base", "4:Base", EXACT)],
        ("dfg_label_exact", 1)
    ),
    case!(
        "js-object-pattern-reuse",
        "object_pattern",
        "function f(o) {\n  let keep = source();\n  ({x} = o);\n  sink(keep);\n}\n",
        &[("2:keep", "4:keep", EXACT)],
        ("dfg_label_exact", 1)
    ),
    case!(
        "js-array-pattern-reuse",
        "array_pattern",
        "function f(o) {\n  let keep = source();\n  [x] = o;\n  sink(keep);\n}\n",
        &[("2:keep", "4:keep", EXACT)],
        ("dfg_label_exact", 1)
    ),
    case!(
        "js-rest-pattern-reuse",
        "rest_pattern",
        "function f(o) {\n  let keep = source();\n  [...x] = o;\n  sink(keep);\n}\n",
        &[("2:keep", "4:keep", EXACT)],
        ("dfg_label_exact", 1)
    ),
    case!(
        "js-shorthand-pattern-reuse",
        "shorthand_property_identifier_pattern",
        "function f(o) {\n  let keep = source();\n  ({x} = o);\n  sink(keep);\n}\n",
        &[("2:keep", "4:keep", EXACT)],
        ("dfg_label_exact", 1)
    ),
    case!(
        "js-assignment-pattern-catch-preserved",
        "assignment_pattern",
        "function f() {\n  let keep = source();\n  try { risky(); } catch ([x = clean()]) { consume(x); }\n  sink(keep);\n}\n",
        &[("2:keep", "4:keep", EXACT)],
        ("dfg_label_exact", 1)
    ),
    case!(
        "js-augmented-assignment-reuse",
        "augmented_assignment_expression",
        "function f() {\n  x = source();\n  x += clean();\n  sink(x);\n}\n",
        &[("2:x", "4:x", killed(3)), ("3:x", "4:x", EXACT)],
        ("dfg_label_nameonly_killed", 1)
    ),
    case!(
        "js-class-expression-name-uncertain",
        "class",
        "function f() {\n  C = source();\n  const holder = class C { method() { sink(C); } };\n  sink(C);\n}\n",
        &[
            (
                "2:C",
                "3:C",
                FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
            ),
            ("2:C", "4:C", EXACT),
        ],
        ("dfg_label_nameonly_cfg_incomplete", 1)
    ),
    case!(
        "js-generator-expression-name-uncertain",
        "generator_function",
        "function f() {\n  G = source();\n  const generator = function* G() { sink(G); };\n  sink(G);\n}\n",
        &[
            (
                "2:G",
                "3:G",
                FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
            ),
            ("2:G", "4:G", EXACT),
        ],
        ("dfg_label_nameonly_cfg_incomplete", 1)
    ),
    case!(
        "js-generator-declaration-name-uncertain",
        "generator_function_declaration",
        "function f() {\n  G = source();\n  function* G() { sink(G); }\n  sink(G);\n}\n",
        &[
            (
                "2:G",
                "3:G",
                FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
            ),
            ("2:G", "4:G", EXACT),
        ],
        ("dfg_label_nameonly_cfg_incomplete", 1)
    ),
    case!(
        "js-class-static-block-scope",
        "class_static_block",
        "function f() {\n  let x = source();\n  class C {\n    static {\n      let x = clean();\n      sink(x);\n    }\n  }\n  sink(x);\n}\n",
        &[
            (
                "2:x",
                "6:x",
                FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
            ),
            (
                "5:x",
                "6:x",
                FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
            ),
            ("2:x", "9:x", EXACT),
        ],
        ("dfg_label_nameonly_cfg_incomplete", 2)
    ),
    case!(
        "js-method-definition-scope",
        "method_definition",
        "function f() {\n  let x = source();\n  class C { method() { sink(x); } }\n  sink(x);\n}\n",
        &[
            (
                "2:x",
                "3:x",
                FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
            ),
            ("2:x", "4:x", EXACT),
        ],
        ("dfg_label_nameonly_cfg_incomplete", 1)
    ),
    case!(
        "js-switch-body-scope",
        "switch_body",
        "function f(v) {\n  let x = source();\n  switch (v) { case 1: let x = clean(); sink(x); break; default: break; }\n  sink(x);\n}\n",
        &[("2:x", "3:x", killed(3)), ("2:x", "4:x", EXACT)],
        ("dfg_label_nameonly_killed", 1)
    ),
    case!(
        "js-object-assignment-pattern-reuse",
        "object_assignment_pattern",
        "function f(o) {\n  let keep = source();\n  ({x = clean()} = o);\n  sink(keep);\n}\n",
        &[("2:keep", "4:keep", EXACT)],
        ("dfg_label_exact", 1)
    ),
    case!(
        "js-switch-case-not-binding",
        "switch_case",
        "function f(v) {\n  let keep = source();\n  switch (v) { case 1: consume(v); break; }\n  sink(keep);\n}\n",
        &[("2:keep", "4:keep", EXACT)],
        ("dfg_label_exact", 1)
    ),
    case!(
        "js-with-statement-not-binding",
        "with_statement",
        "function f(o) {\n  let keep = source();\n  with (o) { consume(o); }\n  sink(keep);\n}\n",
        &[("2:keep", "4:keep", EXACT)],
        ("dfg_label_exact", 1)
    ),
    case!(
        "js-dynamic-import-not-binding",
        "import",
        "function f() {\n  let keep = source();\n  import('pkg');\n  sink(keep);\n}\n",
        &[("2:keep", "4:keep", EXACT)],
        ("dfg_label_exact", 1)
    ),
    case!("js-import-attribute-not-binding", "import_attribute", IMPORT_SOURCE, &[("4:keep", "5:keep", EXACT)], ("dfg_label_exact", 1)),
    case!("js-import-clause-not-binding", "import_clause", IMPORT_SOURCE, &[("4:keep", "5:keep", EXACT)], ("dfg_label_exact", 1)),
    case!("js-import-specifier-not-binding", "import_specifier", IMPORT_SOURCE, &[("4:keep", "5:keep", EXACT)], ("dfg_label_exact", 1)),
    case!("js-import-statement-not-binding", "import_statement", IMPORT_SOURCE, &[("4:keep", "5:keep", EXACT)], ("dfg_label_exact", 1)),
    case!("js-named-imports-not-binding", "named_imports", IMPORT_SOURCE, &[("4:keep", "5:keep", EXACT)], ("dfg_label_exact", 1)),
    case!("js-namespace-import-not-binding", "namespace_import", IMPORT_SOURCE, &[("4:keep", "5:keep", EXACT)], ("dfg_label_exact", 1)),
];

#[test]
fn task16_red_destructure_mask() {
    let case = CASES
        .iter()
        .find(|case| case.id == "js-destructure-masks")
        .expect("Task 16 destructuring case");
    super::super::case::check_case(case);
}

#[test]
fn task16_javascript_cases() {
    for case in CASES {
        super::super::case::check_case(case);
    }
}

#[test]
fn javascript_destructure_conditional_bypass_preserves_outer_binding() {
    const EXPECT: &[(&str, &str, FlowConfidence)] =
        &[("2:x", "3:x", uncertain(3)), ("2:x", "4:x", EXACT)];
    let case = Case {
        id: "js-destructure-conditional-bypass",
        language: Language::JavaScript,
        kind: "object_pattern",
        variant: None,
        row_variant: Some(Predicate::JavaScriptBindingPattern),
        class: CaseClass::Behavioral,
        src: "function f(o) {\n  let x = source();\n  if (ready) { const {x} = o; sink(x); }\n  sink(x);\n}\n",
        expect: EXPECT,
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
    };
    super::super::case::check_case(&case);
}

#[test]
fn javascript_declaration_rows_own_only_their_introduced_identifiers() {
    use super::super::super::binding_table::{select_binding_row, DeclarationKind, Role};
    use super::super::super::transfer::row_introduced_identifiers;

    let parsed = super::super::super::parsed(
        "function declared() {\n  let owned = (() => { nested = 1; return nested; })();\n  var varOwned = 0;\n  let {destructured, renamed: localName, nested: [deep], ...rest} = input;\n  var [varFirst, ...varRest] = input;\n  for (let loopOwned = 0; loopOwned < 1; loopOwned++) { body = 1; }\n  class Owned { method() { member = 1; } }\n  try { risky(); } catch { optional = 1; }\n}\n",
        Language::JavaScript,
    );
    for (marker, expected) in [
        (
            "destructured",
            &["destructured", "localName", "deep", "rest"][..],
        ),
        ("varFirst", &["varFirst", "varRest"][..]),
    ] {
        let node = super::super::case::nodes_of_kind(&parsed, "variable_declarator")
            .into_iter()
            .find(|node| parsed.node_text(node).contains(marker))
            .unwrap_or_else(|| panic!("missing variable_declarator containing {marker}"));
        let row = select_binding_row(&parsed, node).expect("variable_declarator row");
        let actual: Vec<_> = row_introduced_identifiers(&parsed, node, row.fields)
            .into_iter()
            .map(|identifier| parsed.node_text(&identifier))
            .collect();
        assert_eq!(actual, expected);
    }

    for marker in ["owned =", "varOwned", "loopOwned ="] {
        let node = super::super::case::nodes_of_kind(&parsed, "variable_declarator")
            .into_iter()
            .find(|node| parsed.node_text(node).contains(marker))
            .unwrap_or_else(|| panic!("missing variable_declarator containing {marker}"));
        let row = select_binding_row(&parsed, node).expect("ordinary variable_declarator row");
        assert!(row.roles.has(Role::NotBinding));
        assert!(row_introduced_identifiers(&parsed, node, row.fields).is_empty());
    }

    for kind in ["function_declaration", "for_statement", "method_definition"] {
        let node = super::super::case::nodes_of_kind(&parsed, kind)[0];
        let row = select_binding_row(&parsed, node).expect("scope row");
        assert_eq!(
            row.declaration, None,
            "{kind} must not claim body assignments"
        );
    }

    let optional_catch = super::super::case::nodes_of_kind(&parsed, "catch_clause")[0];
    let catch_row = select_binding_row(&parsed, optional_catch).expect("catch row");
    assert_eq!(catch_row.declaration, Some(DeclarationKind::Other));
    assert!(row_introduced_identifiers(&parsed, optional_catch, catch_row.fields).is_empty());

    for assignment in super::super::case::nodes_of_kind(&parsed, "assignment_expression") {
        let row = select_binding_row(&parsed, assignment).expect("assignment row");
        assert_eq!(row.declaration, Some(DeclarationKind::Reuse));
        assert_eq!(row.fields, &["left"]);
    }
}
