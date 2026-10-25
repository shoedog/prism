use super::super::super::binding_table::{
    rows, DeclarationKind, Predicate, Role, Ruling, Visibility,
};
use crate::cpg::{FlowConfidence, FlowDoubt};
use crate::languages::Language;

pub(super) const SOURCE: &str = r#"import defaultName, * as ns from "pkg" with { type: "json" };
import {item as alias} from "pkg";
var outer = source();
function declared(a, ...rest) {
  let [first, ...tail] = rest;
  const {key: renamed = first, shorthand, ...others} = ns;
  outer += 1;
  for (let i = 0; i < 1; i++) { outer = i; }
  for (const item of rest) { outer = item; }
  try { throw outer; } catch ({message}) { outer = message; }
  switch (outer) { case 1: break; default: break; }
  with (ns) { outer = value; }
  return /pattern/.test(String(outer));
}
function* generated() { yield outer; }
const generatorExpression = function* () { yield outer; };
const expression = function named() { return outer; };
const arrow = (x = outer) => x;
({assigned = outer, ...assignedRest} = ns);
class Child extends Base { static { this.ready = true; } method(p) { return p; } }
"#;

pub(super) const CLASSIFIED_MASK_SOURCE: &str = r#"function f() {
  let x = source();
  { let x = clean(); sink(x); }
  sink(x);
}
"#;

