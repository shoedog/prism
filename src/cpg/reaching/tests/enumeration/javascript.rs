use super::super::super::binding_table::{
    rows, DeclarationKind, Predicate, Role, Ruling, Visibility,
};
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

pub(super) const CASES: &[super::case::Case] = &[];
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
    assert!(javascript_rows[curated_len..].iter().all(|row| matches!(
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