pub(super) const CASES: &[super::case::Case] = &[
    super::case::Case {
        id: "e0a-js-class_declaration",
        language: Language::JavaScript,
        kind: "class_declaration",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "function f() {\n  C = source();\n  { class C {} sink(C); }\n  sink(C);\n}",
        expect: &[
            (
                "2:C",
                "3:C",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
            ),
            ("2:C", "4:C", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "e0a-js-lexical_declaration",
        language: Language::JavaScript,
        kind: "lexical_declaration",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: CLASSIFIED_MASK_SOURCE,
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
        id: "e0a-x-for_in_statement-predicate",
        language: Language::JavaScript,
        kind: "for_in_statement",
        variant: Some("binding"),
        row_variant: Some(Predicate::FieldTextIs {
            field: "kind",
            any_of: &["let", "const"],
        }),
        class: super::case::CaseClass::Behavioral,
        src: "function f(items) {\n  let item = source();\n  for (const item in items) { sink(item); }\n  sink(item);\n}",
        expect: &[
            (
                "2:item",
                "3:item",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
            ),
            ("2:item", "4:item", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "e0a-x-for_in_statement-residual",
        language: Language::JavaScript,
        kind: "for_in_statement",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "function f(items) {\n  let outer = source();\n  for (item in items) { sink(outer); }\n  sink(outer);\n}",
        expect: &[
            ("2:outer", "3:outer", FlowConfidence::Exact),
            ("2:outer", "4:outer", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_exact", 2)),
    },
    super::case::Case {
        id: "js-catch-param-masks-declared",
        language: Language::JavaScript,
        kind: "catch_clause",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "function f() {\n  let x = source();\n  try { risky(); } catch (x) { sink(x); }\n  sink(x);\n}\n",
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
        id: "js-catch-param-masks-undeclared",
        language: Language::JavaScript,
        kind: "catch_clause",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "function f() {\n  x = source();\n  try { risky(); } catch (x) { sink(x); }\n  sink(x);\n}\n",
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
        id: "js-catch-after-try-preserved",
        language: Language::JavaScript,
        kind: "catch_clause",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "function f() {\n  let x = source();\n  try { risky(); } catch (x) { consume(x); }\n  sink(x);\n}\n",
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
        id: "js-catch-param-assignment-keeps-binding",
        language: Language::JavaScript,
        kind: "catch_clause",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "function f() {\n  x = source();\n  try { risky(); } catch (x) {\n    x = source();\n    sink(x);\n  }\n  sink(x);\n}\n",
        expect: &[
            (
                "2:x",
                "5:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
            ),
            ("4:x", "5:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "js-catch-outer-name-assignment-preserved",
        language: Language::JavaScript,
        kind: "catch_clause",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "function f() {\n  x = source();\n  try { risky(); } catch (x) {\n    y = source();\n    sink(x);\n  }\n  sink(x); sink(y);\n}\n",
        expect: &[
            (
                "2:x",
                "5:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
            ),
            ("2:x", "7:x", FlowConfidence::Exact),
            ("4:y", "7:y", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "js-catch-object-rename-masks-value-only",
        language: Language::JavaScript,
        kind: "catch_clause",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "function f() {\n  x = source();\n  y = source();\n  try { risky(); } catch ({x: y}) { sink(x); sink(y); }\n}\n",
        expect: &[
            ("2:x", "4:x", FlowConfidence::Exact),
            (
                "3:y",
                "4:y",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 }),
            ),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "js-catch-array-default-masks-left-only",
        language: Language::JavaScript,
        kind: "catch_clause",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "function f() {\n  x = source();\n  y = source();\n  try { risky(); } catch ([y = x]) { sink(x); sink(y); }\n}\n",
        expect: &[
            ("2:x", "4:x", FlowConfidence::Exact),
            (
                "3:y",
                "4:y",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 }),
            ),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "js-catch-object-shorthand-masks-name",
        language: Language::JavaScript,
        kind: "catch_clause",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "function f() {\n  x = source();\n  try { risky(); } catch ({x}) { sink(x); }\n  sink(x);\n}\n",
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
        id: "js-catch-underscore-masks-name",
        language: Language::JavaScript,
        kind: "catch_clause",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "function f() {\n  _ = source();\n  try { risky(); } catch (_) { sink(_); }\n  sink(_);\n}\n",
        expect: &[
            (
                "2:_",
                "3:_",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
            ),
            ("2:_", "4:_", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "js-catch-multiline-param-kill-line-is-construct",
        language: Language::JavaScript,
        kind: "catch_clause",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "function f() {\n  x = source();\n  try { risky(); } catch (\n    x\n  ) { sink(x); }\n  sink(x);\n}\n",
        expect: &[
            (
                "2:x",
                "5:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
            ),
            ("2:x", "6:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "js-catch-var-redecl-writes-simple-param",
        language: Language::JavaScript,
        kind: "catch_clause",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "function f() {\n  x = source();\n  try { risky(); } catch (x) {\n    var x = source();\n    sink(x);\n  }\n  sink(x);\n}\n",
        expect: &[
            (
                "2:x",
                "5:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
            ),
            ("4:x", "5:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "js-catch-nested-var-redecl-writes-simple-param",
        language: Language::JavaScript,
        kind: "catch_clause",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "function f() {\n  x = source();\n  try { risky(); } catch (x) {\n    {\n      var x = source();\n      sink(x);\n    }\n  }\n  sink(x);\n}\n",
        expect: &[
            (
                "2:x",
                "6:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
            ),
            ("5:x", "6:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "js-catch-for-var-redecl-writes-simple-param",
        language: Language::JavaScript,
        kind: "catch_clause",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "function f() {\n  x = source();\n  try { risky(); } catch (x) {\n    for (var x = source(); ready; step()) {\n      sink(x);\n    }\n  }\n  sink(x);\n}\n",
        expect: &[
            (
                "2:x",
                "5:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
            ),
            ("4:x", "5:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "js-catch-different-var-keeps-function-binding",
        language: Language::JavaScript,
        kind: "catch_clause",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "function f() {\n  x = source();\n  try { risky(); } catch (x) { var y = source(); sink(x); }\n  sink(x); sink(y);\n}\n",
        expect: &[
            (
                "2:x",
                "3:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
            ),
            ("2:x", "4:x", FlowConfidence::Exact),
            ("3:y", "4:y", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "fix2-s6-js-optional-catch-assignment",
        language: Language::JavaScript,
        kind: "catch_clause",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "function f() {\n  let x = source(), y = source();\n  { let y = clean(); sink(y); } try {\n    g();\n  } catch {\n    x = clean();\n    sink(x);\n  }\n  sink(x);\n}\n",
        expect: &[
            ("6:x", "7:x", FlowConfidence::Exact),
            (
                "2:y",
                "3:y",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
            ),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "fix2-s6-js-optional-catch-destructure-assignment",
        language: Language::JavaScript,
        kind: "catch_clause",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "function f(arr) {\n  let x = source(), y = source();\n  { let y = clean(); sink(y); } try {\n    h();\n  } catch {\n    [x] = arr;\n    sink(x);\n  }\n  sink(x);\n}\n",
        expect: &[
            ("6:x", "7:x", FlowConfidence::Exact),
            (
                "2:y",
                "3:y",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
            ),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
];

#[test]
fn fix2_s6_optional_catch_cases() {
    for case in CASES.iter().filter(|case| case.id.starts_with("fix2-s6-")) {
        super::case::check_case(case);
    }
}
pub(super) const CURATED: &[(&str, &str)] = &[
    ("statement_block", "e0a-js-statement_block"),
    ("class_body", "e0a-js-class_body"),
    ("function_declaration", "e0a-js-function_declaration"),
    ("function_expression", "e0a-js-function_expression"),
    ("arrow_function", "e0a-js-arrow_function"),
    ("for_statement", "e0a-js-for_statement"),
    ("for_in_statement", "e0a-js-for_in_statement"),
    ("variable_declaration", "e0a-js-variable_declaration"),
    ("lexical_declaration", "e0a-js-lexical_declaration"),
    ("class_declaration", "e0a-js-class_declaration"),
    ("formal_parameters", "e0a-x-js-formal_parameters"),
];

pub(super) fn source_for_kind(kind: &str) -> &'static str {
    match kind {
        "class" => "const C = class Named { method() {} };",
        "import" => "async function f() { return import('pkg'); }",
        _ => SOURCE,
    }
}

#[test]
fn javascript_rows_reproduce_the_old_match_arms() {
    let javascript_rows = rows(Language::JavaScript);
    let curated_len = CURATED.len() + 1;
    for language in [Language::TypeScript, Language::Tsx] {
        assert_eq!(
            &javascript_rows[..curated_len],
            &rows(language)[..curated_len],
            "{language:?}: shared curated prefix"
        );
    }
    let catch = &javascript_rows[curated_len];
    assert_eq!(catch.kind, "catch_clause");
    assert_eq!(catch.declaration, Some(DeclarationKind::Other));
    assert!(catch.roles.has(Role::Binding));
    assert!(catch.roles.has(Role::Scope));
    assert_eq!(catch.fields, &["parameter"]);
    assert_eq!(catch.visibility, Visibility::WholeScope);
    assert!(javascript_rows[curated_len + 1..]
        .iter()
        .all(|row| matches!(
            row.ruling,
            Ruling::Uncertain {
                reason: "not yet curated",
                ..
            }
        )));

    for k in [
        "statement_block",
        "class_body",
        "function_declaration",
        "function_expression",
        "arrow_function",
        "for_statement",
    ] {
        assert!(
            javascript_rows
                .iter()
                .any(|r| r.kind == k && r.roles.has(Role::Scope)),
            "missing scope row {k}"
        );
    }

    for (k, declaration) in [
        ("variable_declaration", DeclarationKind::JavaScriptVar),
        ("lexical_declaration", DeclarationKind::Other),
        ("class_declaration", DeclarationKind::Other),
    ] {
        let r = javascript_rows
            .iter()
            .find(|r| r.kind == k)
            .unwrap_or_else(|| panic!("missing {k}"));
        assert_eq!(r.declaration, Some(declaration));
        assert!(r.roles.has(Role::Binding));
    }

    let for_in_rows: Vec<_> = javascript_rows
        .iter()
        .filter(|r| r.kind == "for_in_statement")
        .collect();
    assert_eq!(for_in_rows.len(), 2);
    let header = for_in_rows
        .iter()
        .find(|r| r.variant.is_some())
        .expect("missing for_in_statement predicate row");
    assert_eq!(header.declaration, Some(DeclarationKind::Other));
    assert_eq!(header.visibility, Visibility::Header);
    assert!(header.roles.has(Role::Scope));
    assert!(header.roles.has(Role::Binding));
    assert!(matches!(
        header.variant,
        Some(Predicate::FieldTextIs {
            field: "kind",
            any_of: &["let", "const"]
        })
    ));
    let residual = for_in_rows
        .iter()
        .find(|r| r.variant.is_none())
        .expect("missing for_in_statement residual row");
    assert_eq!(residual.declaration, None);
    assert!(residual.roles.has(Role::Scope));
    assert!(!residual.roles.has(Role::Binding));
    assert!(residual.fields.is_empty());
    assert_eq!(residual.visibility, Visibility::WholeScope);
}
